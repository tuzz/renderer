#[derive(Clone, PartialEq)]
pub struct Viewport {
    pub width: f32,
    pub height: f32,
    pub margin_x: f32,
    pub margin_y: f32,
}

impl Viewport {
    pub fn new(aspect_x: f32, aspect_y: f32, max_width: f32, max_height: f32) -> Self {
        let current_aspect = max_width / max_height;
        let desired_aspect = aspect_x / aspect_y;

        let mut width = max_width as f32;
        let mut height = max_height as f32;
        let mut margin_x = 0.;
        let mut margin_y = 0.;

        if current_aspect > desired_aspect {
            width = height * desired_aspect;
            margin_x = (max_width as f32 - width) / 2.;
        } else {
            height = width / desired_aspect;
            margin_y = (max_height as f32 - height) / 2.;
        }

        Self { width, height, margin_x, margin_y }
    }
}
