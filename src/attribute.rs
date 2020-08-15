pub struct Attribute {
    buffer: wgpu::Buffer,
    descriptor: wgpu::VertexAttributeDescriptor,
}

impl Attribute {
    pub fn new(device: &wgpu::Device, location: u32, size: u32) -> Self {
        let buffer = create_vertex_buffer(device);
        let descriptor = attribute_descriptor(location, size);

        Self { buffer, descriptor }
    }
}

fn create_vertex_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    let descriptor = wgpu::BufferDescriptor { label: None, size: 0, usage: wgpu::BufferUsage::VERTEX };

    device.create_buffer(&descriptor)
}

fn attribute_descriptor(shader_location: u32, size: u32) -> wgpu::VertexAttributeDescriptor {
    let format = match size {
        1 => wgpu::VertexFormat::Float,
        2 => wgpu::VertexFormat::Float2,
        3 => wgpu::VertexFormat::Float3,
        4 => wgpu::VertexFormat::Float4,
        _ => panic!("Unspported attribute size"),
    };

    wgpu::VertexAttributeDescriptor { offset: 0, shader_location, format }
}
