use std::process::{Command, Child, Stdio};
use std::io::Write;
use std::thread;

pub struct FfmpegPipe {
    child: Option<Child>,
}

impl FfmpegPipe {
    pub fn new() -> Self {
        Self { child: None }
    }

    pub fn write(&mut self, png_bytes: &[u8]) {
        if self.child.is_none() {
            self.child = Some(spawn_process());
        }

        let child = self.child.as_mut().unwrap();
        let stdin = child.stdin.as_mut().unwrap();

        stdin.write_all(png_bytes).unwrap();
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
        let result = match self.child.take() { Some(mut c) => c.wait(), _ => return };

        // Don't panic while panicking if stdin already closed (broken pipe).
        if thread::panicking() { return; }

        let exit_status = result.unwrap();
        if !exit_status.success() {
            panic!("ffmpeg exited with {}", exit_status);
        }
    }
}
