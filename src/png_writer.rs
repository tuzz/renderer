use std::{thread, time};
use std::io::Write;
use crossbeam_channel::{Sender, Receiver};

pub struct PngWriter {
    pub threads: Vec<thread::JoinHandle<()>>,
    pub sender: Option<Sender::<(crate::StreamFrame, String)>>,
}

impl PngWriter {
    pub fn new() -> Self {
        let (sender, receiver) = crossbeam_channel::bounded(num_cpus::get());

        let threads = (0..num_cpus::get()).map(|_| spawn_thread(&receiver)).collect();

        Self { threads, sender: Some(sender) }
    }

    pub fn write_png(&self, stream_frame: crate::StreamFrame, filename: String) -> Result<(), &'static str> {
        if stream_frame.image_data.is_some() {
            self.sender.as_ref().unwrap().send((stream_frame, filename)).unwrap();
            Ok(())
        } else {
            Err("StreamFrame could not be written because image_data is None.")
        }
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

impl Drop for PngWriter {
    fn drop(&mut self) {
        self.finish();
    }
}

fn spawn_thread(receiver: &Receiver::<(crate::StreamFrame, String)>) -> thread::JoinHandle<()> {
    let receiver = receiver.clone();

    thread::spawn(move || {
        loop {
            let (stream_frame, filename) = match receiver.recv() { Ok(f) => f, _ => break };

            let file = std::fs::File::create(filename).unwrap();
            let mut png = png::Encoder::new(file, stream_frame.width as u32, stream_frame.height as u32);

            png.set_depth(png::BitDepth::Eight);
            png.set_color(png::ColorType::RGBA);

            let mut writer = png.write_header().unwrap().into_stream_writer_with_size(stream_frame.unpadded_bytes_per_row);
            let image_data = stream_frame.image_data.as_ref().unwrap();

            image_data.bytes_fn(|bytes| {
                for chunk in bytes.chunks(stream_frame.padded_bytes_per_row) {
                    writer.write_all(&chunk[..stream_frame.unpadded_bytes_per_row]).unwrap();
                }
            });

            writer.finish().unwrap();
        }
    })
}
