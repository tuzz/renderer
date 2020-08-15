pub struct RenderPass;

impl RenderPass {
    pub fn render(device: &wgpu::Device, target: &wgpu::TextureView, pipeline: &crate::Pipeline, clear_color: Option<crate::ClearColor>) -> wgpu::CommandBuffer {
        let color_attachments = color_attachments(target, clear_color);
        let descriptor = render_pass_descriptor(&color_attachments);
        let attributes = &pipeline.program.attributes;

        let mut encoder = create_command_encoder(device);
        let mut render_pass = encoder.begin_render_pass(&descriptor);

        render_pass.set_pipeline(&pipeline.inner);

        for (slot, attribute) in attributes.iter().enumerate() {
            render_pass.set_vertex_buffer(slot as u32, &attribute.buffer, 0, 0);
        }

        drop(render_pass);
        encoder.finish()
    }
}

fn color_attachments(target: &wgpu::TextureView, clear_color: Option<crate::ClearColor>) -> Vec<wgpu::RenderPassColorAttachmentDescriptor> {
    if let Some(clear_color) = clear_color {
        vec![wgpu::RenderPassColorAttachmentDescriptor {
            attachment: target,
            resolve_target: None,
            load_op: wgpu::LoadOp::Clear,
            store_op: wgpu::StoreOp::Store,
            clear_color: clear_color.inner,
        }]
    } else {
        vec![wgpu::RenderPassColorAttachmentDescriptor {
            attachment: target,
            resolve_target: None,
            load_op: wgpu::LoadOp::Load,
            store_op: wgpu::StoreOp::Store,
            clear_color: wgpu::Color::TRANSPARENT,
        }]
    }
}

fn render_pass_descriptor<'a>(color_attachments: &'a [wgpu::RenderPassColorAttachmentDescriptor]) -> wgpu::RenderPassDescriptor<'a, 'a> {
    wgpu::RenderPassDescriptor { depth_stencil_attachment: None, color_attachments }
}

fn create_command_encoder(device: &wgpu::Device) -> wgpu::CommandEncoder {
    let descriptor = wgpu::CommandEncoderDescriptor { label: None };

    device.create_command_encoder(&descriptor)
}
