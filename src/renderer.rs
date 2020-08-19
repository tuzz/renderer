use futures::executor;
use winit::{dpi, window};

pub struct Renderer {
    pub window_size: dpi::PhysicalSize<u32>,
    pub surface: wgpu::Surface,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub swap_chain: wgpu::SwapChain,
}

impl Renderer {
    pub fn new(window: &window::Window) -> Self {
        let window_size = window.inner_size();
        let surface = wgpu::Surface::create(window);
        let adapter = get_adapter(&surface);
        let (device, queue) = get_device(&adapter);
        let swap_chain = create_swap_chain(&window_size, &surface, &device);

        Self { window_size, surface, adapter, device, queue, swap_chain }
    }

    pub fn resize_swap_chain(&mut self, new_size: &dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 { return; }

        self.window_size = *new_size;
        self.swap_chain = create_swap_chain(&new_size, &self.surface, &self.device);
    }

    pub fn resize_texture(&self, texture: &mut crate::Texture, new_size: (u32, u32)) {
        texture.resize(&self.device, new_size);
    }

    pub fn render(&mut self, pipeline: &crate::Pipeline, clear_color: Option<crate::ClearColor>, aspect: Option<crate::AspectRatio>, count: (u32, u32)) {
        match &pipeline.target {
            crate::Target::Screen => self.render_to_screen(pipeline, clear_color, aspect, count),
            crate::Target::Texture(texture) => self.render_to_texture(texture, pipeline, clear_color, count),
        }
    }

    // You can render to a different target than was specified when setting up
    // the pipeline but it might crash(?) if the texture format is different.

    pub fn render_to_screen(&mut self, pipeline: &crate::Pipeline, clear_color: Option<crate::ClearColor>, mut aspect: Option<crate::AspectRatio>, count: (u32, u32)) {
        if let Some(aspect_ratio) = &mut aspect {
            aspect_ratio.window_size = Some(self.window_size);
        }

        let frame = self.swap_chain.get_next_texture().unwrap();
        let commands = crate::RenderPass::render(&self.device, &frame.view, pipeline, clear_color, aspect, count);

        self.queue.submit(&[commands]);
    }

    pub fn render_to_texture(&mut self, texture: &crate::Texture, pipeline: &crate::Pipeline, clear_color: Option<crate::ClearColor>, count: (u32, u32)) {
        let commands = crate::RenderPass::render(&self.device, &texture.view, pipeline, clear_color, None, count);

        self.queue.submit(&[commands]);
    }

    pub fn set_attribute(&self, pipeline: &crate::Pipeline, location: usize, data: &[f32]) {
        let attribute = pipeline.program.attributes.iter().find(|a| a.location == location).unwrap();
        let option = attribute.buffer.set_data(&self.device, data);

        if let Some(commands) = option {
            self.queue.submit(&[commands]);
        }
    }

    pub fn set_instanced(&self, pipeline: &crate::Pipeline, index: usize, data: &[f32]) {
        let instanced = &pipeline.program.instances[index];
        let option = instanced.buffer.set_data(&self.device, data);

        if let Some(commands) = option {
            self.queue.submit(&[commands]);
        }
    }

    pub fn set_uniform(&self, pipeline: &crate::Pipeline, index: usize, data: &[f32]) {
        let relative_index = uniform_index(index, &pipeline.program);

        let (uniform, _) = &pipeline.program.uniforms[relative_index];
        let option = uniform.buffer.set_data(&self.device, data);

        if let Some(commands) = option {
            self.queue.submit(&[commands]);
        }
    }

    pub fn set_texture(&self, pipeline: &crate::Pipeline, index: usize, data: &[u8]) {
        let relative_index = texture_index(index, &pipeline.program);

        let (texture, _) = &pipeline.program.textures[relative_index];
        let commands = texture.set_data(&self.device, data);

        self.queue.submit(&[commands]);
    }

    pub fn pipeline(&self, program: crate::Program, blend_mode: crate::BlendMode, primitive: crate::Primitive, target: crate::Target) -> crate::Pipeline {
        crate::Pipeline::new(&self.device, program, blend_mode, primitive, target)
    }

    pub fn attribute(&self, location: usize, size: u32) -> crate::Attribute {
        crate::Attribute::new(&self.device, location, size)
    }

    pub fn instanced(&self, size: u32) -> crate::Instanced {
        crate::Instanced::new(&self.device, size)
    }

    pub fn uniform(&self, size: u32) -> crate::Uniform {
        crate::Uniform::new(&self.device, size)
    }

    pub fn texture(&self, width: u32, height: u32, filter_mode: crate::FilterMode, format: crate::Format, renderable: bool) -> crate::Texture {
        crate::Texture::new(&self.device, (width, height), filter_mode, format, renderable)
    }

    pub fn program(&self, vert: &[u8], frag: &[u8], attributes: crate::Attributes, instances: crate::Instances, uniforms: crate::Uniforms, textures: crate::Textures) -> crate::Program {
        crate::Program::new(&self.device, vert, frag, attributes, instances, uniforms, textures)
    }

    pub fn screen_target(&self) -> crate::Target {
        crate::Target::Screen
    }

    pub fn texture_target(&self, texture: crate::Texture) -> crate::Target {
        crate::Target::Texture(texture)
    }

    pub fn bgra_u8(&self) -> crate::Format {
        crate::Format::BgraU8
    }

    pub fn rgba_u8(&self) -> crate::Format {
        crate::Format::RgbaU8
    }

    pub fn rgba_f16(&self) -> crate::Format {
        crate::Format::RgbaF16
    }

    pub fn rgba_f32(&self) -> crate::Format {
        crate::Format::RgbaF32
    }

    pub fn linear_filtering(&self) -> crate::FilterMode {
        crate::FilterMode::Linear
    }

    pub fn nearest_filtering(&self) -> crate::FilterMode {
        crate::FilterMode::Nearest
    }

    pub fn visible_to_vertex_shader(&self) -> crate::Visibility {
        crate::Visibility::VertexShader
    }

    pub fn visible_to_fragment_shader(&self) -> crate::Visibility {
        crate::Visibility::FragmentShader
    }

    pub fn additive_blend(&self) -> crate::BlendMode {
        crate::BlendMode::additive()
    }

    pub fn pre_multiplied_blend(&self) -> crate::BlendMode {
        crate::BlendMode::pre_multiplied_alpha()
    }

    pub fn triangle_primitive(&self) -> crate::Primitive {
        crate::Primitive::Triangle
    }

    pub fn triangle_strip_primitive(&self) -> crate::Primitive {
        crate::Primitive::TriangleStrip
    }

    pub fn clear_color(&self, red: f32, green: f32, blue: f32, alpha: f32) -> crate::ClearColor {
        crate::ClearColor::new(red, green, blue, alpha)
    }

    pub fn aspect_ratio(&self, width: f32, height: f32) -> crate::AspectRatio {
        crate::AspectRatio::new(width, height)
    }
}

fn get_adapter(surface: &wgpu::Surface) -> wgpu::Adapter {
    let options = wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::Default,
        compatible_surface: Some(surface)
    };

    let future = wgpu::Adapter::request(&options, wgpu::BackendBit::PRIMARY);

    executor::block_on(future).unwrap()
}

fn get_device(adapter: &wgpu::Adapter) -> (wgpu::Device, wgpu::Queue) {
    let descriptor = wgpu::DeviceDescriptor::default();
    let future = adapter.request_device(&descriptor);

    executor::block_on(future)
}

fn create_swap_chain(window_size: &dpi::PhysicalSize<u32>, surface: &wgpu::Surface, device: &wgpu::Device) -> wgpu::SwapChain {
    let format = crate::Target::Screen.format();

    let descriptor = wgpu::SwapChainDescriptor {
        width: window_size.width,
        height: window_size.height,
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT, // Writes to the screen
        format: format.texture_format(),              // Guaranteed to be supported
        present_mode: wgpu::PresentMode::Fifo,        // Enable vsync
    };

    device.create_swap_chain(surface, &descriptor)
}


fn uniform_index(index: usize, program: &crate::Program) -> usize {
    index - program.instances.len()
}

fn texture_index(index: usize, program: &crate::Program) -> usize {
    index - program.instances.len() - program.uniforms.len()
}
