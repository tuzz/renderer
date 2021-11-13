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
pub enum ImageData {
    Buffer(wgpu::Buffer),
    Bytes(Vec<u8>),
}

impl ImageData {
    pub fn buffer(&self) -> &wgpu::Buffer {
        match self {
            Self::Buffer(b) => b,
            Self::Bytes(_) => panic!("The buffer is no longer available."),
        }
    }

    pub fn bytes(&self) -> &[u8] {
        match self {
            Self::Buffer(_) => panic!("Please use ImageData::bytes_fn instead."),
            Self::Bytes(v) => v,
        }
    }

    pub fn bytes_fn<F: FnMut(&[u8])>(&self, mut f: F) {
        match self {
            Self::Buffer(b) => f(&b.slice(..).get_mapped_range()),
            Self::Bytes(v) => f(v),
        }
    }
}

impl Drop for StreamFrame {
    fn drop(&mut self) {
        self.buffer_size_in_bytes.fetch_sub(self.frame_size_in_bytes, Relaxed);
    }
}

impl Default for FrameStatus {
    fn default() -> Self {
        FrameStatus::Missing
    }
}

#[cfg(feature="bincode")]
impl bincode::Encode for ImageData {
    fn encode<E: bincode::enc::Encoder>(&self, _encoder: E) -> Result<(), bincode::error::EncodeError> {
        Ok(())
    }
}

#[cfg(feature="bincode")]
impl bincode::Decode for ImageData {
    fn decode<D: bincode::de::Decoder>(mut _decoder: D) -> Result<Self, bincode::error::DecodeError> {
        Ok(ImageData::Bytes(vec![]))
    }
}
