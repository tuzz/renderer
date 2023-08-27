#[derive(Clone, Copy, Debug)]
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
        let blend_component = blend_component(self.src_factor, self.dst_factor);

        color_target_state(blend_component, target_format)
    }
}

fn blend_component(src_factor: wgpu::BlendFactor, dst_factor: wgpu::BlendFactor) -> wgpu::BlendComponent {
    wgpu::BlendComponent {
        src_factor,
        dst_factor,
        operation: wgpu::BlendOperation::Add,
    }
}

fn color_target_state(blend_component: wgpu::BlendComponent, target_format: crate::Format) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
        blend: Some(wgpu::BlendState {
            color: blend_component.clone(),
            alpha: blend_component,
        }),
        format: target_format.texture_format(),
        write_mask: wgpu::ColorWrites::ALL,
    }
}
