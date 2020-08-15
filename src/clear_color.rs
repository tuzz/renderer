#[derive(Clone, Copy)]
pub struct ClearColor {
    pub inner: wgpu::Color,
}

impl ClearColor {
    pub fn new(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        let inner = wgpu::Color {
            r: red as f64,
            g: green as f64,
            b: blue as f64,
            a: alpha as f64,
        };

        Self { inner }
    }
}
