use std::{fs, thread, time, io::Write};
use crossbeam_channel::{Sender, Receiver};
use lzzzz::lz4f;

pub struct Compressor {
    pub timestamp: String,
    pub threads: Vec<thread::JoinHandle<()>>,
    pub sender: Option<Sender<crate::StreamFrame>>,
    pub receiver: Receiver<crate::StreamFrame>,
}

impl Compressor {
    pub fn new(directory: &str, max_frames_queued: Option<usize>, lz4_compression_level: u8) -> Self {
        let is_valid_level = lz4_compression_level as i32 <= lz4f::CLEVEL_MAX;
        assert!(is_valid_level, "Please choose a compression level in the range 0..={}", lz4f::CLEVEL_MAX);

        fs::create_dir_all(directory).unwrap();

        let timestamp = generate_timestamp();
        let (sender, receiver) = create_channel(max_frames_queued);

        let threads = (0..num_cpus::get()).map(|i| {
            spawn_thread(&receiver, &directory, &timestamp, i, lz4_compression_level)
        }).collect();

        Compressor { timestamp, threads, sender: Some(sender), receiver }
    }

    pub fn compress_to_disk(&self, stream_frame: crate::StreamFrame) {
        self.sender.as_ref().unwrap().send(stream_frame).unwrap();
    }

    pub fn finish(&mut self) {
        if self.sender.is_none() { return; }

        // Wait for the worker threads to exhaust the channel.
        while self.receiver.len() > 0 {
            thread::sleep(time::Duration::from_millis(10));
        }

        // Disconnect the channel so that the workers threads break.
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
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true).replace(":", "_")
}

// If max_frames_queued is set, create a bounded queue that blocks the main
// thread and slows the renderer down if the compression threads can't keep up.
//
// Otherwise, create an unbounded queue that won't block the main thread but
// will cause frames to be dropped if max_buffer_size_in_bytes is exceeded.
fn create_channel(max_frames_queued: Option<usize>) -> (Sender<crate::StreamFrame>, Receiver<crate::StreamFrame>){
    if let Some(channel_capacity) = max_frames_queued {
        crossbeam_channel::bounded::<crate::StreamFrame>(channel_capacity)
    } else {
        crossbeam_channel::unbounded::<crate::StreamFrame>()
    }
}

fn spawn_thread(receiver: &Receiver<crate::StreamFrame>, directory: &str, timestamp: &str, i: usize, lz4_compression_level: u8) -> thread::JoinHandle<()> {
    let receiver = receiver.clone();

    let preferences = lz4f::PreferencesBuilder::new()
        .compression_level(lz4_compression_level as i32)
        .favor_dec_speed(lz4f::FavorDecSpeed::Disabled)
        .auto_flush(lz4f::AutoFlush::Enabled)
        .build();

    let file = fs::File::create(format!("{}/{}--{}.sz", directory, timestamp, i)).unwrap();
    let mut writer = lz4f::WriteCompressor::new(file, preferences).unwrap();

    thread::spawn(move || {
        loop {
            let _stream_frame = match receiver.recv() { Ok(f) => f, _ => break };

            writer.write_all(b"hello").unwrap();
        }
    })
}
