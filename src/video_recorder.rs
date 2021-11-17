use std::{collections::VecDeque, rc, cell, pin};
use std::{future::Future, task::Context, task::Poll};
use std::sync::{Arc, atomic::{AtomicUsize, Ordering::Relaxed}};
use futures::FutureExt;
use noop_waker::noop_waker;

pub struct VideoRecorder {
    pub max_buffer_size_in_bytes: usize,
    pub process_function: Box<dyn FnMut(crate::VideoFrame)>,
    pub inner: rc::Rc<cell::RefCell<Inner>>,
}

pub struct Inner {
    pub recording_texture: crate::Texture,
    pub clear_color: Option<crate::ClearColor>,
    pub cleared_this_frame: bool,

    pub buffer_size_in_bytes: Arc<AtomicUsize>,
    pub video_frames: VecDeque<crate::VideoFrame>,
    pub map_futures: VecDeque<Option<MapFuture>>,

    pub frame_number: usize,
}

pub struct MapFuture(pin::Pin<Box<dyn Future<Output=Result<(), wgpu::BufferAsyncError>>>>);

impl VideoRecorder {
    pub fn new(renderer: &crate::Renderer, clear_color: Option<crate::ClearColor>, max_buffer_size_in_bytes: usize, process_function: Box<dyn FnMut(crate::VideoFrame)>) -> Self {
        let size = (renderer.window_size.width, renderer.window_size.height);

        let inner = Inner {
            recording_texture: create_recording_texture(&renderer.device, size),
            cleared_this_frame: false,
            clear_color,

            buffer_size_in_bytes: Arc::new(AtomicUsize::new(0)),
            video_frames: VecDeque::new(),
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

        wgpu::RenderPassColorAttachment { view: &inner.recording_texture.view, resolve_target: None, ops }
    }

    pub fn finish_frame(&self) {
        self.inner.borrow_mut().cleared_this_frame = false;
    }

    pub fn create_buffer_if_within_memory_limit(&self, device: &wgpu::Device, viewport: Option<&crate::Viewport>) {
        let mut inner = self.inner.borrow_mut();

        let width = viewport.map(|v| v.width.floor() as usize).unwrap_or(inner.recording_texture.size.0 as usize);
        let height = viewport.map(|v| v.height.floor() as usize).unwrap_or(inner.recording_texture.size.1 as usize);
        let format = inner.recording_texture.format;

        let unpadded_bytes_per_row = width * format.bytes_per_texel() as usize;

        let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let row_padding = (alignment - unpadded_bytes_per_row % alignment) % alignment;

        let padded_bytes_per_row = unpadded_bytes_per_row + row_padding;
        let frame_size_in_bytes = padded_bytes_per_row * height;

        let prev_size = inner.buffer_size_in_bytes.fetch_add(frame_size_in_bytes, Relaxed);
        let drop_frame = prev_size > self.max_buffer_size_in_bytes;

        let buffer = if drop_frame {
            inner.buffer_size_in_bytes.fetch_sub(frame_size_in_bytes, Relaxed);
            None
        } else {
            let usage =  wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ;
            let descriptor = wgpu::BufferDescriptor { label: None, size: frame_size_in_bytes as u64, usage, mapped_at_creation: false };

            Some(device.create_buffer(&descriptor))
        };

        // The frame number is incremented regardless of whether the frame is dropped.
        inner.frame_number += 1;

        let status = if drop_frame { crate::FrameStatus::Dropped } else { crate::FrameStatus::Captured };
        let image_data = buffer.map(|b| crate::ImageData::Buffer(b));
        let frame_number = inner.frame_number;
        let buffer_size_in_bytes = Arc::clone(&inner.buffer_size_in_bytes);

        inner.video_frames.push_back(crate::VideoFrame {
            status, image_data, format, width, height, unpadded_bytes_per_row, padded_bytes_per_row, frame_number, frame_size_in_bytes, buffer_size_in_bytes
        });
    }

    pub fn copy_texture_to_buffer_if_present(&self, encoder: &mut wgpu::CommandEncoder, viewport: Option<&crate::Viewport>) {
        let inner = self.inner.borrow_mut();

        let video_frame = inner.video_frames.back().unwrap();
        let image_data = match &video_frame.image_data { Some(d) => d, _ => return };

        let margin_x = viewport.map(|v| v.margin_x.ceil() as u32).unwrap_or(0);
        let margin_y = viewport.map(|v| v.margin_y.ceil() as u32).unwrap_or(0);

        let image_copy = inner.recording_texture.image_copy_texture((margin_x, margin_y));

        let buffer_copy = wgpu::ImageCopyBuffer {
            buffer: image_data.buffer(),
            layout: inner.recording_texture.image_data_layout(video_frame.padded_bytes_per_row as u32),
        };

        let mut extent = inner.recording_texture.extent();
        extent.width -= 2 * margin_x;
        extent.height -= 2 * margin_y;

        encoder.copy_texture_to_buffer(image_copy, buffer_copy, extent);
    }

    pub fn initiate_buffer_mapping(&mut self) {
        let mut inner = self.inner.borrow_mut();

        for i in 0..inner.video_frames.len() {
            if inner.map_futures.get(i).is_some() { continue; }
            let video_frame = &inner.video_frames[i];

            if let Some(image_data) = &video_frame.image_data {
                let future = image_data.buffer().slice(..).map_async(wgpu::MapMode::Read);
                inner.map_futures.push_back(Some(MapFuture(future.boxed())));
            } else {
                inner.map_futures.push_back(None);
            }
        }
    }

    pub fn process_mapped_buffers(&mut self) {
        let mut inner = self.inner.borrow_mut();

        loop {
            if inner.video_frames.is_empty() { break; }
            let option = &mut inner.map_futures[0];

            // If the frame was dropped, immediately call the process function.
            // Let it decide what to do with the dropped frame.
            if option.is_none() {
                let video_frame = inner.video_frames.pop_front().unwrap();
                inner.map_futures.pop_front().unwrap();

                (self.process_function)(video_frame);
                continue;
            }

            let map_future = option.as_mut().unwrap();
            let waker = noop_waker();
            let mut context = Context::from_waker(&waker);

            match map_future.0.as_mut().poll(&mut context) {
                Poll::Pending => break,
                Poll::Ready(result) => {
                    result.unwrap(); // Panic if mapping failed.

                    let video_frame = inner.video_frames.pop_front().unwrap();
                    inner.map_futures.pop_front().unwrap();

                    (self.process_function)(video_frame);
                },
            }
        }
    }
}

fn create_recording_texture(device: &wgpu::Device, size: (u32, u32)) -> crate::Texture {
    let filter_mode = crate::FilterMode::Nearest; // Not used
    let format = crate::Format::RgbaU8;
    let msaa_samples = 1;
    let renderable = true;
    let copyable = true;
    let with_sampler = false;

    crate::Texture::new(device, size, filter_mode, format, msaa_samples, renderable, copyable, with_sampler)
}
