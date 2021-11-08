pub struct Program {
    pub vertex_shader: wgpu::ShaderModule,
    pub fragment_shader: wgpu::ShaderModule,
    pub attributes: Attributes,
    pub instances: Instances,
    pub uniforms: Uniforms,
    pub textures: Textures,
    pub seen_generations: Vec<u32>,
}

pub type Attributes = Vec<crate::Attribute>;
pub type Instances = Vec<crate::Instanced>;
pub type Uniforms = Vec<(crate::Uniform, crate::Visibility)>;
pub type Textures = Vec<(crate::Texture, crate::Visibility)>;

impl Program {
    pub fn new(device: &wgpu::Device, vert: &[u8], frag: &[u8], attributes: Attributes, instances: Instances, uniforms: Uniforms, textures: Textures) -> Self {
        let mut program = Self {
            vertex_shader: create_shader_module(device, vert),
            fragment_shader: create_shader_module(device, frag),
            attributes, instances, uniforms, textures,
            seen_generations: vec![],
        };

        program.seen_generations = program.latest_generations().collect();
        program
    }

    pub fn latest_generations(&self) -> impl Iterator<Item=u32> + '_ {
        let g1 = self.attributes.iter().map(|a| a.buffer.generation());
        let g2 = self.instances.iter().map(|i| i.buffer.generation());
        let g3 = self.uniforms.iter().map(|(u, _)| u.buffer.generation());
        let g4 = self.textures.iter().map(|(t, _)| t.generation);

        g1.chain(g2).chain(g3).chain(g4)
    }
}

fn create_shader_module(device: &wgpu::Device, bytes: &[u8]) -> wgpu::ShaderModule {
    let spirv = wgpu::util::make_spirv(bytes);
    let descriptor = wgpu::ShaderModuleDescriptor { label: None, source: spirv };

    device.create_shader_module(&descriptor)
}
