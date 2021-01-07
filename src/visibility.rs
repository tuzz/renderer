#[derive(Clone, Copy)]
pub enum Visibility {
    VertexShader,
    FragmentShader,
    BothShaders,
}

impl Visibility {
    pub fn shader_stage(&self) -> wgpu::ShaderStage {
        match self {
            Self::VertexShader => wgpu::ShaderStage::VERTEX,
            Self::FragmentShader => wgpu::ShaderStage::FRAGMENT,
            Self::BothShaders => wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
        }
    }
}
