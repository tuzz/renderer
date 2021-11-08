use std::num;

#[derive(Clone)]
pub struct Uniform {
    pub buffer: crate::Buffer,
}

impl Uniform {
    pub fn new(device: &wgpu::Device) -> Self {
        let usage = wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST;
        let buffer = crate::Buffer::new(device, usage);

        Self { buffer }
    }

    pub fn binding(&self, visibility: &crate::Visibility, id: u32) -> (wgpu::BindGroupEntry, wgpu::BindGroupLayoutEntry) {
        let layout = uniform_binding_layout(id, visibility, &self.buffer);
        let binding = uniform_binding(id, &self.buffer, self.buffer.inner.borrow().size);

        (binding, layout)
    }
}

fn uniform_binding_layout(id: u32, visibility: &crate::Visibility, buffer: &crate::Buffer) -> wgpu::BindGroupLayoutEntry {
    let size = num::NonZeroU64::new(buffer.inner.borrow().size as u64);
    let uniform = wgpu::BufferBindingType::Uniform;

    let ty = wgpu::BindingType::Buffer { ty: uniform, has_dynamic_offset: false, min_binding_size: size };

    wgpu::BindGroupLayoutEntry { binding: id, visibility: visibility.shader_stage(), ty, count: None }
}

fn uniform_binding(id: u32, buffer: &wgpu::Buffer, size: usize) -> wgpu::BindGroupEntry {
    let size = num::NonZeroU64::new(size as u64);
    let binding = wgpu::BufferBinding { buffer, offset: 0, size };

    wgpu::BindGroupEntry { binding: id, resource: wgpu::BindingResource::Buffer(binding) }
}
