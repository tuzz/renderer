use std::{fs, thread, time, io::Write};
use crossbeam_channel::{Sender, Receiver};

pub struct Compressor {
    pub timestamp: String,
    pub threads: Vec<thread::JoinHandle<()>>,
    pub sender: Option<Sender<crate::StreamFrame>>,
    pub receiver: Receiver<crate::StreamFrame>,
}

impl Compressor {
    pub fn new(directory: &str, max_frames_queued: Option<usize>) -> Self {
        fs::create_dir_all(directory).unwrap();

        let timestamp = generate_timestamp();
        let (sender, receiver) = create_channel(max_frames_queued);

        let threads = (0..num_cpus::get()).map(|i| {
            spawn_thread(&directory, &timestamp, i, &receiver)
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

fn spawn_thread(directory: &str, timestamp: &str, i: usize, receiver: &Receiver<crate::StreamFrame>) -> thread::JoinHandle<()> {
    let mut file = fs::File::create(format!("{}/{}--{}.sz", directory, timestamp, i)).unwrap();
    let receiver = receiver.clone();

    thread::spawn(move || {
        loop {
            let _stream_frame = match receiver.recv() { Ok(f) => f, _ => break };

            file.write_all(b"hello").unwrap();
        }
    })
}
