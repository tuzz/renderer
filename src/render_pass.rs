pub struct RenderPass;

type Clear = Option<crate::ClearColor>;
type View<'a> = Option<&'a crate::Viewport>;

impl RenderPass {
    pub fn render(device: &wgpu::Device, targets: &[&wgpu::TextureView], pipeline: &crate::Pipeline, clear: &Clear, viewport: View, count: (u32, u32)) -> wgpu::CommandBuffer {
        pipeline.recreate_on_buffer_or_texture_resize(device);

        let color_attachments = color_attachments(targets, clear);
        let descriptor = render_pass_descriptor(&color_attachments);
        let attributes = &pipeline.program.attributes;
        let (instance_count, vertices_per_instance) = count;

        let mut encoder = create_command_encoder(device);
        let mut render_pass = encoder.begin_render_pass(&descriptor);

        render_pass.set_pipeline(&pipeline.pipeline);
        render_pass.set_bind_group(0, &pipeline.bind_group, &[]);

        for (slot, attribute) in attributes.iter().enumerate() {
            render_pass.set_vertex_buffer(slot as u32, &attribute.buffer, 0, 0);
        }

        if let Some(v) = viewport {
            render_pass.set_viewport(v.margin_x, v.margin_y, v.width, v.height, 0., 1.);
        }

        render_pass.draw(0..vertices_per_instance, 0..instance_count);

        drop(render_pass);
        encoder.finish()
    }
}

fn color_attachments<'a>(targets: &'a [&wgpu::TextureView], clear: &Clear) -> Vec<wgpu::RenderPassColorAttachmentDescriptor<'a>> {
    targets.iter().map(|t| color_attachment(t, clear)).collect()
}

fn color_attachment<'a>(target: &'a wgpu::TextureView, clear: &Clear) -> wgpu::RenderPassColorAttachmentDescriptor<'a> {
    let attachment = target;
    let resolve_target = None;
    let load_op = match clear { Some(_) => wgpu::LoadOp::Clear, _ => wgpu::LoadOp::Load };
    let store_op = wgpu::StoreOp::Store;
    let clear_color = match clear { Some(c) => c.inner, _ => wgpu::Color::TRANSPARENT };

    wgpu::RenderPassColorAttachmentDescriptor { attachment, resolve_target, load_op, store_op, clear_color }
}

fn render_pass_descriptor<'a>(color_attachments: &'a [wgpu::RenderPassColorAttachmentDescriptor]) -> wgpu::RenderPassDescriptor<'a, 'a> {
    wgpu::RenderPassDescriptor { depth_stencil_attachment: None, color_attachments }
}

fn create_command_encoder(device: &wgpu::Device) -> wgpu::CommandEncoder {
    let descriptor = wgpu::CommandEncoderDescriptor { label: None };

    device.create_command_encoder(&descriptor)
}
