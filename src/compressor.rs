use std::{mem, thread, time, io::{Write, BufWriter}, cell::RefCell, sync::atomic::Ordering};
use std::{path::Path, fs};
use chrono::{DateTime, SecondsFormat, Utc};
use crossbeam_channel::{Sender, Receiver};
use lzzzz::lz4f;

pub struct Compressor {
    pub timestamp: String,
    pub threads: Vec<thread::JoinHandle<()>>,
    pub sender: Option<Sender<crate::VideoFrame>>,
    pub stats: Option<RefCell<Stats>>,
}

impl Compressor {
    pub fn new(directory: &str, max_frames_queued: Option<usize>, lz4_compression_level: u8, print_stats: bool) -> Self {
        let is_valid_level = lz4_compression_level as i32 <= lz4f::CLEVEL_MAX;
        assert!(is_valid_level, "Please choose a compression level in the range 0..={}", lz4f::CLEVEL_MAX);

        fs::create_dir_all(directory).unwrap();

        let timestamp = generate_timestamp();
        let (sender, receiver) = create_channel(max_frames_queued);

        let threads = (0..num_cpus::get()).map(|i| {
            spawn_thread(&receiver, &directory, &timestamp, i, lz4_compression_level)
        }).collect();

        let mut stats = None;
        if print_stats {
            stats = Some(RefCell::new(Stats::new(directory, lz4_compression_level, max_frames_queued)));
        }

        Compressor { timestamp, threads, sender: Some(sender), stats }
    }

    pub fn compress_to_disk(&self, video_frame: crate::VideoFrame) {
        let sender = self.sender.as_ref().unwrap();

        if let Some(stats) = self.stats.as_ref() {
            stats.borrow_mut().update(&video_frame, &self.timestamp, self.threads.len(), sender.len());
        }

        sender.send(video_frame).unwrap();
    }

    pub fn finish(&mut self) {
        if self.sender.is_none() { return; }

        // Wait for the worker threads to exhaust the channel.
        while self.sender.as_ref().unwrap().len() > 0 {
            thread::sleep(time::Duration::from_millis(10));
        }

        // Disconnect the channel so that the worker threads break.
        let sender = self.sender.take().unwrap();
        drop(sender);

        // Wait for the worker threads to exit.
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}

impl Drop for Compressor {
    fn drop(&mut self) {
        self.finish();
    }
}

fn generate_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true).replace(":", "_")
}

// If max_frames_queued is set, create a bounded queue that blocks the main
// thread and slows the renderer down if the compression threads can't keep up.
//
// Otherwise, create an unbounded queue that won't block the main thread but
// will cause frames to be dropped if max_buffer_size_in_bytes is exceeded.
fn create_channel(max_frames_queued: Option<usize>) -> (Sender<crate::VideoFrame>, Receiver<crate::VideoFrame>){
    if let Some(channel_capacity) = max_frames_queued {
        crossbeam_channel::bounded::<crate::VideoFrame>(channel_capacity)
    } else {
        crossbeam_channel::unbounded::<crate::VideoFrame>()
    }
}

fn spawn_thread(receiver: &Receiver<crate::VideoFrame>, directory: &str, timestamp: &str, i: usize, lz4_compression_level: u8) -> thread::JoinHandle<()> {
    let receiver = receiver.clone();

    let compress_config = compression_config(lz4_compression_level);
    let encode_config = encoding_config();

    let filename = format!("{}--{}.sz", timestamp, i);
    let path = Path::new(directory).join(filename).into_os_string().into_string().unwrap();

    let file = fs::File::create(path).unwrap();
    let mut writer = lz4f::WriteCompressor::new(BufWriter::new(file), compress_config).unwrap();

    thread::spawn(move || {
        // When a video_frame is received from the channel, write it to the
        // compressor in packets of bytes that have this layout:
        //
        // [ packet_len | video_frame_len | video_frame | image_data ]
        //     (u64)           (u64)          (bincode)        (raw)

        loop {
            let mut packet_len: u64 = (U64_LEN + U64_LEN) as u64;

            let video_frame = match receiver.recv() { Ok(f) => f, _ => break };
            let video_frame_bytes = bincode::encode_to_vec(&video_frame, encode_config).unwrap();
            let video_frame_len = video_frame_bytes.len() as u64;
            packet_len += video_frame_len;

            let mut write_image_data_bytes = None;

            if let Some(image_data) = &video_frame.image_data {
                let image_data_bytes = image_data.buffer().slice(..).get_mapped_range();
                packet_len += image_data_bytes.len() as u64;

                write_image_data_bytes = Some(move |w: &mut Writer| {
                    w.write_all(&image_data_bytes).unwrap()
                });
            }

            writer.write_all(&packet_len.to_be_bytes()).unwrap();
            writer.write_all(&video_frame_len.to_be_bytes()).unwrap();
            writer.write_all(&video_frame_bytes).unwrap();
            write_image_data_bytes.map(|closure| closure(&mut writer));
        }
    })
}

const U64_LEN: usize = mem::size_of::<u64>();

type Writer = lz4f::WriteCompressor::<BufWriter<fs::File>>;

fn compression_config(lz4_compression_level: u8) -> lz4f::Preferences {
    lz4f::PreferencesBuilder::new()
        .compression_level(lz4_compression_level as i32)
        .favor_dec_speed(lz4f::FavorDecSpeed::Disabled)
        .auto_flush(lz4f::AutoFlush::Enabled)
        .build()
}

fn encoding_config() -> bincode::config::Configuration {
    bincode::config::Configuration::standard()
}

#[derive(Default)]
pub struct Stats {
    pub directory: String,
    pub lz4_compression_level: u8,
    pub max_frames_queued: Option<usize>,
    pub started_at: Option<DateTime<Utc>>,
    pub frames_captured: usize,
    pub frames_dropped: usize,
    pub has_resized: bool,
    pub prev_width: usize,
    pub prev_height: usize,
    pub raw_video_size: usize,
}

impl Stats {
    fn new(directory: &str, lz4_compression_level: u8, max_frames_queued: Option<usize>) -> Self {
        Self {
            directory: directory.to_string(),
            lz4_compression_level,
            max_frames_queued,
            ..Self::default()
        }
    }

    fn update(&mut self, video_frame: &crate::VideoFrame, filename_timestamp: &str, num_threads: usize, queue_size: usize) {
        match &video_frame.status {
            crate::FrameStatus::Captured => {
                self.frames_captured += 1;
                self.raw_video_size += video_frame.frame_size_in_bytes;

                if self.frames_captured > 1 {
                    self.has_resized |= video_frame.width != self.prev_width;
                    self.has_resized |= video_frame.height != self.prev_height;
                } else {
                    self.started_at = Some(Utc::now());
                }

                self.prev_width = video_frame.width;
                self.prev_height = video_frame.height;
            },
            crate::FrameStatus::Dropped => {
                self.frames_dropped += 1;
            },
            _ => unreachable!(),
        }

        if video_frame.frame_number % 60 != 0 { return; }

        let started_at = *self.started_at.as_ref().unwrap();
        let elapsed = (Utc::now() - started_at).to_std().unwrap().as_secs();

        print!("{esc}c", esc = 27 as char); // Clear terminal.

        println!("Capturing frames to disk...");
        println!("Directory: {}", self.directory);
        println!();
        println!("Started at: {}", started_at.to_rfc3339_opts(SecondsFormat::Millis, true));
        println!("Duration: {:02}:{:02}:{:02}", elapsed / 3600, (elapsed % 3600) / 60, elapsed % 60);
        println!();
        println!("Frames captured: {}", self.frames_captured);
        println!("Frames dropped: {}", self.frames_dropped);
        println!();
        println!("Average frame rate: {:.1} Hz", video_frame.frame_number as f32 / elapsed as f32);
        println!("Average frame size: {:.2} MB", self.raw_video_size as f32 / self.frames_captured as f32 / 1000. / 1000.);
        println!();
        println!("Current resolution: {}x{}", video_frame.width, video_frame.height);
        println!("Viewport has resized: {}", if self.has_resized { "Yes" } else { "No" });
        println!();
        println!("Raw video size: {:.1} GB", self.raw_video_size as f32 / 1000. / 1000. / 1000.);

        let mut size_on_disk = 0;
        for result in fs::read_dir(&self.directory).unwrap() {
            let dir_entry = match result { Ok(d) => d, _ => continue };
            let filename = match dir_entry.file_name().into_string() { Ok(s) => s, _ => continue };
            if !filename.contains(filename_timestamp) { continue; }

            let num_bytes = dir_entry.metadata().unwrap().len();
            size_on_disk += num_bytes;
        }

        println!("Compressed size on disk: {:.1} GB", size_on_disk as f32 / 1000. / 1000. / 1000.);
        println!();
        println!("Compression ratio: {:.1}x", self.raw_video_size as f32 / size_on_disk as f32);
        println!("Average write speed to disk: {:.1} MB/s", size_on_disk as f32 / elapsed as f32 / 1000. / 1000.);
        println!();

        if let Some(queue_limit) = self.max_frames_queued.as_ref() {
            println!("Compression queue size: {} (limit={})", queue_size, queue_limit);
        } else {
            println!("Compression queue size: {} (no limit)", queue_size);
        }

        println!("Compression worker threads: {}", num_threads);
        println!();
        println!("LZ4 compression level: {}", self.lz4_compression_level);
        println!("GPU memory buffer size: {:.1} MB", video_frame.buffer_size_in_bytes.load(Ordering::Relaxed) as f32 / 1000. / 1000.);
    }
}
