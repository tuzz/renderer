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
    pub position_in_recording: RecordingPosition,
    pub targets: Vec<crate::Target>,
    pub window_size: (u32, u32),
    pub seen_generations: Vec<u32>,
}

// We only want to copy the VideoRecorder's texture to a buffer after the last
// pipeline has finished. Otherwise, we'd record all intermediate writes as well.
pub enum RecordingPosition { None, NotLast, Last }

// At time of writing, wgpu limits the number of bind group sets to 8 and the
// number of bindings per group to 4, so chunk the bindings into 4s.
pub const BINDINGS_PER_GROUP: usize = 4;

impl Pipeline {
    pub fn new(device: &wgpu::Device, window_size: (u32, u32), program: crate::Program, blend_mode: crate::BlendMode, primitive: crate::Primitive, msaa_samples: u32, targets: Vec<crate::Target>) -> Self {
        let msaa_texture = if msaa_samples > 1 { Some(create_msaa_texture(device, window_size, &targets, msaa_samples)) } else { None };
        let position_in_recording = RecordingPosition::None;

        let (bind_groups, layouts) = create_bind_groups(device, &program);
        let color_states = create_color_target_states(&targets, &blend_mode, &position_in_recording);
        let pipeline = create_render_pipeline(device, &program, &primitive, &layouts, msaa_samples, &color_states);
        let seen_generations = program.latest_generations().collect();

        let inner = InnerP { pipeline, bind_groups, program, blend_mode, primitive, msaa_samples, msaa_texture, position_in_recording, targets, window_size, seen_generations };

        Self { inner: cell::RefCell::new(inner) }
    }

    pub fn recreate_on_buffer_or_texture_resize(&self, device: &wgpu::Device, window_size: (u32, u32), targets: &[&crate::Target]) {
        resize_msaa_texture(&self, device, window_size, targets);

        let actual = self.program.latest_generations();
        let expected = &self.seen_generations;

        if actual.zip(expected).all(|(g1, g2)| g1 == *g2) { return; }
        let actual = self.program.latest_generations().collect();

        let (bind_groups, layouts) = create_bind_groups(device, &self.program);
        let color_states = create_color_target_states(&self.targets, &self.blend_mode, &self.position_in_recording);
        let pipeline = create_render_pipeline(device, &self.program, &self.primitive, &layouts, self.msaa_samples, &color_states);

        let mut inner = self.inner.borrow_mut();
        inner.bind_groups = bind_groups;
        inner.pipeline = pipeline;
        inner.window_size = window_size;
        inner.seen_generations = actual;
    }

    pub fn set_msaa_samples(&self, device: &wgpu::Device, msaa_samples: u32) {
        let msaa_texture = if msaa_samples > 1 { Some(create_msaa_texture(device, self.window_size, &self.targets, msaa_samples)) } else { None };

        let (bind_groups, layouts) = create_bind_groups(device, &self.program);
        let color_states = create_color_target_states(&self.targets, &self.blend_mode, &self.position_in_recording);
        let pipeline = create_render_pipeline(device, &self.program, &self.primitive, &layouts, msaa_samples, &color_states);

        let mut inner = self.inner.borrow_mut();
        inner.msaa_samples = msaa_samples;
        inner.msaa_texture = msaa_texture;
        inner.bind_groups = bind_groups;
        inner.pipeline = pipeline;
    }

    pub fn set_stream_position(&self, device: &wgpu::Device, position_in_recording: RecordingPosition) {
        let (bind_groups, layouts) = create_bind_groups(device, &self.program);
        let color_states = create_color_target_states(&self.targets, &self.blend_mode, &position_in_recording);
        let pipeline = create_render_pipeline(device, &self.program, &self.primitive, &layouts, self.msaa_samples, &color_states);

        let mut inner = self.inner.borrow_mut();
        inner.position_in_recording = position_in_recording;
        inner.bind_groups = bind_groups;
        inner.pipeline = pipeline;
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

fn create_color_target_states(targets: &[crate::Target], blend_mode: &crate::BlendMode, stream_position: &RecordingPosition) -> Vec<Option<wgpu::ColorTargetState>> {
    let mut color_target_states = targets.iter().map(|t| Some(blend_mode.state(t.format()))).collect::<Vec<_>>();

    match stream_position {
        RecordingPosition::None => {},
        _ => color_target_states.push(Some(blend_mode.state(crate::Format::RgbaU8))),
    }

    color_target_states
}

fn create_render_pipeline(device: &wgpu::Device, program: &crate::Program, primitive: &crate::Primitive, layouts: &[wgpu::BindGroupLayout], msaa_samples: u32, color_states: &[Option<wgpu::ColorTargetState>]) -> wgpu::RenderPipeline {
    let attribute_descriptors = attribute_descriptors(&program.attributes);
    let vertex_buffers = vertex_buffers(&attribute_descriptors);
    let layout = create_layout(device, layouts);
    let multisample_state = multisample_state(msaa_samples);

    let descriptor = wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&layout),
        vertex: vertex_state(&program.vertex_shader, &vertex_buffers),
        primitive: primitive_state(primitive),
        depth_stencil: None,
        multisample: multisample_state,
        fragment: Some(fragment_state(&program.fragment_shader, color_states)),
        multiview: None,
    };

    device.create_render_pipeline(&descriptor)
}

fn create_msaa_texture(device: &wgpu::Device, window_size: (u32, u32), targets: &[crate::Target], msaa_samples: u32) -> crate::Texture {
    // If there are multiple render targets, configure the MSAA texture based on the first one.
    let target = &targets[0];

    let size = target.size(window_size);
    let filter_mode = crate::FilterMode::Nearest; // Not used
    let format = target.format();
    let renderable = true;
    let copyable = false;
    let with_sampler = false;

    crate::Texture::new(device, size, filter_mode, format, msaa_samples, renderable, copyable, with_sampler)
}

fn resize_msaa_texture(pipeline: &Pipeline, device: &wgpu::Device, window_size: (u32, u32), targets: &[&crate::Target]) {
    let target = &targets[0];
    let new_size = target.size(window_size);

    let mut inner = pipeline.inner.borrow_mut();

    if let Some(texture) = inner.msaa_texture.as_mut() {
        texture.resize(device, new_size);
    }
}

fn create_layout(device: &wgpu::Device, layouts: &[wgpu::BindGroupLayout]) -> wgpu::PipelineLayout {
    let layouts = layouts.iter().collect::<Vec<_>>();

    let descriptor = wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &layouts, push_constant_ranges: &[] };

    device.create_pipeline_layout(&descriptor)
}

fn primitive_state(primitive: &crate::Primitive) -> wgpu::PrimitiveState {
    wgpu::PrimitiveState {
        topology: primitive.topology(),
        strip_index_format: None,
        front_face: wgpu::FrontFace::default(),
        cull_mode: None,
        unclipped_depth: false,
        polygon_mode: wgpu::PolygonMode::default(),
        conservative: false,
    }
}

fn multisample_state(msaa_samples: u32) -> wgpu::MultisampleState {
    wgpu::MultisampleState { count: msaa_samples, mask: !0, alpha_to_coverage_enabled: false }
}

type AttributesAndSize = (Vec<wgpu::VertexAttribute>, u32);

fn attribute_descriptors(attributes: &[crate::Attribute]) -> Vec<AttributesAndSize> {
    attributes.iter().map(|a| (vec![a.inner.clone()], a.size)).collect::<Vec<_>>()
}

fn vertex_buffers(slice: &[AttributesAndSize]) -> Vec<wgpu::VertexBufferLayout> {
    slice.iter().map(|(descriptors, size)| {
        let stride = std::mem::size_of::<f32>() * *size as usize;

        wgpu::VertexBufferLayout {
          array_stride: stride as wgpu::BufferAddress,
          step_mode: wgpu::VertexStepMode::Vertex,
          attributes: descriptors,
      }
    }).collect::<Vec<_>>()
}

fn vertex_state<'a>(module: &'a wgpu::ShaderModule, buffers: &'a [wgpu::VertexBufferLayout]) -> wgpu::VertexState<'a> {
    wgpu::VertexState { module, entry_point: "main", buffers }
}

fn fragment_state<'a>(module: &'a wgpu::ShaderModule, targets: &'a [Option<wgpu::ColorTargetState>]) -> wgpu::FragmentState<'a> {
    wgpu::FragmentState { module, entry_point: "main", targets }
}

impl ops::Deref for Pipeline {
    type Target = InnerP;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.inner.try_borrow_unguarded().unwrap() }
    }
}
