use std::future::Future;
use std::{rc, cell};

pub struct Stream {
    stream_buffers: rc::Rc<cell::RefCell<Vec<StreamBuffer>>>,
}

pub struct StreamBuffer {
    buffer: wgpu::Buffer,
    width: u32,
    height: u32,
    bytes_per_row: u32,
    row_padding: u32,
    map_future: Option<Box<dyn Future<Output=Result<(), wgpu::BufferAsyncError>>>>,
}

impl Stream {
    pub fn new() -> Self {
        Self { stream_buffers: rc::Rc::new(cell::RefCell::new(vec![])) }
    }

    pub fn create_buffer(&self, device: &wgpu::Device, texture: &crate::Texture) {
        let (width, height) = texture.size;

        let num_bytes = width * texture.format.bytes_per_texel();

        let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let row_padding = (alignment - num_bytes % alignment) % alignment;
        let bytes_per_row = num_bytes + row_padding;

        let size = (bytes_per_row * height) as u64;
        let usage =  wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::MAP_READ;

        let descriptor = wgpu::BufferDescriptor { label: None, size, usage, mapped_at_creation: false };
        let buffer = device.create_buffer(&descriptor);

        let mut vec = self.stream_buffers.borrow_mut();
        vec.push(StreamBuffer { buffer, width, height, bytes_per_row, row_padding, map_future: None });
    }

    pub fn copy_texture_to_buffer(&self, encoder: &mut wgpu::CommandEncoder, texture: &crate::Texture) {
        let vec = self.stream_buffers.borrow_mut();

        let stream_buffer = vec.last().unwrap();
        let texture_copy_view = texture.texture_copy_view((0, 0));

        let buffer_copy_view = wgpu::BufferCopyView {
            buffer: &stream_buffer.buffer,
            layout: wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: stream_buffer.bytes_per_row,
                rows_per_image: stream_buffer.height,
            },
        };

        encoder.copy_texture_to_buffer(texture_copy_view, buffer_copy_view, texture.extent());
    }

    pub fn initiate_buffer_mapping(&mut self) {
        for stream_buffer in self.stream_buffers.borrow_mut().iter_mut() {
            let slice = stream_buffer.buffer.slice(..);
            let future = slice.map_async(wgpu::MapMode::Read);

            stream_buffer.map_future = Some(Box::new(future));
        }
    }
}
