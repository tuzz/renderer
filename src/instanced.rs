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

    pub fn create_bind_group(&self, device: &wgpu::Device) -> (wgpu::BindGroup, wgpu::BindGroupLayout) {
        let l1 = instanced_binding_layout();
        let descriptor = wgpu::BindGroupLayoutDescriptor { bindings: &[l1], label: None };
        let layout = device.create_bind_group_layout(&descriptor);

        let b1 = instanced_binding(&self.buffer, self.size);
        let descriptor = wgpu::BindGroupDescriptor { layout: &layout, bindings: &[b1], label: None };
        let bind_group = device.create_bind_group(&descriptor);

        (bind_group, layout)
    }
}

fn instanced_binding_layout() -> wgpu::BindGroupLayoutEntry {
    let ty = wgpu::BindingType::StorageBuffer { dynamic: false, readonly: true };

    wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStage::VERTEX, ty }
}

fn instanced_binding(buffer: &wgpu::Buffer, size: u32) -> wgpu::Binding {
    let len = (size * std::mem::size_of::<f32>() as u32) as wgpu::BufferAddress;

    wgpu::Binding { binding: 0, resource: wgpu::BindingResource::Buffer { buffer, range: 0..len } }
}
