#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "bincode", derive(bincode::Encode))]
pub enum Format {
    RU8,
    BgraU8,
    RgbaU8,
    RgbaF16,
    RgbaF32,
}

impl Format {
    pub fn texture_format(&self) -> wgpu::TextureFormat {
        match self {
            Self::RU8 => wgpu::TextureFormat::R8Unorm,
            Self::BgraU8 => wgpu::TextureFormat::Bgra8Unorm,
            Self::RgbaU8 => wgpu::TextureFormat::Rgba8Unorm,
            Self::RgbaF16 => wgpu::TextureFormat::Rgba16Float,
            Self::RgbaF32 => wgpu::TextureFormat::Rgba32Float,
        }
    }

    pub fn sample_type(&self, filterable: bool) -> wgpu::TextureSampleType {
        wgpu::TextureSampleType::Float { filterable }
    }

    pub fn channels(&self) -> u32 {
        match self { Self::RU8 => 1, _ => 4, }
    }

    pub fn bytes_per_channel(&self) -> u32 {
        match self {
            Self::RU8 => 1,
            Self::BgraU8 => 1,
            Self::RgbaU8 => 1,
            Self::RgbaF16 => 2,
            Self::RgbaF32 => 4,
        }
    }

    pub fn bytes_per_texel(&self) -> u32 {
        self.channels() * self.bytes_per_channel()
    }
}
