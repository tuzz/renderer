use std::process::{Command, Child, Stdio};
use std::io::Write;
use std::thread;
use chrono::{DateTime, Utc};

pub struct FfmpegPipe {
    process: Option<Process>,
}

struct Process {
    child: Child,
    timestamp: String,
}

impl FfmpegPipe {
    pub fn new() -> Self {
        Self { process: None }
    }

    pub fn available() -> bool {
        Command::new("ffmpeg").arg("-loglevel").arg("error").spawn().is_ok()
    }

    pub fn write(&mut self, stream_frame: &crate::StreamFrame, png_bytes: &[u8], timestamp: Option<&DateTime<Utc>>) {
        self.re_spawn_if_new_capture_timestamp(stream_frame);

//
//        if self.child.is_none() { self.child = Some(spawn_process()); }
//
//        let child = self.child.as_mut().unwrap();
//        let stdin = child.stdin.as_mut().unwrap();
//
//        stdin.write_all(png_bytes).unwrap();
    }

    fn re_spawn_if_new_capture_timestamp(&mut self, stream_frame: &crate::StreamFrame) {
        if let Some(process) = self.process.as_ref() {
            //if process.timestamp != stream_frame.timestamp
        }
    }
}

fn spawn_process() -> Child {
    Command::new("ffmpeg")
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
        .unwrap()
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
