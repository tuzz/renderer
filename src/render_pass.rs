pub struct RenderPass<'a> {
    renderer: &'a crate::Renderer,
}

type Clear = Option<crate::ClearColor>;
type View<'a> = Option<&'a crate::Viewport>;

impl<'a> RenderPass<'a> {
    pub fn new(renderer: &'a crate::Renderer) -> Self {
        Self { renderer }
    }

    pub fn render(&self, targets: &[&crate::Target], pipeline: &crate::Pipeline, clear: &Clear, viewport: View, count: (u32, u32)) -> wgpu::CommandBuffer {
        pipeline.recreate_on_buffer_or_texture_resize(&self.renderer.device, self.window_size(), targets);

        let color_attachments = self.color_attachments(targets, pipeline, clear);
        let descriptor = render_pass_descriptor(&color_attachments);
        let attributes = &pipeline.program.attributes;
        let (instance_count, vertices_per_instance) = count;

        let mut encoder = create_command_encoder(&self.renderer.device);
        if targets.is_empty() { return encoder.finish(); }

        let mut render_pass = encoder.begin_render_pass(&descriptor);
        render_pass.set_pipeline(&pipeline.pipeline);

        for (i, bind_group) in pipeline.bind_groups.iter().enumerate() {
            render_pass.set_bind_group(i as u32, bind_group, &[]);
        }

        for (slot, attribute) in attributes.iter().enumerate() {
            render_pass.set_vertex_buffer(slot as u32, attribute.buffer.slice(..));
        }

        if let Some(v) = viewport {
            render_pass.set_viewport(v.margin_x, v.margin_y, v.width, v.height, 0., 1.);
        }

        render_pass.draw(0..vertices_per_instance, 0..instance_count);
        drop(render_pass);

        if let Some(stream) = &pipeline.stream {
            let texture = pipeline.screen_texture.as_ref().unwrap();

            encoder.copy_texture_to_buffer(
                texture.texture_copy_view((0, 0)),
                stream.buffer_copy_view_for_next_frame(),
                texture.extent(),
            );
        };

        encoder.finish()
    }

    fn window_size(&self) -> (u32, u32) {
        (self.renderer.window_size.width, self.renderer.window_size.height)
    }

    fn color_attachments(&self, targets: &'a [&crate::Target], pipeline: &'a crate::Pipeline, clear: &Clear) -> Vec<wgpu::RenderPassColorAttachmentDescriptor<'a>> {
        targets.iter().map(|t| self.color_attachment(t, pipeline, clear)).collect()
    }

    fn color_attachment(&self, target: &'a crate::Target, pipeline: &'a crate::Pipeline, clear: &Clear) -> wgpu::RenderPassColorAttachmentDescriptor<'a> {
        let load = match clear { Some(c) => wgpu::LoadOp::Clear(c.inner), _ => wgpu::LoadOp::Load };
        let store = true;
        let ops = wgpu::Operations { load, store };

        let output = target.view(&self.renderer);

        let (attachment, resolve_target) = match &pipeline.screen_texture {
            Some(t) => (&t.view, Some(output)),
            None => (output, None),
        };

        wgpu::RenderPassColorAttachmentDescriptor { attachment, resolve_target, ops }
    }
}

fn render_pass_descriptor<'a>(color_attachments: &'a [wgpu::RenderPassColorAttachmentDescriptor]) -> wgpu::RenderPassDescriptor<'a, 'a> {
    wgpu::RenderPassDescriptor { label: None, depth_stencil_attachment: None, color_attachments }
}

fn create_command_encoder(device: &wgpu::Device) -> wgpu::CommandEncoder {
    let descriptor = wgpu::CommandEncoderDescriptor { label: None };

    device.create_command_encoder(&descriptor)
}
