pub struct RenderPass;

type Clear = Option<crate::ClearColor>;
type Aspect = Option<crate::AspectRatio>;

impl RenderPass {
    pub fn render(device: &wgpu::Device, target: &wgpu::TextureView, pipeline: &crate::Pipeline, clear: Clear, count: (u32, u32), aspect: Aspect) -> wgpu::CommandBuffer {
        let color_attachments = color_attachments(target, clear);
        let descriptor = render_pass_descriptor(&color_attachments);
        let attributes = &pipeline.program.attributes;
        let (instance_count, vertices_per_instance) = count;

        let mut encoder = create_command_encoder(device);
        let mut render_pass = encoder.begin_render_pass(&descriptor);

        render_pass.set_pipeline(&pipeline.inner);

        for (index, bind_group) in pipeline.bind_groups.iter().enumerate() {
            render_pass.set_bind_group(index as u32, bind_group, &[]);
        }

        for (slot, attribute) in attributes.iter().enumerate() {
            render_pass.set_vertex_buffer(slot as u32, &attribute.buffer, 0, 0);
        }

        if let Some(aspect_ratio) = aspect {
            set_viewport(&mut render_pass, aspect_ratio);
        }

        render_pass.draw(0..vertices_per_instance, 0..instance_count);

        drop(render_pass);
        encoder.finish()
    }
}

fn color_attachments(target: &wgpu::TextureView, clear: Clear) -> Vec<wgpu::RenderPassColorAttachmentDescriptor> {
    if let Some(clear_color) = clear {
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

fn set_viewport(render_pass: &mut wgpu::RenderPass, aspect: crate::AspectRatio) {
    let window = aspect.window_size.unwrap();

    let current_aspect = window.width as f32 / window.height as f32;
    let desired_aspect = aspect.width as f32 / aspect.height as f32;

    let mut width = window.width as f32;
    let mut height = window.height as f32;
    let mut margin_x = 0.;
    let mut margin_y = 0.;

    if current_aspect > desired_aspect {
        width = height * desired_aspect;
        margin_x = (window.width as f32 - width) / 2.;
    } else {
        height = width / desired_aspect;
        margin_y = (window.height as f32 - height) / 2.;
    }

    render_pass.set_viewport(margin_x, margin_y, width, height, 0., 1.);
}
