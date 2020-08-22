#[derive(Clone, Copy)]
pub enum Visibility {
    VertexShader,
    FragmentShader,
}

impl Visibility {
    pub fn shader_stage(&self) -> wgpu::ShaderStage {
        match self {
            Self::VertexShader => wgpu::ShaderStage::VERTEX,
            Self::FragmentShader => wgpu::ShaderStage::FRAGMENT,
        }
    }
}
