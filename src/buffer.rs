use std::{cell, mem, ops};

pub struct Buffer {
    pub inner: cell::RefCell<InnerB>,
}

pub struct InnerB {
    pub buffer: wgpu::Buffer,
    pub usage: wgpu::BufferUsage,
    pub size: usize,
}

const INITIAL_SIZE: usize = mem::size_of::<f32>() * 1024;

impl Buffer {
    pub fn new(device: &wgpu::Device, usage: wgpu::BufferUsage) -> Self {
        let buffer = create_buffer(device, usage);
        let inner = InnerB { buffer, usage, size: INITIAL_SIZE };

        Self { inner: cell::RefCell::new(inner) }
    }

    pub fn set_data(&self, device: &wgpu::Device, data: &[f32]) -> Option<wgpu::CommandBuffer> {
        let mut inner = self.inner.borrow_mut();

        let bytes = bytemuck::cast_slice(data);
        let usage = inner.usage | wgpu::BufferUsage::COPY_SRC;
        let staging = device.create_buffer_with_data(bytes, usage);

        let data_size = mem::size_of::<f32>() * data.len();

        if data_size > inner.size {
            inner.buffer = staging;
            inner.usage = usage;
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

fn create_command_encoder(device: &wgpu::Device) -> wgpu::CommandEncoder {
    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None })
}

impl ops::Deref for Buffer {
    type Target = wgpu::Buffer;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.inner.try_borrow_unguarded().unwrap().buffer }
    }
}
