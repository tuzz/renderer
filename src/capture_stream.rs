use std::{collections::VecDeque, rc, cell, pin, fmt};
use std::{future::Future, task::Context, task::Poll};
use futures::FutureExt;
use noop_waker::noop_waker;

pub struct CaptureStream {
    pub max_buffer_size_in_bytes: usize,
    pub process_function: Box<dyn FnMut(StreamBuffer, StreamInfo)>,

    inner: rc::Rc<cell::RefCell<Inner>>,
}

pub struct Inner {
    current_buffer_size_in_bytes: usize,
    stream_buffers: VecDeque<StreamBuffer>,
    map_futures: VecDeque<MapFuture>,
}

#[derive(Debug)]
pub struct StreamBuffer {
    pub buffer: wgpu::Buffer,
    pub format: crate::Format,
    pub size_in_bytes: usize,

    pub width: usize,
    pub height: usize,

    pub unpadded_bytes_per_row: usize,
    pub padded_bytes_per_row: usize,
}

#[derive(Debug)]
pub struct StreamInfo {
    pub number_of_frames_behind: usize,
    pub current_buffer_size_in_bytes: usize,
    pub max_buffer_size_in_bytes: usize,
}

impl CaptureStream {
    pub fn new(max_buffer_size_in_bytes: usize, process_function: Box<dyn FnMut(StreamBuffer, StreamInfo)>) -> Self {
        let inner = Inner { current_buffer_size_in_bytes: 0, stream_buffers: VecDeque::new(), map_futures: VecDeque::new() };

        Self { max_buffer_size_in_bytes, process_function, inner: rc::Rc::new(cell::RefCell::new(inner)) }
    }

    pub fn try_create_buffer(&self, device: &wgpu::Device, texture: &crate::Texture) -> bool {
        let width = texture.size.0 as usize;
        let height = texture.size.1 as usize;

        let unpadded_bytes_per_row = width * texture.format.bytes_per_texel() as usize;

        let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let row_padding = (alignment - unpadded_bytes_per_row % alignment) % alignment;

        let padded_bytes_per_row = unpadded_bytes_per_row + row_padding;
        let size_in_bytes = padded_bytes_per_row * height;

        let mut inner = self.inner.borrow_mut();
        let new_size = inner.current_buffer_size_in_bytes + size_in_bytes;

        if new_size > self.max_buffer_size_in_bytes {
            eprintln!("Frame dropped from CaptureStream because the maximum buffer size of {} bytes was exceeded.", self.max_buffer_size_in_bytes);
            return false;
        }

        let usage =  wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ;
        let descriptor = wgpu::BufferDescriptor { label: None, size: size_in_bytes as u64, usage, mapped_at_creation: false };

        let buffer = device.create_buffer(&descriptor);
        let format = texture.format;

        inner.stream_buffers.push_back(StreamBuffer { buffer, format, size_in_bytes, width, height, unpadded_bytes_per_row, padded_bytes_per_row });
        inner.current_buffer_size_in_bytes = new_size;

        true
    }

    pub fn copy_texture_to_buffer(&self, encoder: &mut wgpu::CommandEncoder, texture: &crate::Texture) {
        let inner = self.inner.borrow_mut();

        let stream_buffer = inner.stream_buffers.back().unwrap();
        let image_copy = texture.image_copy_texture();

        let buffer_copy = wgpu::ImageCopyBuffer {
            buffer: &stream_buffer.buffer,
            layout: texture.image_data_layout(stream_buffer.padded_bytes_per_row as u32),
        };

        encoder.copy_texture_to_buffer(image_copy, buffer_copy, texture.extent());
    }

    pub fn initiate_buffer_mapping(&mut self) {
        let mut inner = self.inner.borrow_mut();

        for i in 0..inner.stream_buffers.len() {
            if inner.map_futures.get(i).is_some() { continue; }

            let slice = inner.stream_buffers[i].buffer.slice(..);
            let future = slice.map_async(wgpu::MapMode::Read);

            inner.map_futures.push_back(MapFuture(future.boxed()));
        }
    }

    pub fn process_mapped_buffers(&mut self) {
        let mut inner = self.inner.borrow_mut();

        loop {
            if inner.map_futures.is_empty() { break; }
            let map_future = &mut inner.map_futures[0];

            let waker = noop_waker();
            let mut context = Context::from_waker(&waker);

            match map_future.0.as_mut().poll(&mut context) {
                Poll::Pending => break,
                Poll::Ready(result) => {
                    result.unwrap(); // Panic if mapping failed.

                    let stream_buffer = inner.stream_buffers.pop_front().unwrap();
                    inner.map_futures.pop_front().unwrap();

                    inner.current_buffer_size_in_bytes -= stream_buffer.size_in_bytes as usize;

                    let stream_info = StreamInfo {
                        number_of_frames_behind: inner.stream_buffers.len(),
                        current_buffer_size_in_bytes: inner.current_buffer_size_in_bytes,
                        max_buffer_size_in_bytes: self.max_buffer_size_in_bytes,
                    };

                    (self.process_function)(stream_buffer, stream_info);
                },
            }
        }
    }
}

struct MapFuture(pin::Pin<Box<dyn Future<Output=Result<(), wgpu::BufferAsyncError>>>>);

impl fmt::Debug for MapFuture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MapFuture").finish()
    }
}
