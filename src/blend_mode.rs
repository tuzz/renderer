pub struct BlendMode {
    pub src_factor: wgpu::BlendFactor,
    pub dst_factor: wgpu::BlendFactor,
}

impl BlendMode {
    pub fn additive() -> Self {
        Self { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::One }
    }

    pub fn pre_multiplied_alpha() -> Self {
        Self { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha }
    }

    pub fn descriptor(&self, target_format: crate::Format) -> wgpu::ColorStateDescriptor {
        let blend_descriptor = blend_descriptor(self.src_factor, self.dst_factor);

        color_state_descriptor(blend_descriptor, target_format)
    }
}

fn blend_descriptor(src_factor: wgpu::BlendFactor, dst_factor: wgpu::BlendFactor) -> wgpu::BlendDescriptor {
    wgpu::BlendDescriptor {
        src_factor,
        dst_factor,
        operation: wgpu::BlendOperation::Add,
    }
}

fn color_state_descriptor(blend_descriptor: wgpu::BlendDescriptor, target_format: crate::Format) -> wgpu::ColorStateDescriptor {
    wgpu::ColorStateDescriptor {
        color_blend: blend_descriptor.clone(),
        alpha_blend: blend_descriptor,
        format: target_format.texture_format(),
        write_mask: wgpu::ColorWrite::ALL,
    }
}
