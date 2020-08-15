use std::{cell, mem};

pub struct Buffer {
    pub inner: cell::RefCell<Inner>,
}

pub struct Inner {
    pub buffer: wgpu::Buffer,
    pub size: usize,
}

const INITIAL_SIZE: usize = mem::size_of::<f32>() * 1024;

impl Buffer {
    pub fn new(device: &wgpu::Device, usage: wgpu::BufferUsage) -> Self {
        let buffer = create_buffer(device, usage);
        let inner = Inner { buffer, size: INITIAL_SIZE };

        Self { inner: cell::RefCell::new(inner) }
    }

    pub fn set_data(&self, device: &wgpu::Device, data: &[f32]) -> Option<wgpu::CommandBuffer> {
        let staging = create_buffer_with_data(device, data);
        let data_size = mem::size_of::<f32>() * data.len();
        let mut inner = self.inner.borrow_mut();

        if data_size > inner.size {
            inner.buffer = staging;
            None
        } else {
            let mut encoder = create_command_encoder(device);

            encoder.copy_buffer_to_buffer(&staging, 0, &inner.buffer, 0, data_size as u64);
            Some(encoder.finish())
        }
    }
}

fn create_buffer(device: &wgpu::Device, usage: wgpu::BufferUsage) -> wgpu::Buffer {
    let descriptor = wgpu::BufferDescriptor { label: None, size: INITIAL_SIZE as u64, usage };

    device.create_buffer(&descriptor)
}

fn create_buffer_with_data(device: &wgpu::Device, data: &[f32]) -> wgpu::Buffer {
    let bytes = bytemuck::cast_slice(data);
    let usage = wgpu::BufferUsage::COPY_SRC;

    device.create_buffer_with_data(bytes, usage)
}

fn create_command_encoder(device: &wgpu::Device) -> wgpu::CommandEncoder {
    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None })
}
