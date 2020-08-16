use winit::dpi;

#[derive(Clone, Copy)]
pub struct AspectRatio {
    pub width: f32,
    pub height: f32,

    pub window_size: Option<dpi::PhysicalSize<u32>>,
}

impl AspectRatio {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height, window_size: None }
    }
}
