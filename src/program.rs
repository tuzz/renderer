use std::io;

pub struct Program {
    pub vertex_shader: wgpu::ShaderModule,
    pub fragment_shader: wgpu::ShaderModule,
    pub attributes: Vec<crate::Attribute>,
}

impl Program {
    pub fn new(device: &wgpu::Device, vert: &[u8], frag: &[u8], attributes: Vec<crate::Attribute>) -> Self {
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
