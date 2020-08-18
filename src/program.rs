use std::io;

pub struct Program {
    pub vertex_shader: wgpu::ShaderModule,
    pub fragment_shader: wgpu::ShaderModule,
    pub attributes: Attributes,
    pub instances: Instances,
    pub uniforms: Uniforms,
    pub textures: Textures,
    pub generations: Vec<u32>,
}

pub type Attributes = Vec<crate::Attribute>;
pub type Instances = Vec<crate::Instanced>;
pub type Uniforms = Vec<(crate::Uniform, crate::Visibility)>;
pub type Textures = Vec<(crate::Texture, crate::Visibility)>;

impl Program {
    pub fn new(device: &wgpu::Device, vert: &[u8], frag: &[u8], attributes: Attributes, instances: Instances, uniforms: Uniforms, textures: Textures) -> Self {
        let vertex_shader = create_shader_module(device, vert);
        let fragment_shader = create_shader_module(device, frag);
        let generations = textures.iter().map(|(t, _)| t.generation).collect();

        Self { vertex_shader, fragment_shader, attributes, instances, uniforms, textures, generations }
    }
}

fn create_shader_module(device: &wgpu::Device, bytes: &[u8]) -> wgpu::ShaderModule {
    let cursor = io::Cursor::new(bytes);
    let spirv = wgpu::read_spirv(cursor).unwrap();

    device.create_shader_module(&spirv)
}
