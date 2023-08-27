#[derive(Clone, Copy, Debug)]
pub enum Primitive {
    Triangle,
    TriangleStrip,
}

impl Primitive {
    pub fn topology(&self) -> wgpu::PrimitiveTopology {
        match self {
            Self::Triangle => wgpu::PrimitiveTopology::TriangleList,
            Self::TriangleStrip => wgpu::PrimitiveTopology::TriangleStrip,
        }
    }
}
