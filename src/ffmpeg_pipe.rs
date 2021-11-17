use std::process::{Command, Child, Stdio};
use std::{io::Write, path::Path};
use std::thread;
use chrono::{DateTime, Utc, SecondsFormat};

pub struct FfmpegPipe {
    pub child: Option<Child>,
    pub timestamp: Option<DateTime<Utc>>,
    pub base_directory: Option<String>,
    pub filename: Option<String>,
    pub prev_bytes: Option<Vec<u8>>,
    pub ffmpeg_args: Vec<String>,
}

impl FfmpegPipe {
    pub fn new(base_directory: Option<&str>, filename: Option<&str>, ffmpeg_args: &[&str]) -> Self {
        let base_directory = base_directory.map(|s| s.to_string());
        let filename = filename.map(|s| s.to_string());
        let ffmpeg_args = ffmpeg_args.iter().map(|s| s.to_string()).collect();

        Self { child: None, timestamp: None, base_directory, filename, prev_bytes: None, ffmpeg_args }
    }

    pub fn available() -> bool {
        Command::new("ffmpeg").arg("-loglevel").arg("error").spawn().is_ok()
    }

    pub fn write(&mut self, video_frame: &crate::VideoFrame, png_bytes: Vec<u8>, timestamp: Option<&DateTime<Utc>>) {
        if png_bytes.is_empty() && self.prev_bytes.is_none() { return; }

        if self.child.is_none() || self.timestamp_has_changed(timestamp) {
            self.re_spawn_process(timestamp);
        }

        let child = self.child.as_mut().unwrap();
        let stdin = child.stdin.as_mut().unwrap();

        let duplicate_frame = png_bytes.is_empty();

        if duplicate_frame {
            eprintln!("Warning: Frame {} is {}. Duplicating previous frame to maintain a steady frame rate.", video_frame.frame_number, video_frame.status);

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

        let mut command = Command::new("ffmpeg");

        command.arg("-hide_banner").arg("-loglevel").arg("error").arg("-stats");
        command.arg("-f").arg("image2pipe");

        // TODO: Make this better. Ideally, we'd store a timestamp_offset on
        // each video frame since the start of the recording timestamp.
        //
        // We'd then use a Rust crate to do the encoding (e.g. rav1e) and
        // pass the explicit frame times through (variable frame rate - VRF).
        //
        // The timestamp_offset should be as close as possible to when the
        // frame is displayed on screen (maybe the time the render pass ends?).
        //
        // Doing this should make it easier to synchronize video with audio.
        command.arg("-framerate").arg("60");

        command.arg("-y").arg("-i").arg("-");

        for arg in &self.ffmpeg_args {
            command.arg(arg);
        }

        command.arg(&self.output_filename());
        command.stdin(Stdio::piped()).stdout(Stdio::piped());

        self.child = Some(command.spawn().unwrap());
    }

    fn output_filename(&self) -> String {
        let directory = self.base_directory.clone().unwrap_or_else(|| ".".to_string());

        let filename = self.filename.clone().unwrap_or_else(|| {
            let timestamp = self.timestamp.clone().unwrap_or_else(|| Utc::now());
            let formatted = timestamp.to_rfc3339_opts(SecondsFormat::Millis, true).replace(":", "_");

            format!("{}.mp4", formatted)
        });

        Path::new(&directory).join(&filename).into_os_string().into_string().unwrap()
    }
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
