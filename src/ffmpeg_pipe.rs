use std::process::{Command, Child, Stdio};
use std::{io::Write, path::Path};
use std::thread;
use chrono::{DateTime, Utc, SecondsFormat};

pub struct FfmpegPipe {
    pub audio_directory: Option<String>,
    pub output_directory: Option<String>,
    pub output_filename: Option<String>,
    pub ffmpeg_args: Vec<String>,

    pub child: Option<Child>,
    pub timestamp: Option<DateTime<Utc>>,
    pub prev_bytes: Option<Vec<u8>>,
}

// If audio_directory is provided, looks for an audio file with the same name as
// the output_filename (or the timestamp) in that directory, e.g. recorded.wav

impl FfmpegPipe {
    pub fn new(audio_directory: Option<&str>, output_directory: Option<&str>, output_filename: Option<&str>, ffmpeg_args: &[&str]) -> Self {
        let audio_directory = audio_directory.map(|s| s.to_string());
        let output_directory = output_directory.map(|s| s.to_string());
        let output_filename = output_filename.map(|s| s.to_string());
        let ffmpeg_args = ffmpeg_args.iter().map(|s| s.to_string()).collect();

        Self { audio_directory, output_directory, output_filename, ffmpeg_args, child: None, timestamp: None, prev_bytes: None }
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

        if let Some(output_filename) = self.output_filename.as_ref() {
            eprintln!("Warning: Compressed data contains multiple videos but only writing one file: {}", output_filename);
        }

        true
    }

    fn re_spawn_process(&mut self, timestamp: Option<&DateTime<Utc>>) {
        self.timestamp = timestamp.cloned();

        let mut command = Command::new("ffmpeg");

        command.arg("-hide_banner").arg("-loglevel").arg("error").arg("-stats");
        command.arg("-f").arg("image2pipe");

        // TODO: Make this better. Ideally, we'd store elapsed_time on each
        // video frame since the start of the recording timestamp.
        //
        // We'd then use a Rust crate to do the encoding (e.g. rav1e) and
        // pass the explicit frame times through (variable frame rate - VRF).
        //
        // The elapsed_time should be as close as possible to when the frame is
        // displayed on screen (maybe the time the render pass ends?).
        //
        // Doing this should make it easier to synchronize video with audio from
        // my AudioMixer crate which uses a similar pattern.
        command.arg("-framerate").arg("60");

        command.arg("-y").arg("-i").arg("-");

        let (output_filename, output_path) = self.output_filename_and_path();

        if let Some(wav_filename) = self.look_for_wav_file(&output_filename) {
            command.arg("-i").arg(wav_filename);
        }

        for arg in &self.ffmpeg_args {
            command.arg(arg);
        }

        command.arg(output_path);
        command.stdin(Stdio::piped()).stdout(Stdio::piped());

        self.child = Some(command.spawn().unwrap());
    }

    fn output_filename_and_path(&self) -> (String, String) {
        let directory = self.output_directory.clone().unwrap_or_else(|| ".".to_string());

        let filename = self.output_filename.clone().unwrap_or_else(|| {
            let timestamp = self.timestamp.clone().unwrap_or_else(|| Utc::now());
            let formatted = timestamp.to_rfc3339_opts(SecondsFormat::Millis, true).replace(":", "_");

            format!("{}.mp4", formatted)
        });

        let path = Path::new(&directory).join(&filename).into_os_string().into_string().unwrap();

        (filename, path)
    }

    fn look_for_wav_file(&self, output_filename: &str) -> Option<String> {
        if let Some(directory) = self.audio_directory.as_ref() {
            let mut path_buf = Path::new(directory).join(output_filename).to_path_buf();
            path_buf.set_extension("wav");

            let found = path_buf.exists();
            let path = path_buf.into_os_string().into_string().unwrap();

            if found {
                return Some(path);
            } else {
                eprintln!("Skipping audio because {} doesn't exist", path);
            }
        }

        None
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
