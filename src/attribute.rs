#[derive(Clone)]
pub struct Attribute {
    pub buffer: crate::Buffer,
    pub inner: wgpu::VertexAttribute,
    pub location: usize,
    pub size: u32,
}

impl Attribute {
    pub fn new(device: &wgpu::Device, location: usize, size: u32) -> Self {
        let usage = wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST;
        let buffer = crate::Buffer::new(device, usage);
        let inner = wgpu_attribute(location as u32, size);

        Self { buffer, inner, location, size }
    }
}

fn wgpu_attribute(shader_location: u32, size: u32) -> wgpu::VertexAttribute {
    let format = match size {
        1 => wgpu::VertexFormat::Float32,
        2 => wgpu::VertexFormat::Float32x2,
        3 => wgpu::VertexFormat::Float32x3,
        4 => wgpu::VertexFormat::Float32x4,
        _ => panic!("Unsupported attribute size"),
    };

    wgpu::VertexAttribute { offset: 0, shader_location, format }
}
