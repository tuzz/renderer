use std::mem;

pub struct Attribute {
    pub buffer: crate::Buffer,
    pub descriptor: wgpu::VertexAttributeDescriptor,
    pub location: usize,
    pub size: u32,
}

impl Attribute {
    pub fn new(device: &wgpu::Device, location: usize, size: u32) -> Self {
        let usage = wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST;
        let buffer = crate::Buffer::new(device, usage);
        let descriptor = attribute_descriptor(location as u32, size);

        Self { buffer, descriptor, location, size }
    }
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
