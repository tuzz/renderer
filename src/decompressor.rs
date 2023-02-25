use std::{mem, fs, path::Path, thread, cmp, ops, io::{Read, BufReader}};
use std::collections::{BinaryHeap, BTreeMap};
use std::sync::{Arc, atomic::AtomicUsize};
use chrono::{DateTime, Utc};
use crossbeam_channel::Receiver;
use lzzzz::lz4f::BufReadDecompressor;

pub struct Decompressor {
    pub directory: String,
    pub remove_files_after_decompression: bool,
}

struct Worker<T> {
    pub thread: thread::JoinHandle<()>,
    pub receiver: Receiver<(crate::VideoFrame, T)>,
}

pub type PerThreadFunction<T> = Arc<dyn Fn(&crate::VideoFrame, DateTime<Utc>) -> T + Send + Sync>;
pub type InOrderFunction<T> = Box<dyn FnMut(crate::VideoFrame, Result<T, &'static str>, &DateTime<Utc>)>;

impl Decompressor {
    pub fn new(directory: &str, remove_files_after_decompression: bool) -> Self {
        Self { directory: directory.to_string(), remove_files_after_decompression }
    }

    pub fn can_run(directory: &str) -> bool {
        !scan_directory_for_timestamps(directory).is_empty()
    }

    pub fn decompress_from_disk<T: Send + 'static>(&self, per_thread_function: PerThreadFunction<T>, mut in_order_function: InOrderFunction<T>) {
        let mut ordered_timestamps = scan_directory_for_timestamps(&self.directory);

        for (timestamp, filenames) in ordered_timestamps.iter_mut() {
            filenames.sort();

            let workers = filenames.iter().map(|filename| {
                spawn_worker(&self.directory, &filename, &per_thread_function, timestamp)
            }).collect();

            order_frames_from_worker_threads(workers, &mut in_order_function, timestamp);
        }

        // Wait until the very end before removing files in case a panic happens mid-way through.
        if self.remove_files_after_decompression {
            for (_timestamp, filenames) in ordered_timestamps {
                for filename in filenames {
                    let _ = fs::remove_file(path(&self.directory, &filename));
                }
            }
        }
    }
}

fn path(directory: &str, filename: &str) -> String {
    Path::new(directory).join(filename).into_os_string().into_string().unwrap()
}

fn scan_directory_for_timestamps(directory: &str) -> BTreeMap<DateTime<Utc>, Vec<String>> {
    let mut map = BTreeMap::new();

    let listing = match fs::read_dir(directory) { Ok(d) => d, _ => return map };

    for result in listing {
        let dir_entry = match result { Ok(d) => d, _ => continue };

        let result = dir_entry.metadata();
        let metadata = match result { Ok(m) => m, _ => continue };

        if metadata.len() == 0 { continue; }

        let result = dir_entry.file_name().into_string();
        let filename = match result { Ok(f) => f, _ => continue };

        let result = recover_timestamp_from_filename(&filename);
        let timestamp = match result { Ok(t) => t, _ => continue };

        let filenames = map.entry(timestamp).or_insert_with(|| vec![]);
        filenames.push(filename);
    }

    map
}

fn recover_timestamp_from_filename(filename: &str) -> Result<DateTime<Utc>, ()> {
    if !filename.ends_with(".sz") { return Err(()); }

    let option = filename.split("--").next();
    let prefix = match option { Some(s) => s, _ => return Err(()) };

    let result = chrono::DateTime::parse_from_rfc3339(&prefix.replace("_", ":"));
    let timestamp = match result { Ok(t) => t, _ => return Err(()) };

    Ok(timestamp.into())
}

fn order_frames_from_worker_threads<T>(mut workers: Vec<Worker<T>>, in_order_function: &mut InOrderFunction<T>, timestamp: &DateTime<Utc>) {
    let mut min_heap = BinaryHeap::new();
    let mut expected_frame = 1;

    loop {
        // Ask each worker for their next stream frame. If the stream frame doesn't
        // have image data then ask again until one that does have image data is received.
        //
        // We need to do this to avoid decompression filling up memory when there's a run
        // of dropped frames when recording video. What tends to happen is one of the
        // compression threads picks up almost all of the dropped frames and writes it to
        // its compressed file while the others are busy. If we always received frames in
        // round robin from the decompression threads then we'd fill up memory from the
        // threads that didn't process the dropped frames and are further ahead.
        //
        // Therefore, keep consuming until a frame with real image data is received so
        // that we mimic the thread balancing pattern from the compression side.
        let drained = workers.drain_filter(|worker| {
            loop {
                if let Ok((video_frame, t)) = worker.receiver.recv() {
                    let has_image_data = video_frame.image_data.is_some();

                    min_heap.push(cmp::Reverse(OrderableFrame((video_frame, t))));

                    if has_image_data { return false; }
                } else {
                    return true; // The worker has run out of frames so remove it.
                }
            }
        });

        // Panic in the main thread if a worker thread didn't terminate properly.
        for worker in drained { worker.thread.join().unwrap(); }

        let mut advanced_by_at_least_one_frame = false;

        // Keep getting the next ordered frame from the heap and process it.
        // If there's a gap in frame_number then break so we can request more frames.
        // If there are no more frames to request (workers.is_empty) then we're done.
        loop {
            let min_frame = match min_heap.pop() {
                Some(cmp_reverse_wrapper) => cmp_reverse_wrapper,
                _ => if workers.is_empty() { return } else { break },
            };

            if min_frame.0.frame_number == expected_frame {
                let (video_frame, t) = min_frame.0.0;
                in_order_function(video_frame, Ok(t), timestamp);

                expected_frame += 1;
                advanced_by_at_least_one_frame = true;
            } else {
                min_heap.push(min_frame); // Put the frame back.
                break;
            }
        }

        if advanced_by_at_least_one_frame { continue; }

        // If we didn't advance by at least one frame in each iteration of the loop then
        // we must be missing some compressed data, e.g. maybe a .sz file was deleted.
        //
        // This isn't the same as a frame being dropped during recording as those still
        // appear in the compressed data as VideoFrames with status=Dropped.
        //
        // If we are missing data then yield VideoFrames with a status of Missing so
        // that the calling code can decide what to do.

        let next_available_frame = min_heap.peek().unwrap().0.frame_number;

        loop {
            in_order_function(
                crate::VideoFrame {
                    status: crate::FrameStatus::Missing,
                    image_data: None,
                    frame_number: expected_frame,
                    buffer_size_in_bytes: Arc::new(AtomicUsize::new(0)),
                    ..Default::default()
                },
                Err("The frame was missing from the compressed files."),
                timestamp,
            );

            expected_frame += 1;

            if expected_frame == next_available_frame { break; }
        }
    }
}

fn spawn_worker<T: Send + 'static>(directory: &str, filename: &str, per_thread_function: &PerThreadFunction<T>, timestamp: &DateTime<Utc>) -> Worker<T> {
    // Usually the slow part of the code will be the actual processing rather
    // than decompressing and decoding stream frames. Therefore, bound the
    // channel size to 0 to keep memory usage down. This forces worker threads
    // to wait for the main thread to be ready before decoding their next frame.
    let (sender, receiver) = crossbeam_channel::bounded(0);

    let per_thread_function = Arc::clone(per_thread_function);
    let timestamp = timestamp.clone();
    let decode_config = decoding_config();

    let file = fs::File::open(path(directory, filename)).unwrap();
    let mut reader = BufReadDecompressor::new(BufReader::new(file)).unwrap();

    let mut packet_len_bytes = [0; U64_LEN];
    let mut video_frame_len_bytes = [0; U64_LEN];
    let mut video_frame_bytes = vec![];

    let thread = thread::spawn(move || {
        // Read decompressed bytes from the file. Decode each packet to a
        // VideoFrame and send it to the channel. The packets have this layout:
        //
        // [ packet_len | video_frame_len | video_frame | image_data ]
        //     (u64)           (u64)          (bincode)        (raw)
        //
        // If the reader ends cleanly at the end of a packet then return.
        // Otherwise, send a VideoFrame to the channel with FrameStatus::Corrupt.

        loop {
            // Read and decode packet_len.
            match reader.read_exact(&mut packet_len_bytes) { Ok(_) => {}, _ => return }
            let packet_len = u64::from_be_bytes(packet_len_bytes) as usize;

            // Read and decode video_frame_len.
            match reader.read_exact(&mut video_frame_len_bytes) { Ok(_) => {}, _ => break }
            let video_frame_len = u64::from_be_bytes(video_frame_len_bytes) as usize;

            // Read video_frame.
            video_frame_bytes.resize(video_frame_len, 0);
            match reader.read_exact(&mut video_frame_bytes) { Ok(_) => {}, _ => break }

            // Decode video_frame.
            let result = bincode::decode_from_slice(&video_frame_bytes, decode_config);
            let mut video_frame: crate::VideoFrame = match result { Ok((f, _)) => f, _ => break }; // TODO: advance to next packet instead of breaking

            if video_frame.image_data.is_some() {
                // Read image_data.
                let remainder_len = packet_len - U64_LEN - U64_LEN - video_frame_len;
                let mut image_data_bytes = vec![0; remainder_len];
                match reader.read_exact(&mut image_data_bytes) { Ok(_) => {}, _ => break } // TODO: advance to next packet instead of breaking

                // Decode image_data.
                video_frame.image_data = Some(crate::ImageData::Bytes(image_data_bytes));
            }

            let t = per_thread_function(&video_frame, timestamp);
            sender.send((video_frame, t)).unwrap();
        }

        // TODO: corrupt frame
    });

    Worker { thread, receiver }
}

const U64_LEN: usize = mem::size_of::<u64>();

fn decoding_config() -> bincode::config::Configuration {
    bincode::config::standard()
}

struct OrderableFrame<T>((crate::VideoFrame, T));

impl<T> ops::Deref for OrderableFrame<T> {
    type Target = crate::VideoFrame;

    fn deref(&self) -> &Self::Target {
        &self.0.0
    }
}

impl<T> cmp::Ord for OrderableFrame<T> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.frame_number.cmp(&other.frame_number)
    }
}

impl<T> cmp::PartialOrd for OrderableFrame<T> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.frame_number.partial_cmp(&other.frame_number)
    }
}

impl<T> cmp::PartialEq for OrderableFrame<T> {
    fn eq(&self, other: &Self) -> bool {
        self.frame_number.eq(&other.frame_number)
    }
}

impl<T> cmp::Eq for OrderableFrame<T> {}
