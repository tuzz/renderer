use std::num;

#[derive(Clone)]
pub struct Instanced {
    pub buffer: crate::Buffer,
}

impl Instanced {
    pub fn new(device: &wgpu::Device) -> Self {
        let usage = wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::COPY_DST;
        let buffer = crate::Buffer::new(device, usage);

        Self { buffer }
    }

    pub fn binding(&self, id: u32) -> (wgpu::BindGroupEntry, wgpu::BindGroupLayoutEntry) {
        let layout = instanced_binding_layout(id, &self.buffer);
        let binding = instanced_binding(id, &self.buffer);

        (binding, layout)
    }
}

fn instanced_binding_layout(id: u32, buffer: &crate::Buffer) -> wgpu::BindGroupLayoutEntry {
    let size = num::NonZeroU64::new(buffer.inner.borrow().size as u64);

    let ty = wgpu::BindingType::StorageBuffer { dynamic: false, readonly: true, min_binding_size: size };

    wgpu::BindGroupLayoutEntry { binding: id, visibility: wgpu::ShaderStage::VERTEX, ty, count: None }
}

fn instanced_binding(id: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry {
    wgpu::BindGroupEntry { binding: id, resource: wgpu::BindingResource::Buffer(buffer.slice(..)) }
}
