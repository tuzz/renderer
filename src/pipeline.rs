pub struct Pipeline {
    pub inner: wgpu::RenderPipeline,
    pub bind_group: wgpu::BindGroup,
    pub program: crate::Program,
    pub blend_mode: crate::BlendMode,
    pub primitive: crate::Primitive,
    pub target: crate::Target,
}

impl Pipeline {
    pub fn new(device: &wgpu::Device, program: crate::Program, blend_mode: crate::BlendMode, primitive: crate::Primitive, target: crate::Target) -> Self {
        let (bind_group, layout) = create_bind_group(device, &program);
        let inner = create_render_pipeline(device, &program, &blend_mode, &primitive, &layout, &target);

        Self { inner, bind_group, program, blend_mode, primitive, target }
    }

    pub fn recreate_on_texture_resize(&mut self, device: &wgpu::Device) {
        let actual = self.program.textures.iter().map(|(t, _)| t.generation);
        let expected = &self.program.generations;

        if actual.clone().zip(expected).all(|(g1, g2)| g1 == *g2) { return; }

        let (bind_group, layout) = create_bind_group(device, &self.program);
        let inner = create_render_pipeline(device, &self.program, &self.blend_mode, &self.primitive, &layout, &self.target);

        self.bind_group = bind_group;
        self.inner = inner;
        self.program.generations = actual.collect();
    }
}

fn create_bind_group(device: &wgpu::Device, program: &crate::Program) -> (wgpu::BindGroup, wgpu::BindGroupLayout) {
    let mut bindings = vec![];
    let mut layouts = vec![];
    let mut binding_id = 0;

    for instanced in &program.instances {
        let (binding, layout) = instanced.binding(binding_id);
        bindings.push(binding); layouts.push(layout); binding_id += 1;
    }

    for (uniform, visibility) in &program.uniforms {
        let (binding, layout) = uniform.binding(visibility, binding_id);
        bindings.push(binding); layouts.push(layout); binding_id += 1;
    }

    for (texture, visibility) in &program.textures {
        let (binding, layout) = texture.texture_binding(visibility, binding_id);
        bindings.push(binding); layouts.push(layout); binding_id += 1;

        let (binding, layout) = texture.sampler_binding(visibility, binding_id);
        bindings.push(binding); layouts.push(layout); binding_id += 1;
    }

    let descriptor = wgpu::BindGroupLayoutDescriptor { bindings: &layouts, label: None };
    let layout = device.create_bind_group_layout(&descriptor);

    let descriptor = wgpu::BindGroupDescriptor { layout: &layout, bindings: &bindings, label: None };
    let bind_group = device.create_bind_group(&descriptor);

    (bind_group, layout)
}

fn create_render_pipeline(device: &wgpu::Device, program: &crate::Program, blend_mode: &crate::BlendMode, primitive: &crate::Primitive, layout: &wgpu::BindGroupLayout, target: &crate::Target) -> wgpu::RenderPipeline {
    let attribute_descriptors = attribute_descriptors(&program.attributes);
    let vertex_buffers = vertex_buffers(&attribute_descriptors);

    let descriptor = wgpu::RenderPipelineDescriptor {
        layout: &create_layout(device, layout),
        vertex_stage: programmable_stage(&program.vertex_shader),
        fragment_stage: Some(programmable_stage(&program.fragment_shader)),
        rasterization_state: None,
        primitive_topology: primitive.topology(),
        color_states: &[blend_mode.descriptor(target.format())],
        depth_stencil_state: None,
        vertex_state: vertex_state(&vertex_buffers),
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
    };

    device.create_render_pipeline(&descriptor)
}

fn create_layout(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> wgpu::PipelineLayout {
    let descriptor = wgpu::PipelineLayoutDescriptor { bind_group_layouts: &[layout] };

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
    wgpu::VertexStateDescriptor { index_format: wgpu::IndexFormat::Uint16, vertex_buffers }
}
