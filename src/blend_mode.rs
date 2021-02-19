#[derive(Clone, Copy)]
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

    pub fn state(&self, target_format: crate::Format) -> wgpu::ColorTargetState {
        let blend_state = blend_state(self.src_factor, self.dst_factor);

        color_target_state(blend_state, target_format)
    }
}

fn blend_state(src_factor: wgpu::BlendFactor, dst_factor: wgpu::BlendFactor) -> wgpu::BlendState {
    wgpu::BlendState {
        src_factor,
        dst_factor,
        operation: wgpu::BlendOperation::Add,
    }
}

fn color_target_state(blend_state: wgpu::BlendState, target_format: crate::Format) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
        color_blend: blend_state.clone(),
        alpha_blend: blend_state,
        format: target_format.texture_format(),
        write_mask: wgpu::ColorWrite::ALL,
    }
}
