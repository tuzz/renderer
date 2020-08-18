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
}
