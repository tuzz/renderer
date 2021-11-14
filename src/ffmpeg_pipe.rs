use std::process::{Command, Child, Stdio};
use std::io::Write;
use std::thread;
use chrono::{DateTime, Utc};

pub struct FfmpegPipe {
    process: Option<Process>,
}

struct Process {
    child: Child,
    timestamp: DateTime<Utc>,
}

impl FfmpegPipe {
    pub fn new() -> Self {
        Self { process: None }
    }

    pub fn available() -> bool {
        Command::new("ffmpeg").arg("-loglevel").arg("error").spawn().is_ok()
    }

    pub fn write(&mut self, _stream_frame: &crate::StreamFrame, png_bytes: &[u8], timestamp: Option<&DateTime<Utc>>) {
        self.re_spawn_if_new_capture_timestamp(timestamp);

        let process = self.process.as_mut().unwrap();
        let stdin = process.child.stdin.as_mut().unwrap();

        stdin.write_all(png_bytes).unwrap();
    }

    fn re_spawn_if_new_capture_timestamp(&mut self, timestamp: Option<&DateTime<Utc>>) {
        match (timestamp, self.process.as_ref().map(|p| p.timestamp)) {
            (Some(t1), Some(t2)) if *t1 != t2 => { self.process = None; }
            _ => {},
        }

        if self.process.is_some() { return; }

        let child = Command::new("ffmpeg")
            .arg("-hide_banner")
            .arg("-loglevel")
            .arg("error")
            .arg("-stats")
            .arg("-f")
            .arg("image2pipe")
            .arg("-y")
            .arg("-framerate")
            .arg("60")
            .arg("-i")
            .arg("-")
            .arg("-c:v")
            .arg("libx264")
            .arg("-r")
            .arg("60")
            .arg("-pix_fmt")
            .arg("yuv420p")
            .arg("out.mp4")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let timestamp = timestamp.cloned().unwrap_or_else(|| Utc::now());

        self.process = Some(Process { child, timestamp });
    }
}

impl Drop for FfmpegPipe {
    fn drop(&mut self) {
        let mut process = match self.process.take() { Some(p) => p, _ => return };
        let result = process.child.wait();

        // Don't panic while panicking if stdin already closed (broken pipe).
        if thread::panicking() { return; }

        let exit_status = result.unwrap();
        if !exit_status.success() {
            panic!("ffmpeg exited with {}", exit_status);
        }
    }
}
