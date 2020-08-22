#[derive(Clone)]
pub struct Uniform {
    pub buffer: crate::Buffer,
    pub size: u32,
}

impl Uniform {
    pub fn new(device: &wgpu::Device, size: u32) -> Self {
        let usage = wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST;
        let buffer = crate::Buffer::new(device, usage);

        Self { buffer, size }
    }

    pub fn binding(&self, visibility: &crate::Visibility, id: u32) -> (wgpu::Binding, wgpu::BindGroupLayoutEntry) {
        let layout = uniform_binding_layout(id, visibility);
        let binding = uniform_binding(id, &self.buffer, self.size);

        (binding, layout)
    }
}

fn uniform_binding_layout(id: u32, visibility: &crate::Visibility) -> wgpu::BindGroupLayoutEntry {
    let ty = wgpu::BindingType::UniformBuffer { dynamic: false };

    wgpu::BindGroupLayoutEntry { binding: id, visibility: visibility.shader_stage(), ty }
}

fn uniform_binding(id: u32, buffer: &wgpu::Buffer, size: u32) -> wgpu::Binding {
    let len = (size * std::mem::size_of::<f32>() as u32) as wgpu::BufferAddress;

    wgpu::Binding { binding: id, resource: wgpu::BindingResource::Buffer { buffer, range: 0..len } }
}
