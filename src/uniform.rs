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

    pub fn create_bind_group(&self, device: &wgpu::Device, visibility: &crate::Visibility) -> (wgpu::BindGroup, wgpu::BindGroupLayout) {
        let l1 = uniform_binding_layout(visibility);
        let descriptor = wgpu::BindGroupLayoutDescriptor { bindings: &[l1], label: None };
        let layout = device.create_bind_group_layout(&descriptor);

        let b1 = uniform_binding(&self.buffer, self.size);
        let descriptor = wgpu::BindGroupDescriptor { layout: &layout, bindings: &[b1], label: None };
        let bind_group = device.create_bind_group(&descriptor);

        (bind_group, layout)
    }
}

fn uniform_binding_layout(visibility: &crate::Visibility) -> wgpu::BindGroupLayoutEntry {
    let ty = wgpu::BindingType::UniformBuffer { dynamic: false };

    wgpu::BindGroupLayoutEntry { binding: 0, visibility: visibility.shader_stage(), ty }
}

fn uniform_binding(buffer: &wgpu::Buffer, size: u32) -> wgpu::Binding {
    let range = 0..size as wgpu::BufferAddress;

    wgpu::Binding { binding: 0, resource: wgpu::BindingResource::Buffer { buffer, range } }
}
