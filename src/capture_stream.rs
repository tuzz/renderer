use std::{collections::VecDeque, rc, cell, pin, fmt};
use std::{future::Future, task::Context, task::Poll};
use std::sync::{Arc, atomic::{AtomicUsize, Ordering::Relaxed}};
use futures::FutureExt;
use noop_waker::noop_waker;

pub struct CaptureStream {
    pub max_buffer_size_in_bytes: usize,
    pub process_function: Box<dyn FnMut(StreamFrame)>,
    pub inner: rc::Rc<cell::RefCell<Inner>>,
}

pub struct Inner {
    pub texture: crate::Texture,
    pub clear_color: Option<crate::ClearColor>,
    pub cleared_this_frame: bool,

    pub buffer_size_in_bytes: Arc<AtomicUsize>,
    pub stream_buffers: VecDeque<StreamFrame>,
    pub map_futures: VecDeque<MapFuture>,

    pub frame_number: usize,
}

#[derive(Debug)]
pub struct StreamFrame {
    pub buffer: wgpu::Buffer,
    pub format: crate::Format,

    pub width: usize,
    pub height: usize,

    pub unpadded_bytes_per_row: usize,
    pub padded_bytes_per_row: usize,

    pub frame_number: usize,

    pub frame_size_in_bytes: usize,
    pub buffer_size_in_bytes: Arc<AtomicUsize>,
}

impl CaptureStream {
    pub fn new(renderer: &crate::Renderer, clear_color: Option<crate::ClearColor>, max_buffer_size_in_bytes: usize, process_function: Box<dyn FnMut(StreamFrame)>) -> Self {
        let size = (renderer.window_size.width, renderer.window_size.height);

        let inner = Inner {
            texture: create_stream_texture(&renderer.device, size),
            cleared_this_frame: false,
            clear_color,

            buffer_size_in_bytes: Arc::new(AtomicUsize::new(0)),
            stream_buffers: VecDeque::new(),
            map_futures: VecDeque::new(),

            frame_number: 0,
        };

        Self { max_buffer_size_in_bytes, process_function, inner: rc::Rc::new(cell::RefCell::new(inner)) }
    }

    pub fn color_attachment(&self) -> wgpu::RenderPassColorAttachment {
        let mut inner = self.inner.borrow_mut();

        let load = if inner.cleared_this_frame || inner.clear_color.is_none() {
            wgpu::LoadOp::Load
        } else {
            inner.cleared_this_frame = true;
            wgpu::LoadOp::Clear(inner.clear_color.as_ref().unwrap().inner)
        };

        let store = true;
        let ops = wgpu::Operations { load, store };

        drop(inner);
        let inner = unsafe { self.inner.try_borrow_unguarded().unwrap() };

        wgpu::RenderPassColorAttachment { view: &inner.texture.view, resolve_target: None, ops }
    }

    pub fn finish_frame(&self) {
        self.inner.borrow_mut().cleared_this_frame = false;
    }

    pub fn try_create_buffer(&self, device: &wgpu::Device) -> bool {
        let mut inner = self.inner.borrow_mut();

        let width = inner.texture.size.0 as usize;
        let height = inner.texture.size.1 as usize;
        let format = inner.texture.format;

        let unpadded_bytes_per_row = width * format.bytes_per_texel() as usize;

        let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let row_padding = (alignment - unpadded_bytes_per_row % alignment) % alignment;

        let padded_bytes_per_row = unpadded_bytes_per_row + row_padding;
        let frame_size_in_bytes = padded_bytes_per_row * height;

        let prev_size = inner.buffer_size_in_bytes.fetch_add(frame_size_in_bytes, Relaxed);

        inner.frame_number += 1;

        if prev_size > self.max_buffer_size_in_bytes {
            eprintln!("Frame {} dropped from CaptureStream because the maximum buffer size of {} bytes was exceeded.", inner.frame_number, self.max_buffer_size_in_bytes);
            inner.buffer_size_in_bytes.store(prev_size, Relaxed);

            return false;
        }

        let usage =  wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ;
        let descriptor = wgpu::BufferDescriptor { label: None, size: frame_size_in_bytes as u64, usage, mapped_at_creation: false };

        let buffer = device.create_buffer(&descriptor);
        let frame_number = inner.frame_number;
        let buffer_size_in_bytes = Arc::clone(&inner.buffer_size_in_bytes);

        inner.stream_buffers.push_back(StreamFrame {
            buffer, format, width, height, unpadded_bytes_per_row, padded_bytes_per_row, frame_number, frame_size_in_bytes, buffer_size_in_bytes
        });

        true
    }

    pub fn copy_texture_to_buffer(&self, encoder: &mut wgpu::CommandEncoder) {
        let inner = self.inner.borrow_mut();

        let stream_buffer = inner.stream_buffers.back().unwrap();
        let image_copy = inner.texture.image_copy_texture();

        let buffer_copy = wgpu::ImageCopyBuffer {
            buffer: &stream_buffer.buffer,
            layout: inner.texture.image_data_layout(stream_buffer.padded_bytes_per_row as u32),
        };

        encoder.copy_texture_to_buffer(image_copy, buffer_copy, inner.texture.extent());
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

                    (self.process_function)(stream_buffer);
                },
            }
        }
    }
}

fn create_stream_texture(device: &wgpu::Device, size: (u32, u32)) -> crate::Texture {
    let filter_mode = crate::FilterMode::Nearest; // Not used
    let format = crate::Format::RgbaU8;
    let msaa_samples = 1;
    let renderable = true;
    let copyable = true;
    let with_sampler = false;

    crate::Texture::new(device, size, filter_mode, format, msaa_samples, renderable, copyable, with_sampler)
}

pub struct MapFuture(pin::Pin<Box<dyn Future<Output=Result<(), wgpu::BufferAsyncError>>>>);

impl fmt::Debug for MapFuture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MapFuture").finish()
    }
}

impl Drop for StreamFrame {
    fn drop(&mut self) {
        self.buffer_size_in_bytes.fetch_sub(self.frame_size_in_bytes, Relaxed);
    }
}
