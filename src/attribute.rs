#[derive(Clone)]
pub struct Attribute {
    pub buffer: crate::Buffer,
    pub inner: wgpu::VertexAttribute,
    pub location: usize,
    pub size: u32,
}

impl Attribute {
    pub fn new(device: &wgpu::Device, location: usize, size: u32) -> Self {
        let usage = wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST;
        let buffer = crate::Buffer::new(device, usage);
        let inner = wgpu_attribute(location as u32, size);

        Self { buffer, inner, location, size }
    }
}

fn wgpu_attribute(shader_location: u32, size: u32) -> wgpu::VertexAttribute {
    let format = match size {
        1 => wgpu::VertexFormat::Float,
        2 => wgpu::VertexFormat::Float2,
        3 => wgpu::VertexFormat::Float3,
        4 => wgpu::VertexFormat::Float4,
        _ => panic!("Unspported attribute size"),
    };

    wgpu::VertexAttribute { offset: 0, shader_location, format }
}
