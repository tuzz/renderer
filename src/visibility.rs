#[derive(Clone, Copy)]
pub enum Visibility {
    VertexShader,
    FragmentShader,
    BothShaders,
}

impl Visibility {
    pub fn shader_stage(&self) -> wgpu::ShaderStages {
        match self {
            Self::VertexShader => wgpu::ShaderStages::VERTEX,
            Self::FragmentShader => wgpu::ShaderStages::FRAGMENT,
            Self::BothShaders => wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
        }
    }
}
