use std::{cell, ops};

pub struct Pipeline {
    pub inner: cell::RefCell<InnerP>,
}

pub struct InnerP {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_groups: Vec<wgpu::BindGroup>,
    pub program: crate::Program,
    pub blend_mode: crate::BlendMode,
    pub primitive: crate::Primitive,
    pub msaa_samples: u32,
    pub msaa_texture: Option<crate::Texture>,
    pub targets: Vec<crate::Target>,
}

// At time of writing, wgpu limits the number of bind group sets to 8 and the
// number of bindings per group to 4, so chunk the bindings into 4s.
pub const BINDINGS_PER_GROUP: usize = 4;

impl Pipeline {
    pub fn new(device: &wgpu::Device, window_size: (u32, u32), program: crate::Program, blend_mode: crate::BlendMode, primitive: crate::Primitive, msaa_samples: u32, targets: Vec<crate::Target>) -> Self {
        let (bind_groups, layouts) = create_bind_groups(device, &program);
        let pipeline = create_render_pipeline(device, &program, &blend_mode, &primitive, &layouts, msaa_samples, &targets);
        let msaa_texture = create_msaa_texture(device, window_size, msaa_samples, &targets);
        let inner = InnerP { pipeline, bind_groups, program, blend_mode, primitive, msaa_samples, msaa_texture, targets };

        Self { inner: cell::RefCell::new(inner) }
    }

    pub fn recreate_on_buffer_or_texture_resize(&self, device: &wgpu::Device, window_size: (u32, u32), targets: &[&crate::Target]) {
        resize_msaa_texture(&self, device, window_size, targets);

        let actual = self.program.latest_generations();
        let expected = &self.program.seen_generations;

        if actual.zip(expected).all(|(g1, g2)| g1 == *g2) { return; }
        let actual = self.program.latest_generations().collect();

        let (bind_groups, layouts) = create_bind_groups(device, &self.program);
        let pipeline = create_render_pipeline(device, &self.program, &self.blend_mode, &self.primitive, &layouts, self.msaa_samples, &self.targets);

        let mut inner = self.inner.borrow_mut();
        inner.bind_groups = bind_groups;
        inner.pipeline = pipeline;
        inner.program.seen_generations = actual;
    }

    pub fn set_msaa_samples(&self, device: &wgpu::Device, window_size: (u32, u32), msaa_samples: u32) {
        let (bind_groups, layouts) = create_bind_groups(device, &self.program);
        let pipeline = create_render_pipeline(device, &self.program, &self.blend_mode, &self.primitive, &layouts, msaa_samples, &self.targets);
        let msaa_texture = create_msaa_texture(device, window_size, msaa_samples, &self.targets);

        let mut inner = self.inner.borrow_mut();
        inner.bind_groups = bind_groups;
        inner.pipeline = pipeline;
        inner.msaa_samples = msaa_samples;
        inner.msaa_texture = msaa_texture;
    }
}

fn create_bind_groups(device: &wgpu::Device, program: &crate::Program) -> (Vec<wgpu::BindGroup>, Vec<wgpu::BindGroupLayout>) {
    let entries = &mut vec![];
    let layouts = &mut vec![];
    let binding_id = &mut 0;

    for instanced in &program.instances {
        let (entry, layout) = instanced.binding(*binding_id);
        entries.push(entry); layouts.push(layout); next(binding_id);
    }

    for (uniform, visibility) in &program.uniforms {
        let (entry, layout) = uniform.binding(visibility, *binding_id);
        entries.push(entry); layouts.push(layout); next(binding_id);
    }

    for (texture, visibility) in &program.textures {
        let (entry, layout) = texture.texture_binding(visibility, *binding_id);
        entries.push(entry); layouts.push(layout); next(binding_id);

        if texture.sampler.is_some() {
            let (entry, layout) = texture.sampler_binding(visibility, *binding_id);
            entries.push(entry); layouts.push(layout); next(binding_id);
        }
    }

    let wgpu_layouts = layouts.chunks(BINDINGS_PER_GROUP).map(|entries| {
        let descriptor = wgpu::BindGroupLayoutDescriptor { entries, label: None };
        device.create_bind_group_layout(&descriptor)
    }).collect::<Vec<_>>();

    let wgpu_groups = entries.chunks(BINDINGS_PER_GROUP).enumerate().map(|(i, entries)| {
        let descriptor = wgpu::BindGroupDescriptor { layout: &wgpu_layouts[i], entries, label: None };
        device.create_bind_group(&descriptor)
    }).collect();

    (wgpu_groups, wgpu_layouts)
}

fn next(binding_id: &mut u32) {
    *binding_id += 1;
    *binding_id %= BINDINGS_PER_GROUP as u32;
}

fn create_render_pipeline(device: &wgpu::Device, program: &crate::Program, blend_mode: &crate::BlendMode, primitive: &crate::Primitive, layouts: &[wgpu::BindGroupLayout], msaa_samples: u32, targets: &[crate::Target]) -> wgpu::RenderPipeline {
    let attribute_descriptors = attribute_descriptors(&program.attributes);
    let vertex_buffers = vertex_buffers(&attribute_descriptors);
    let color_states = targets.iter().map(|t| blend_mode.descriptor(t.format())).collect::<Vec<_>>();
    let layout = create_layout(device, layouts);

    let descriptor = wgpu::RenderPipelineDescriptor {
        layout: Some(&layout),
        vertex_stage: programmable_stage(&program.vertex_shader),
        fragment_stage: Some(programmable_stage(&program.fragment_shader)),
        rasterization_state: None,
        primitive_topology: primitive.topology(),
        color_states: &color_states,
        depth_stencil_state: None,
        vertex_state: vertex_state(&vertex_buffers),
        sample_count: msaa_samples,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
        label: None,
    };

    device.create_render_pipeline(&descriptor)
}

fn create_msaa_texture(device: &wgpu::Device, window_size: (u32, u32), msaa_samples: u32, targets: &[crate::Target]) -> Option<crate::Texture> {
    if msaa_samples == 1 { return None; }

    // If there are multiple render targets, configure the MSAA texture based on the first one.
    let target = &targets[0];

    let size = target.size(window_size);
    let filter_mode = crate::FilterMode::Nearest; // Not used
    let format = target.format();
    let renderable = true;
    let with_sampler = false;

    Some(crate::Texture::new(device, size, filter_mode, format, msaa_samples, renderable, with_sampler))
}

fn resize_msaa_texture(pipeline: &Pipeline, device: &wgpu::Device, window_size: (u32, u32), targets: &[&crate::Target]) {
    if pipeline.msaa_samples == 1 { return; }

    let target = &targets[0];
    let new_size = target.size(window_size);

    let mut inner = pipeline.inner.borrow_mut();
    let msaa_texture = inner.msaa_texture.as_mut().unwrap();

    msaa_texture.resize(device, new_size);
}

fn create_layout(device: &wgpu::Device, layouts: &[wgpu::BindGroupLayout]) -> wgpu::PipelineLayout {
    let layouts = layouts.iter().collect::<Vec<_>>();

    let descriptor = wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &layouts, push_constant_ranges: &[] };

    device.create_pipeline_layout(&descriptor)
}

fn programmable_stage(module: &wgpu::ShaderModule) -> wgpu::ProgrammableStageDescriptor {
    wgpu::ProgrammableStageDescriptor { module, entry_point: "main" }
}

type DescriptorsAndSize = (Vec<wgpu::VertexAttributeDescriptor>, u32);

fn attribute_descriptors(attributes: &[crate::Attribute]) -> Vec<DescriptorsAndSize> {
    attributes.iter().map(|a| (vec![a.descriptor.clone()], a.size)).collect::<Vec<_>>()
}

fn vertex_buffers(slice: &[DescriptorsAndSize]) -> Vec<wgpu::VertexBufferDescriptor> {
    slice.iter().map(|(descriptors, size)| {
        let stride = std::mem::size_of::<f32>() * *size as usize;

        wgpu::VertexBufferDescriptor {
          stride: stride as wgpu::BufferAddress,
          step_mode: wgpu::InputStepMode::Vertex,
          attributes: descriptors,
      }
    }).collect::<Vec<_>>()
}

fn vertex_state<'a>(vertex_buffers: &'a [wgpu::VertexBufferDescriptor]) -> wgpu::VertexStateDescriptor<'a> {
    wgpu::VertexStateDescriptor { index_format: None, vertex_buffers }
}

impl ops::Deref for Pipeline {
    type Target = InnerP;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.inner.try_borrow_unguarded().unwrap() }
    }
}
