use std::{collections::VecDeque, rc, cell, pin};
use std::{future::Future, task::Context, task::Poll};
use std::sync::{Arc, atomic::{AtomicUsize, Ordering::Relaxed}};
use futures::FutureExt;
use noop_waker::noop_waker;

pub struct CaptureStream {
    pub max_buffer_size_in_bytes: usize,
    pub process_function: Box<dyn FnMut(crate::StreamFrame)>,
    pub inner: rc::Rc<cell::RefCell<Inner>>,
}

pub struct Inner {
    pub texture: crate::Texture,
    pub clear_color: Option<crate::ClearColor>,
    pub cleared_this_frame: bool,

    pub buffer_size_in_bytes: Arc<AtomicUsize>,
    pub stream_frames: VecDeque<crate::StreamFrame>,
    pub map_futures: VecDeque<Option<MapFuture>>,

    pub frame_number: usize,
}

pub struct MapFuture(pin::Pin<Box<dyn Future<Output=Result<(), wgpu::BufferAsyncError>>>>);

impl CaptureStream {
    pub fn new(renderer: &crate::Renderer, clear_color: Option<crate::ClearColor>, max_buffer_size_in_bytes: usize, process_function: Box<dyn FnMut(crate::StreamFrame)>) -> Self {
        let size = (renderer.window_size.width, renderer.window_size.height);

        let inner = Inner {
            texture: create_stream_texture(&renderer.device, size),
            cleared_this_frame: false,
            clear_color,

            buffer_size_in_bytes: Arc::new(AtomicUsize::new(0)),
            stream_frames: VecDeque::new(),
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

    pub fn create_buffer_if_within_memory_limit(&self, device: &wgpu::Device) {
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
        let drop_frame = prev_size > self.max_buffer_size_in_bytes;

        let buffer = if drop_frame {
            inner.buffer_size_in_bytes.store(prev_size, Relaxed); // Revert
            None
        } else {
            let usage =  wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ;
            let descriptor = wgpu::BufferDescriptor { label: None, size: frame_size_in_bytes as u64, usage, mapped_at_creation: false };

            Some(device.create_buffer(&descriptor))
        };

        // The frame number is incremented regardless of whether the frame is dropped.
        inner.frame_number += 1;

        let status = if drop_frame { crate::FrameStatus::Dropped } else { crate::FrameStatus::Captured };
        let image_data = buffer.map(|b| crate::ImageData(b));
        let frame_number = inner.frame_number;
        let buffer_size_in_bytes = Arc::clone(&inner.buffer_size_in_bytes);

        inner.stream_frames.push_back(crate::StreamFrame {
            status, image_data, format, width, height, unpadded_bytes_per_row, padded_bytes_per_row, frame_number, frame_size_in_bytes, buffer_size_in_bytes
        });
    }

    pub fn copy_texture_to_buffer_if_present(&self, encoder: &mut wgpu::CommandEncoder) {
        let inner = self.inner.borrow_mut();

        let stream_frame = inner.stream_frames.back().unwrap();
        let image_data = match &stream_frame.image_data { Some(b) => b, _ => return };

        let image_copy = inner.texture.image_copy_texture();

        let buffer_copy = wgpu::ImageCopyBuffer {
            buffer: &image_data,
            layout: inner.texture.image_data_layout(stream_frame.padded_bytes_per_row as u32),
        };

        encoder.copy_texture_to_buffer(image_copy, buffer_copy, inner.texture.extent());
    }

    pub fn initiate_buffer_mapping(&mut self) {
        let mut inner = self.inner.borrow_mut();

        for i in 0..inner.stream_frames.len() {
            if inner.map_futures.get(i).is_some() { continue; }
            let stream_frame = &inner.stream_frames[i];

            if let Some(buffer) = &stream_frame.image_data {
                let future = buffer.slice(..).map_async(wgpu::MapMode::Read);
                inner.map_futures.push_back(Some(MapFuture(future.boxed())));
            } else {
                inner.map_futures.push_back(None);
            }
        }
    }

    pub fn process_mapped_buffers(&mut self) {
        let mut inner = self.inner.borrow_mut();

        loop {
            if inner.stream_frames.is_empty() { break; }
            let option = &mut inner.map_futures[0];

            // If the frame was dropped, immediately call the process function.
            // Let it decide what to do with the dropped frame.
            if option.is_none() {
                let stream_frame = inner.stream_frames.pop_front().unwrap();
                inner.map_futures.pop_front().unwrap();

                (self.process_function)(stream_frame);
                continue;
            }

            let map_future = option.as_mut().unwrap();
            let waker = noop_waker();
            let mut context = Context::from_waker(&waker);

            match map_future.0.as_mut().poll(&mut context) {
                Poll::Pending => break,
                Poll::Ready(result) => {
                    result.unwrap(); // Panic if mapping failed.

                    let stream_frame = inner.stream_frames.pop_front().unwrap();
                    inner.map_futures.pop_front().unwrap();

                    (self.process_function)(stream_frame);
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
