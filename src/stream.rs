pub struct Stream {
    buffer: crate::Buffer,
    offset: u64,
    width: u32,
    height: u32,
    format: crate::Format,
}

impl Stream {
    pub fn buffer_copy_view(&self) -> wgpu::BufferCopyView {
        let offset = self.offset;
        let bytes_per_row = self.width * self.format.bytes_per_channel();
        let rows_per_image = self.height;

        wgpu::BufferCopyView {
            buffer: &self.buffer,
            layout: wgpu::TextureDataLayout { offset, bytes_per_row, rows_per_image },
        }
    }
}
