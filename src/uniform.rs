use std::num;

#[derive(Clone)]
pub struct Uniform {
    pub buffer: crate::Buffer,
}

impl Uniform {
    pub fn new(device: &wgpu::Device) -> Self {
        let usage = wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST;
        let buffer = crate::Buffer::new(device, usage);

        Self { buffer }
    }

    pub fn binding(&self, visibility: &crate::Visibility, id: u32) -> (wgpu::BindGroupEntry, wgpu::BindGroupLayoutEntry) {
        let layout = uniform_binding_layout(id, visibility, &self.buffer);
        let binding = uniform_binding(id, &self.buffer);

        (binding, layout)
    }
}

fn uniform_binding_layout(id: u32, visibility: &crate::Visibility, buffer: &crate::Buffer) -> wgpu::BindGroupLayoutEntry {
    let size = num::NonZeroU64::new(buffer.inner.borrow().size as u64);

    let ty = wgpu::BindingType::UniformBuffer { dynamic: false, min_binding_size: size };

    wgpu::BindGroupLayoutEntry { binding: id, visibility: visibility.shader_stage(), ty, count: None }
}

fn uniform_binding(id: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry {
    wgpu::BindGroupEntry { binding: id, resource: wgpu::BindingResource::Buffer(buffer.slice(..)) }
}
