#[derive(Debug)]
pub enum Format {
    BgraU8,
    RgbaU8,
    RgbaF16,
    RgbaF32,
}

impl Format {
    pub fn texture_format(&self) -> wgpu::TextureFormat {
        match self {
            Self::BgraU8 => wgpu::TextureFormat::Bgra8UnormSrgb,
            Self::RgbaU8 => wgpu::TextureFormat::Rgba8UnormSrgb,
            Self::RgbaF16 => wgpu::TextureFormat::Rgba16Float,
            Self::RgbaF32 => wgpu::TextureFormat::Rgba32Float,
        }
    }

    pub fn component_type(&self) -> wgpu::TextureComponentType {
        match self {
            Self::BgraU8 | Self::RgbaU8 => wgpu::TextureComponentType::Uint,
            Self::RgbaF16 | Self::RgbaF32 => wgpu::TextureComponentType::Float,
        }
    }
}
