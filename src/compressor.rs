use std::{mem, fs, thread, time, io::{Write, BufWriter}};
use chrono::{SecondsFormat, Utc};
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
fn create_channel(max_frames_queued: Option<usize>) -> (Sender<crate::StreamFrame>, Receiver<crate::StreamFrame>){
    if let Some(channel_capacity) = max_frames_queued {
        crossbeam_channel::bounded::<crate::StreamFrame>(channel_capacity)
    } else {
        crossbeam_channel::unbounded::<crate::StreamFrame>()
    }
}

fn spawn_thread(receiver: &Receiver<crate::StreamFrame>, directory: &str, timestamp: &str, i: usize, lz4_compression_level: u8) -> thread::JoinHandle<()> {
    let receiver = receiver.clone();

    let compress_config = compression_config(lz4_compression_level);
    let encode_config = encoding_config();

    let file = fs::File::create(format!("{}/{}--{}.sz", directory, timestamp, i)).unwrap();
    let mut writer = lz4f::WriteCompressor::new(BufWriter::new(file), compress_config).unwrap();

    thread::spawn(move || {
        // When a stream_frame is received from the channel, write it to the
        // compressor in packets of bytes that have this layout:
        //
        // [ packet_len | stream_frame_len | stream_frame | image_data ]
        //     (u64)           (u64)          (bincode)        (raw)

        loop {
            let mut packet_len: u64 = (U64_LEN + U64_LEN) as u64;

            let stream_frame = match receiver.recv() { Ok(f) => f, _ => break };
            let stream_frame_bytes = bincode::encode_to_vec(&stream_frame, encode_config).unwrap();
            let stream_frame_len = stream_frame_bytes.len() as u64;
            packet_len += stream_frame_len;

            let mut write_image_data_bytes = None;

            if let Some(image_data) = &stream_frame.image_data {
                let image_data_bytes = image_data.buffer().slice(..).get_mapped_range();
                packet_len += image_data_bytes.len() as u64;

                write_image_data_bytes = Some(move |w: &mut Writer| {
                    w.write_all(&image_data_bytes).unwrap()
                });
            }

            writer.write_all(&packet_len.to_be_bytes()).unwrap();
            writer.write_all(&stream_frame_len.to_be_bytes()).unwrap();
            writer.write_all(&stream_frame_bytes).unwrap();
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
