use std::{collections::VecDeque, rc, cell};
use std::sync::{Arc, atomic::{AtomicUsize, Ordering::Relaxed}};

pub struct VideoRecorder {
    pub max_buffer_size_in_bytes: usize,
    pub process_function: Box<dyn FnMut(crate::VideoFrame)>,
    pub inner: rc::Rc<cell::RefCell<InnerV>>,
}

pub struct InnerV {
    pub recording_texture: crate::Texture,
    pub clear_color: Option<crate::ClearColor>,
    pub cleared_this_frame: bool,

    pub buffer_size_in_bytes: Arc<AtomicUsize>,
    pub video_frames: VecDeque<crate::VideoFrame>,
    pub frame_states: VecDeque<Arc<FrameState>>,

    pub frame_number: usize,
}

type FrameState = AtomicUsize; // 0=dropped, 1=mapping, 2=mapped, 3=failed-to-map

impl VideoRecorder {
    pub fn new(renderer: &crate::Renderer, clear_color: Option<crate::ClearColor>, max_buffer_size_in_bytes: usize, process_function: Box<dyn FnMut(crate::VideoFrame)>) -> Self {
        let size = (renderer.window_size.width, renderer.window_size.height, 1);

        let inner = InnerV {
            recording_texture: create_recording_texture(&renderer.device, size),
            cleared_this_frame: false,
            clear_color,

            buffer_size_in_bytes: Arc::new(AtomicUsize::new(0)),
            video_frames: VecDeque::new(),
            frame_states: VecDeque::new(),

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

        let store = wgpu::StoreOp::Store;
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

        let image_copy = inner.recording_texture.image_copy_texture((margin_x, margin_y, 0));

        let buffer_copy = wgpu::ImageCopyBuffer {
            buffer: image_data.buffer(),
            layout: inner.recording_texture.image_data_layout(video_frame.padded_bytes_per_row as u32, video_frame.height as u32),
        };

        let mut extent = inner.recording_texture.extent();
        extent.width -= 2 * margin_x;
        extent.height -= 2 * margin_y;

        encoder.copy_texture_to_buffer(image_copy, buffer_copy, extent);
    }

    pub fn initiate_buffer_mapping(&mut self) {
        let mut inner = self.inner.borrow_mut();

        for i in 0..inner.video_frames.len() {
            if inner.frame_states.get(i).is_some() { continue; }
            let video_frame = &inner.video_frames[i];

            if let Some(image_data) = &video_frame.image_data {
                let frame_state = Arc::new(AtomicUsize::new(1)); // 1=mapping
                let frame_state_ = Arc::clone(&frame_state);

                image_data.buffer().slice(..).map_async(wgpu::MapMode::Read, move |result| {
                    frame_state_.store(if result.is_ok() { 2 } else { 3 }, Relaxed); // 2=mapped, 3=failed-to-map
                });

                inner.frame_states.push_back(frame_state);
            } else {
                inner.frame_states.push_back(Arc::new(AtomicUsize::new(0))); // 0=dropped
            }
        }
    }

    pub fn process_mapped_buffers(&mut self) {
        let mut inner = self.inner.borrow_mut();

        loop {
            if inner.video_frames.is_empty() { break; }
            let frame_state = inner.frame_states[0].load(Relaxed);

            match frame_state {
                // If the frame was dropped or mapped, call the process function and keep going.
                // Let the process function decide what to do with dropped frames.
                0 | 2 => {
                    let video_frame = inner.video_frames.pop_front().unwrap();
                    inner.frame_states.pop_front().unwrap();
                    (self.process_function)(video_frame);
                }

                // If the buffer is waiting to be mapped then break.
                // Frames must be processed in order.
                1 => break,

                // If buffer mapping failed then panic since this is unexpected.
                _ => panic!("Failed to memory map buffer data for a video frame."),
            }
        }
    }
}

fn create_recording_texture(device: &wgpu::Device, size: (u32, u32, u32)) -> crate::Texture {
    let filter_mode = crate::FilterMode::Nearest; // Not used
    let format = crate::Format::RgbaU8;
    let msaa_samples = 1;
    let renderable = true;
    let copyable = true;
    let with_sampler = false;

    crate::Texture::new(device, size, filter_mode, format, msaa_samples, renderable, copyable, with_sampler)
}
