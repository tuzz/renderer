#[derive(Clone)]
pub enum Target {
    Screen,
    Texture(crate::Texture),
}

impl Target {
    pub fn format(&self) -> crate::Format {
        match self {
            Self::Screen => crate::Format::BgraU8,
            Self::Texture(t) => t.format,
        }
    }

    pub fn view<'a>(&'a self, renderer: &'a crate::Renderer) -> &'a wgpu::TextureView {
        match self {
            crate::Target::Screen => &renderer.frame.as_ref().unwrap().output.view,
            crate::Target::Texture(t) => &t.view,
        }
    }
}
