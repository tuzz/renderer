use crate::Attribute;
use std::io;

pub struct Program {
    vertex_shader: wgpu::ShaderModule,
    fragment_shader: wgpu::ShaderModule,
    attributes: Vec<Attribute>,
}

impl Program {
    pub fn new(device: &wgpu::Device, vert: &[u8], frag: &[u8], attributes: Vec<Attribute>) -> Self {
        let vertex_shader = create_shader_module(device, vert);
        let fragment_shader = create_shader_module(device, frag);

        Self { vertex_shader, fragment_shader, attributes }
    }
}

fn create_shader_module(device: &wgpu::Device, bytes: &[u8]) -> wgpu::ShaderModule {
    let cursor = io::Cursor::new(bytes);
    let spirv = wgpu::read_spirv(cursor).unwrap();

    device.create_shader_module(&spirv)
}
