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
    pub targets: Vec<crate::Target>,
}

// At time of writing, wgpu limits the number of bind group sets to 8 and the
// number of bindings per group to 4, so chunk the bindings into 4s.
pub const BINDINGS_PER_GROUP: usize = 4;

impl Pipeline {
    pub fn new(device: &wgpu::Device, program: crate::Program, blend_mode: crate::BlendMode, primitive: crate::Primitive, targets: Vec<crate::Target>) -> Self {
        let (bind_groups, layouts) = create_bind_groups(device, &program);
        let pipeline = create_render_pipeline(device, &program, &blend_mode, &primitive, &layouts, &targets);
        let inner = InnerP { pipeline, bind_groups, program, blend_mode, primitive, targets };

        Self { inner: cell::RefCell::new(inner) }
    }

    pub fn recreate_on_buffer_or_texture_resize(&self, device: &wgpu::Device) {
        let actual = self.program.latest_generations();
        let expected = &self.program.seen_generations;

        if actual.zip(expected).all(|(g1, g2)| g1 == *g2) { return; }
        let actual = self.program.latest_generations().collect();

        let (bind_groups, layouts) = create_bind_groups(device, &self.program);
        let pipeline = create_render_pipeline(device, &self.program, &self.blend_mode, &self.primitive, &layouts, &self.targets);

        let mut inner = self.inner.borrow_mut();
        inner.bind_groups = bind_groups;
        inner.pipeline = pipeline;
        inner.program.seen_generations = actual;
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

        let (entry, layout) = texture.sampler_binding(visibility, *binding_id);
        entries.push(entry); layouts.push(layout); next(binding_id);
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

fn create_render_pipeline(device: &wgpu::Device, program: &crate::Program, blend_mode: &crate::BlendMode, primitive: &crate::Primitive, layouts: &[wgpu::BindGroupLayout], targets: &[crate::Target]) -> wgpu::RenderPipeline {
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
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
        label: None,
    };

    device.create_render_pipeline(&descriptor)
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
