#[derive(Clone)]
pub struct Instanced {
    pub buffer: crate::Buffer,
    pub size: u32,
}

impl Instanced {
    pub fn new(device: &wgpu::Device, size: u32) -> Self {
        let usage = wgpu::BufferUsage::STORAGE_READ | wgpu::BufferUsage::COPY_DST;
        let buffer = crate::Buffer::new(device, usage);

        Self { buffer, size }
    }

    pub fn binding(&self, id: u32) -> (wgpu::Binding, wgpu::BindGroupLayoutEntry) {
        let layout = instanced_binding_layout(id);
        let binding = instanced_binding(id, &self.buffer, self.size);

        (binding, layout)
    }
}

fn instanced_binding_layout(id: u32) -> wgpu::BindGroupLayoutEntry {
    let ty = wgpu::BindingType::StorageBuffer { dynamic: false, readonly: true };

    wgpu::BindGroupLayoutEntry { binding: id, visibility: wgpu::ShaderStage::VERTEX, ty }
}

fn instanced_binding(id: u32, buffer: &wgpu::Buffer, size: u32) -> wgpu::Binding {
    let len = (size * std::mem::size_of::<f32>() as u32) as wgpu::BufferAddress;

    wgpu::Binding { binding: id, resource: wgpu::BindingResource::Buffer { buffer, range: 0..len } }
}
