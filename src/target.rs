pub enum Target {
    Screen,
    Texture(usize, Option<crate::Format>),
}

impl Target {
    pub fn format(&self) -> crate::Format {
        match self {
            Self::Screen => crate::Format::BgraU8,
            Self::Texture(_, format) => format.unwrap(),
        }
    }
}
