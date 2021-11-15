use std::process::{Command, Child, Stdio};
use std::io::Write;
use std::thread;
use chrono::{DateTime, Utc, SecondsFormat};

pub struct FfmpegPipe {
    pub child: Option<Child>,
    pub timestamp: Option<DateTime<Utc>>,
    pub filename: Option<String>,
    pub prev_bytes: Option<Vec<u8>>,
}

impl FfmpegPipe {
    pub fn new(filename: Option<&str>) -> Self {
        let filename = filename.map(|s| s.to_string());

        Self { child: None, timestamp: None, filename, prev_bytes: None }
    }

    pub fn available() -> bool {
        Command::new("ffmpeg").arg("-loglevel").arg("error").spawn().is_ok()
    }

    pub fn write(&mut self, stream_frame: &crate::StreamFrame, png_bytes: Vec<u8>, timestamp: Option<&DateTime<Utc>>) {
        if png_bytes.is_empty() && self.prev_bytes.is_none() { return; }

        if self.child.is_none() || self.timestamp_has_changed(timestamp) {
            self.re_spawn_process(timestamp);
        }

        let child = self.child.as_mut().unwrap();
        let stdin = child.stdin.as_mut().unwrap();

        let duplicate_frame = png_bytes.is_empty();

        if duplicate_frame {
            eprintln!("Warning: Frame {} is {}. Duplicating previous frame to maintain a steady frame rate.", stream_frame.frame_number, stream_frame.status);

            let duplicate = self.prev_bytes.as_ref().unwrap();
            stdin.write_all(duplicate).unwrap();
        } else {
            stdin.write_all(&png_bytes).unwrap();
            self.prev_bytes = Some(png_bytes);
        }
    }

    fn timestamp_has_changed(&self, timestamp: Option<&DateTime<Utc>>) -> bool {
        if timestamp == self.timestamp.as_ref() { return false; }

        if let Some(filename) = self.filename.as_ref() {
            eprintln!("Warning: Compressed data contains multiple videos but only writing one file: {}", filename);
        }

        true
    }

    fn re_spawn_process(&mut self, timestamp: Option<&DateTime<Utc>>) {
        self.timestamp = timestamp.cloned();

        if self.filename.is_none() {
            self.filename = Some(filename_for(timestamp));
        }

        self.child = Some(Command::new("ffmpeg")
            .arg("-hide_banner")
            .arg("-loglevel")
            .arg("error")
            .arg("-stats")
            .arg("-f")
            .arg("image2pipe")

            // TODO: Make this better. Ideally, we'd store a timestamp_offset on
            // each stream frame since the start of the capture timestamp.
            //
            // We'd then use a Rust crate to do the encoding (e.g. rav1e) and
            // pass the explicit frame times through (variable frame rate - VRF).
            //
            // The timestamp_offset should be as close as possible to when the
            // frame is displayed on screen (maybe the time the render pass ends?).
            //
            // Doing this should make it easier to synchronize video with audio.
            .arg("-framerate")
            .arg("60")

            .arg("-y")
            .arg("-i")
            .arg("-")

            .arg("-c:v")
            .arg("libx264")
            .arg("-r")
            .arg("60")
            .arg("-pix_fmt")
            .arg("yuv420p")

            .arg(self.filename.as_ref().unwrap())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap()
        );
    }
}

fn filename_for(timestamp: Option<&DateTime<Utc>>) -> String {
    let timestamp = timestamp.cloned().unwrap_or_else(|| Utc::now());
    let formatted = timestamp.to_rfc3339_opts(SecondsFormat::Millis, true).replace(":", "_");

    format!("{}.mp4", formatted)
}

impl Drop for FfmpegPipe {
    fn drop(&mut self) {
        let mut child = match self.child.take() { Some(p) => p, _ => return };
        let result = child.wait();

        // Don't panic while panicking if stdin already closed (broken pipe).
        if thread::panicking() { return; }

        let exit_status = result.unwrap();
        if !exit_status.success() {
            panic!("ffmpeg exited with {}", exit_status);
        }
    }
}
