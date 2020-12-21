#[derive(Clone, Copy)]
pub enum FilterMode {
    Linear,
    Nearest,
}

impl FilterMode {
    pub fn to_wgpu(&self) -> wgpu::FilterMode {
        match self {
            Self::Linear => wgpu::FilterMode::Linear,
            Self::Nearest => wgpu::FilterMode::Nearest,
        }
    }

    pub fn is_linear(&self) -> bool {
        match self {
            Self::Linear => true,
            Self::Nearest => false,
        }
    }
}
