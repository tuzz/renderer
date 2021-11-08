use std::{collections::VecDeque, rc, cell, pin};
use std::{future::Future, task::Context, task::Poll};
use futures::FutureExt;
use noop_waker::noop_waker;

pub struct CaptureStream {
    pub process_function: Box<dyn FnMut(StreamBuffer)>,
    pub stream_buffers: rc::Rc<cell::RefCell<VecDeque<StreamBuffer>>>,
}

pub struct StreamBuffer {
    pub buffer: wgpu::Buffer,
    pub width: u32,
    pub height: u32,
    pub bytes_per_row: u32,
    pub row_padding: u32,
    pub size_in_bytes: u64,

    map_future: Option<pin::Pin<Box<dyn Future<Output=Result<(), wgpu::BufferAsyncError>>>>>
}

impl CaptureStream {
    pub fn new(process_function: Box<dyn FnMut(StreamBuffer)>) -> Self {
        let stream_buffers = rc::Rc::new(cell::RefCell::new(VecDeque::new()));

        Self { process_function, stream_buffers }
    }

    pub fn create_buffer(&self, device: &wgpu::Device, texture: &crate::Texture) {
        let (width, height) = texture.size;

        let num_bytes = width * texture.format.bytes_per_texel();

        let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let row_padding = (alignment - num_bytes % alignment) % alignment;
        let bytes_per_row = num_bytes + row_padding;

        let size_in_bytes = (bytes_per_row * height) as u64;
        let usage =  wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::MAP_READ;

        let descriptor = wgpu::BufferDescriptor { label: None, size: size_in_bytes, usage, mapped_at_creation: false };
        let buffer = device.create_buffer(&descriptor);

        let mut queue = self.stream_buffers.borrow_mut();
        queue.push_back(StreamBuffer { buffer, width, height, bytes_per_row, row_padding, size_in_bytes, map_future: None });
    }

    pub fn copy_texture_to_buffer(&self, encoder: &mut wgpu::CommandEncoder, texture: &crate::Texture) {
        let queue = self.stream_buffers.borrow_mut();

        let stream_buffer = queue.back().unwrap();
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
            if stream_buffer.map_future.is_some() { continue; }

            let slice = stream_buffer.buffer.slice(..);
            let future = slice.map_async(wgpu::MapMode::Read);

            stream_buffer.map_future = Some(future.boxed());
        }
    }

    pub fn process_mapped_buffers(&mut self) {
        let mut queue = self.stream_buffers.borrow_mut();

        loop {
            if queue.is_empty() { break; }

            let stream_buffer = &mut queue[0];
            let future = stream_buffer.map_future.as_mut().unwrap();

            let waker = noop_waker();
            let mut context = Context::from_waker(&waker);

            match future.as_mut().poll(&mut context) {
                Poll::Pending => break,
                Poll::Ready(result) => {
                    result.unwrap(); // Panic if mapping failed.

                    let stream_buffer = queue.pop_front().unwrap();
                    (self.process_function)(stream_buffer);
                },
            }
        }
    }
}
