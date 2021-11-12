use std::ops;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering::Relaxed}};

#[derive(Debug)]
#[cfg_attr(feature="bincode", derive(bincode::Encode))]
pub struct StreamFrame {
    pub image_data: Option<ImageData>,

    pub width: usize,
    pub height: usize,
    pub format: crate::Format,

    pub unpadded_bytes_per_row: usize,
    pub padded_bytes_per_row: usize,

    pub frame_number: usize,

    pub frame_size_in_bytes: usize,
    pub buffer_size_in_bytes: Arc<AtomicUsize>,
}

#[derive(Debug)]
pub struct ImageData(pub wgpu::Buffer);

impl Drop for StreamFrame {
    fn drop(&mut self) {
        self.buffer_size_in_bytes.fetch_sub(self.frame_size_in_bytes, Relaxed);
    }
}

impl ops::Deref for ImageData {
    type Target = wgpu::Buffer;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature="bincode")]
use bincode::enc::write::Writer;

#[cfg(feature="bincode")]
impl bincode::Encode for ImageData {
    fn encode<E: bincode::enc::Encoder>(&self, mut encoder: E) -> Result<(), bincode::error::EncodeError> {
        encoder.writer().write(&[])
    }
}
