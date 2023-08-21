use std::{cell, mem, ops, rc};

#[derive(Clone)]
pub struct Buffer {
    pub inner: rc::Rc<cell::RefCell<InnerB>>,
}

pub struct InnerB {
    pub buffer: wgpu::Buffer,
    pub usage: wgpu::BufferUsages,
    pub size: usize,
    pub generation: u32,
}

const INITIAL_SIZE: usize = mem::size_of::<f32>() * 16; // Enough for a mat4 uniform.
const HEADROOM: usize = mem::size_of::<f32>() * 256;

impl Buffer {
    pub fn new(device: &wgpu::Device, usage: wgpu::BufferUsages) -> Self {
        let buffer = create_buffer(device, usage);
        let inner = InnerB { buffer, usage, size: INITIAL_SIZE, generation: 0 };

        Self { inner: rc::Rc::new(cell::RefCell::new(inner)) }
    }

    pub fn set_data(&self, device: &wgpu::Device, queue: &wgpu::Queue, data: &[f32]) {
        let mut inner = self.inner.borrow_mut();
        let bytes = bytemuck::cast_slice(data);

        if bytes.len() > inner.size {
            let (buffer, size) = create_buffer_with_headroom(device, inner.usage, bytes);

            inner.buffer = buffer;
            inner.size = size;
            inner.generation += 1;
        } else {
            queue.write_buffer(&inner.buffer, 0, bytes);
        }
    }

    pub fn generation(&self) -> u32 {
        self.inner.borrow().generation
    }
}

fn create_buffer(device: &wgpu::Device, usage: wgpu::BufferUsages) -> wgpu::Buffer {
    let descriptor = wgpu::BufferDescriptor { label: None, size: INITIAL_SIZE as u64, usage, mapped_at_creation: false };

    device.create_buffer(&descriptor)
}

fn create_buffer_with_headroom(device: &wgpu::Device, usage: wgpu::BufferUsages, bytes: &[u8]) -> (wgpu::Buffer, usize) {
    let buffer_size = (bytes.len() + HEADROOM).next_power_of_two();

    let descriptor = wgpu::BufferDescriptor { label: None, size: buffer_size as u64, usage, mapped_at_creation: true };
    let buffer = device.create_buffer(&descriptor);

    buffer.slice(0..bytes.len() as u64).get_mapped_range_mut().copy_from_slice(bytes);
    buffer.unmap();

    (buffer, buffer_size)
}

impl ops::Deref for Buffer {
    type Target = wgpu::Buffer;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.inner.try_borrow_unguarded().unwrap().buffer }
    }
}
