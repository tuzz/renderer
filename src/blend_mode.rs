pub struct BlendMode {
    pub descriptor: wgpu::ColorStateDescriptor,
}

impl BlendMode {
    pub fn new(src_factor: wgpu::BlendFactor, dst_factor: wgpu::BlendFactor) -> Self {
        let blend_descriptor = blend_descriptor(src_factor, dst_factor);
        let descriptor = color_state_descriptor(blend_descriptor);

        Self { descriptor }
    }

    pub fn additive() -> Self {
        Self::new(wgpu::BlendFactor::One, wgpu::BlendFactor::One)
    }

    pub fn pre_multiplied_alpha() -> Self {
        Self::new(wgpu::BlendFactor::One, wgpu::BlendFactor::OneMinusSrcAlpha)
    }
}

fn blend_descriptor(src_factor: wgpu::BlendFactor, dst_factor: wgpu::BlendFactor) -> wgpu::BlendDescriptor {
    wgpu::BlendDescriptor {
        src_factor,
        dst_factor,
        operation: wgpu::BlendOperation::Add,
    }
}

fn color_state_descriptor(blend_descriptor: wgpu::BlendDescriptor) -> wgpu::ColorStateDescriptor {
    wgpu::ColorStateDescriptor {
        color_blend: blend_descriptor.clone(),
        alpha_blend: blend_descriptor,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        write_mask: wgpu::ColorWrite::ALL,
    }
}
