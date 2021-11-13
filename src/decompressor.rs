use std::{mem, fs, thread, cmp, ops, io::{Read, BufReader}};
use std::collections::{BinaryHeap, BTreeMap};
use std::sync::{Arc, atomic::AtomicUsize};
use chrono::{DateTime, Utc};
use crossbeam_channel::Receiver;
use lzzzz::lz4f::BufReadDecompressor;

pub struct Decompressor {
    pub directory: String,
    pub remove_files_after_decompression: bool,
}

struct Worker {
    pub thread: thread::JoinHandle<()>,
    pub receiver: Receiver<crate::StreamFrame>,
}

impl Decompressor {
    pub fn new(directory: &str, remove_files_after_decompression: bool) -> Self {
        Self { directory: directory.to_string(), remove_files_after_decompression }
    }

    pub fn decompress_from_disk(&self, mut process_function: Box<dyn FnMut(crate::StreamFrame)>) {
        let mut ordered_timestamps = scan_directory_for_timestamps(&self.directory);

        for (_timestamp, filenames) in ordered_timestamps.iter_mut() {
            filenames.sort();

            let workers = filenames.iter().map(|filename| {
                spawn_worker(&self.directory, &filename)
            }).collect();

            order_frames_from_worker_threads(workers, &mut process_function);
        }

        // Wait until the very end before removing files in case a panic happens mid-way through.
        if self.remove_files_after_decompression {
            for (_timestamp, filenames) in ordered_timestamps {
                for filename in filenames {
                    let _ = fs::remove_file(format!("{}/{}", self.directory, filename));
                }
            }
        }
    }
}

fn scan_directory_for_timestamps(directory: &str) -> BTreeMap<DateTime<Utc>, Vec<String>> {
    let mut map = BTreeMap::new();

    for result in fs::read_dir(directory).unwrap() {
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

fn order_frames_from_worker_threads(mut workers: Vec<Worker>, process_function: &mut Box<dyn FnMut(crate::StreamFrame)>) {
    let mut min_heap = BinaryHeap::new();
    let mut expected_frame = 0;

    loop {
        // Ask each worker for their next stream frame. Remove workers that have finished.
        let drained = workers.drain_filter(|worker| {
            if let Ok(stream_frame) = worker.receiver.recv() {
                min_heap.push(cmp::Reverse(OrderableFrame(stream_frame)));
                false
            } else {
                true
            }
        });

        // Panic if a worker didn't terminate properly.
        for worker in drained { worker.thread.join().unwrap(); }

        let mut found_a_frame = false;

        // Keep getting the next ordered frame from the heap until there's a gap.
        // If the heap is empty, all workers must have finished so return.
        loop {
            let min_frame = match min_heap.pop() { Some(r) => r, _ => return };

            if min_frame.0.frame_number == expected_frame {
                process_function(min_frame.0.0);
                expected_frame += 1;
                found_a_frame = true;
            } else {
                min_heap.push(min_frame); // Put the frame back.
                break;
            }
        }

        if found_a_frame { continue; }

        // If we didn't find a frame then some compressed data is missing.
        // This isn't the same as frames being dropped from the capture.
        process_function(crate::StreamFrame {
            status: crate::FrameStatus::Missing,
            image_data: None,
            frame_number: expected_frame,
            buffer_size_in_bytes: Arc::new(AtomicUsize::new(0)),
            ..Default::default()
        });

        expected_frame += 1;
    }
}

fn spawn_worker(directory: &str, filename: &str) -> Worker {
    let (sender, receiver) = crossbeam_channel::unbounded(); // TODO: bounded

    let decode_config = decoding_config();

    let file = fs::File::open(format!("{}/{}", directory, filename)).unwrap();
    let mut reader = BufReadDecompressor::new(BufReader::new(file)).unwrap();

    let mut lengths_buffer = [0; U64_LEN + U64_LEN];
    let mut remainder_buffer = vec![];

    let thread = thread::spawn(move || {
        // Read decompressed bytes from the file. Decode each packet to a
        // StreamFrame and send it to the channel. The packets have this layout:
        //
        // [ packet_len | stream_frame_len | stream_frame | image_data ]
        //
        // If the reader ends cleanly at the end of a packet then return.
        // Otherwise, send a StreamFrame to the channel with FrameStatus::Corrupt.

        loop {
            match reader.read_exact(&mut lengths_buffer) { Ok(_) => {}, _ => return }

            let packet_len_bytes = &lengths_buffer[0..U64_LEN];
            let packet_len_result = bincode::decode_from_slice(packet_len_bytes, decode_config);
            let packet_len: u64 = match packet_len_result { Ok(p) => p, _ => break };

            let stream_frame_len_bytes = &lengths_buffer[U64_LEN..];
            let stream_frame_len_result = bincode::decode_from_slice(stream_frame_len_bytes, decode_config);
            let stream_frame_len: u64 = match stream_frame_len_result { Ok(p) => p, _ => break };

            let remainder_len = packet_len as usize - U64_LEN - U64_LEN;
            remainder_buffer.resize(remainder_len, 0);
            match reader.read_exact(&mut remainder_buffer) { Ok(_) => {}, _ => break }

            let stream_frame_bytes = &remainder_buffer[0..stream_frame_len as usize];
            let stream_frame_result = bincode::decode_from_slice(stream_frame_bytes, decode_config);
            let mut stream_frame: crate::StreamFrame = match stream_frame_result { Ok(f) => f, _ => break };

            let image_data_bytes = &remainder_buffer[stream_frame_len as usize..];
            stream_frame.image_data = Some(crate::ImageData::Bytes(image_data_bytes.to_vec()));

            sender.send(stream_frame).unwrap();
        }

        // TODO: corrupt frame
    });

    Worker { thread, receiver }
}

const U64_LEN: usize = mem::size_of::<u64>();

fn decoding_config() -> bincode::config::Configuration {
    bincode::config::Configuration::standard()
}

struct OrderableFrame(crate::StreamFrame);

impl ops::Deref for OrderableFrame {
    type Target = crate::StreamFrame;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl cmp::Ord for OrderableFrame {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.frame_number.cmp(&other.frame_number)
    }
}

impl cmp::PartialOrd for OrderableFrame {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.frame_number.partial_cmp(&other.frame_number)
    }
}

impl cmp::PartialEq for OrderableFrame {
    fn eq(&self, other: &Self) -> bool {
        self.frame_number.eq(&other.frame_number)
    }
}

impl cmp::Eq for OrderableFrame {}
