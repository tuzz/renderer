use crate::*;
use shaderc::ShaderKind;
use std::fs;

impl Renderer {
    pub fn compile_shaders(directory: &str) {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            let name = path.as_path().to_str().unwrap();

            if path.is_dir() {
                Self::compile_shaders(name);
            } else if name.ends_with(".vert") {
                Self::compile_shader(name, ShaderKind::Vertex);
            } else if name.ends_with(".frag") {
                Self::compile_shader(name, ShaderKind::Fragment);
            }
        }
    }

    pub fn compile_shader(filename: &str, kind: ShaderKind) {
        let mut compiler = shaderc::Compiler::new().unwrap();

        let source = fs::read_to_string(filename).unwrap();
        let artefact = compiler.compile_into_spirv(&source, kind, filename, "main", None).unwrap();

        let outfile = format!("{}.spirv", filename);
        fs::write(outfile, artefact.as_binary_u8()).unwrap();
    }
}
