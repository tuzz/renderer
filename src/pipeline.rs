pub struct Pipeline {
    pub inner: wgpu::RenderPipeline,
    pub program: crate::Program,
    pub blend_mode: crate::BlendMode,
}

impl Pipeline {
    pub fn new(device: &wgpu::Device, program: crate::Program, blend_mode: crate::BlendMode) -> Self {
        let inner = create_render_pipeline(device, &program, &blend_mode);

        Self { inner, program, blend_mode }
    }
}

fn create_render_pipeline(device: &wgpu::Device, program: &crate::Program, blend_mode: &crate::BlendMode) -> wgpu::RenderPipeline {
    let attribute_descriptors = attribute_descriptors(&program.attributes);
    let vertex_buffers = vertex_buffers(&attribute_descriptors);

    let descriptor = wgpu::RenderPipelineDescriptor {
        layout: &create_layout(device),
        vertex_stage: programmable_stage(&program.vertex_shader),
        fragment_stage: Some(programmable_stage(&program.fragment_shader)),
        rasterization_state: None,
        primitive_topology: wgpu::PrimitiveTopology::TriangleStrip, // TODO
        color_states: &[blend_mode.descriptor.clone()],
        depth_stencil_state: None,
        vertex_state: vertex_state(&vertex_buffers),
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
    };

    device.create_render_pipeline(&descriptor)
}

fn create_layout(device: &wgpu::Device) -> wgpu::PipelineLayout {
    let descriptor = wgpu::PipelineLayoutDescriptor { bind_group_layouts: &[] };

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