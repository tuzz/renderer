use std::thread;
use std::sync::{Arc, atomic::AtomicUsize};

pub struct Stream {
    pub buffer: crate::Buffer,
    pub offset: u64,
    pub width: u32,
    pub height: u32,
    pub format: crate::Format,
    pub process: Box<dyn Fn(wgpu::BufferView)>,
}

impl Stream {
    pub fn new(device: &wgpu::Device) {
        device.poll(wgpu::Maintain::Poll);
    }

    pub async fn foo(&self) {
        let buffer_slice = self.buffer.slice(..);
        let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);

        if let Ok(()) = buffer_future.await {
            let padded_buffer = buffer_slice.get_mapped_range();
            (self.process)(padded_buffer);
        }
    }

    pub fn buffer_copy_view_for_next_frame(&self) -> wgpu::BufferCopyView {
        let offset = self.offset;
        let unpadded_bytes_per_row = (self.width * self.format.bytes_per_texel()) as usize;

        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded_bytes_per_row_padding = (align - unpadded_bytes_per_row % align) % align;
        let padded_bytes_per_row = unpadded_bytes_per_row + padded_bytes_per_row_padding;

        wgpu::BufferCopyView {
            buffer: &self.buffer,
            layout: wgpu::TextureDataLayout {
                offset,
                bytes_per_row: padded_bytes_per_row as u32,
                rows_per_image: self.height,
            },
        }
    }
}
