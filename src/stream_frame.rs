use std::ops;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering::Relaxed}};

#[derive(Debug, Default)]
#[cfg_attr(feature="bincode", derive(bincode::Encode, bincode::Decode))]
pub struct StreamFrame {
    pub status: FrameStatus,
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
#[cfg_attr(feature="bincode", derive(bincode::Encode, bincode::Decode))]
pub enum FrameStatus {
    Captured, // The frame was captured successfully (image_data=Some)
    Dropped,  // The frame was dropped to save memory (image_data=None)
    Missing,  // The frame was missing from the compressed files (image_data=None)
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

impl Default for FrameStatus {
    fn default() -> Self {
        FrameStatus::Missing
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

#[cfg(feature="bincode")]
impl bincode::Decode for ImageData {
    fn decode<D: bincode::de::Decoder>(mut _decoder: D) -> Result<Self, bincode::error::DecodeError> {
        // TODO: replace with an ImageData enum
        #[allow(invalid_value, deprecated)]
        Ok(unsafe { std::mem::uninitialized::<ImageData>() })
    }
}
