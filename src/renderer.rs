use crate::*;
use std::{cell, ops};
use futures::executor;
use winit::{dpi, window};

pub struct Renderer {
    pub inner: cell::RefCell<InnerR>,
}

pub struct InnerR {
    pub window_size: dpi::PhysicalSize<u32>,
    pub instance: wgpu::Instance,
    pub surface: wgpu::Surface,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub vsync: bool,
    pub swap_chain: wgpu::SwapChain,
    pub frame: Option<wgpu::SwapChainFrame>,
    pub commands: Vec<wgpu::CommandBuffer>,
}

impl Renderer {
    pub fn new(window: &window::Window) -> Self {
        let window_size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = get_adapter(&instance, &surface);
        let (device, queue) = get_device(&adapter);
        let vsync = true;
        let swap_chain = create_swap_chain(&window_size, &surface, &device, vsync);
        let frame = Some(swap_chain.get_current_frame().unwrap());
        let commands = vec![];
        let inner = InnerR { window_size, instance, surface, adapter, device, queue, vsync, swap_chain, frame, commands };

        Self { inner: cell::RefCell::new(inner) }
    }

    pub fn resize_swap_chain(&self, new_size: &dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 { return; }

        let mut inner = self.inner.borrow_mut();

        inner.window_size = *new_size;
        inner.frame = None;
        inner.swap_chain = create_swap_chain(&new_size, &inner.surface, &inner.device, inner.vsync);
    }

    pub fn resize_texture(&self, texture: &mut crate::Texture, new_size: (u32, u32)) {
        texture.resize(&self.device, new_size);
    }

    pub fn render(&self, pipeline: &crate::Pipeline, clear_color: Option<crate::ClearColor>, viewport: Option<&crate::Viewport>, count: (u32, u32)) {
        self.render_to(&pipeline.targets, pipeline, clear_color, viewport, count);
    }

    // You can render to different targets than those specified when setting up
    // the pipeline but it will crash if the texture formats are different.

    pub fn render_to(&self, targets: &[crate::Target], pipeline: &crate::Pipeline, clear_color: Option<crate::ClearColor>, viewport: Option<&crate::Viewport>, count: (u32, u32)) {
        let targets = targets.iter().filter(|t| {
            if let crate::Target::Screen = t {
                self.start_frame()
            } else {
                true
            }
        }).collect::<Vec<_>>();

        let render_pass = crate::RenderPass::new(&self);
        let cbuffer = render_pass.render(&targets, pipeline, &clear_color, viewport, count);

        self.inner.borrow_mut().commands.push(cbuffer);
    }

    pub fn start_frame(&self) -> bool {
        if self.frame.is_some() { return true; }

        let mut inner = self.inner.borrow_mut();

        if let Ok(frame) = inner.swap_chain.get_current_frame() {
            inner.frame = Some(frame);
            true
        } else {
            false
        }
    }

    pub fn finish_frame(&self) {
        self.flush_commands();
        self.inner.borrow_mut().frame = None;
    }

    pub fn flush_commands(&self) {
        self.queue.submit(self.inner.borrow_mut().commands.drain(..));
    }

    pub fn set_attribute(&self, pipeline: &crate::Pipeline, location: usize, data: &[f32]) {
        let attribute = pipeline.program.attributes.iter().find(|a| a.location == location).unwrap();
        let option = attribute.buffer.set_data(&self.device, data);

        if let Some(cbuffer) = option {
            self.inner.borrow_mut().commands.push(cbuffer);
        }
    }

    pub fn set_instanced(&self, pipeline: &crate::Pipeline, index_tuple: (usize, usize), data: &[f32]) {
        let index = index_tuple.0 * BINDINGS_PER_GROUP + index_tuple.1;

        let instanced = &pipeline.program.instances[index];
        let option = instanced.buffer.set_data(&self.device, data);

        if let Some(cbuffer) = option {
            self.inner.borrow_mut().commands.push(cbuffer);
        }
    }

    pub fn set_uniform(&self, pipeline: &crate::Pipeline, index_tuple: (usize, usize), data: &[f32]) {
        let index = index_tuple.0 * BINDINGS_PER_GROUP + index_tuple.1;
        let relative_index = uniform_index(index, &pipeline.program);

        let (uniform, _) = &pipeline.program.uniforms[relative_index];
        let option = uniform.buffer.set_data(&self.device, data);

        if let Some(cbuffer) = option {
            self.inner.borrow_mut().commands.push(cbuffer);
        }
    }

    pub fn set_texture<T: bytemuck::Pod>(&self, pipeline: &crate::Pipeline, index_tuple: (usize, usize), data: &[T]) {
        self.set_part_of_texture(pipeline, index_tuple, (0, 0), (0, 0), data);
    }

    pub fn set_part_of_texture<T: bytemuck::Pod>(&self, pipeline: &crate::Pipeline, index_tuple: (usize, usize), offset: (u32, u32), size: (u32, u32), data: &[T]) {
        let index = index_tuple.0 * BINDINGS_PER_GROUP + index_tuple.1;
        let relative_index = texture_index(index, &pipeline.program);

        let (texture, _) = &pipeline.program.textures[relative_index];
        texture.set_data(&self.queue, offset, size, data);
    }

    pub fn set_vsync(&self, boolean: bool) {
        let mut inner = self.inner.borrow_mut();

        inner.vsync = boolean;
        inner.frame = None;
        inner.swap_chain = create_swap_chain(&inner.window_size, &inner.surface, &inner.device, inner.vsync);
    }

    pub fn set_msaa_samples(&self, pipeline: &crate::Pipeline, msaa_samples: u32) {
        let window_size = (self.window_size.width, self.window_size.height);

        pipeline.set_msaa_samples(&self.device, window_size, msaa_samples);
    }

    pub fn adapter_info(&self) -> wgpu::AdapterInfo {
        self.adapter.get_info()
    }

    pub fn pipeline(&self, program: crate::Program, blend_mode: crate::BlendMode, primitive: crate::Primitive, msaa_samples: u32, targets: Vec<crate::Target>) -> crate::Pipeline {
        let window_size = (self.window_size.width, self.window_size.height);
        crate::Pipeline::new(&self.device, window_size, program, blend_mode, primitive, msaa_samples, targets)
    }

    pub fn attribute(&self, location: usize, size: u32) -> crate::Attribute {
        crate::Attribute::new(&self.device, location, size)
    }

    pub fn instanced(&self) -> crate::Instanced {
        crate::Instanced::new(&self.device)
    }

    pub fn uniform(&self) -> crate::Uniform {
        crate::Uniform::new(&self.device)
    }

    pub fn texture(&self, width: u32, height: u32, filter_mode: crate::FilterMode, format: crate::Format, renderable: bool, with_sampler: bool) -> crate::Texture {
        crate::Texture::new(&self.device, (width, height), filter_mode, format, 1, renderable, with_sampler)
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

    pub fn red_u8(&self) -> crate::Format {
        crate::Format::RU8
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

    pub fn visible_to_both_shaders(&self) -> crate::Visibility {
        crate::Visibility::BothShaders
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

    pub fn viewport(&self, aspect_x: f32, aspect_y: f32) -> crate::Viewport {
        crate::Viewport::new(aspect_x, aspect_y, self.window_size.width as f32, self.window_size.height as f32)
    }
}

fn get_adapter(instance: &wgpu::Instance, surface: &wgpu::Surface) -> wgpu::Adapter {
    let options = wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(surface)
    };

    let future = instance.request_adapter(&options);

    executor::block_on(future).unwrap()
}

fn get_device(adapter: &wgpu::Adapter) -> (wgpu::Device, wgpu::Queue) {
    let descriptor = wgpu::DeviceDescriptor::default();
    let future = adapter.request_device(&descriptor, None);

    executor::block_on(future).unwrap()
}

fn create_swap_chain(window_size: &dpi::PhysicalSize<u32>, surface: &wgpu::Surface, device: &wgpu::Device, vsync: bool) -> wgpu::SwapChain {
    let format = crate::Target::Screen.format();

    // Mailbox might also be available for low-latency triple buffering, but
    // this consumes more power so isn't great for mobile devices / laptops.
    let present_mode = match vsync {
        true => wgpu::PresentMode::Fifo,
        false => wgpu::PresentMode::Immediate,
    };

    let descriptor = wgpu::SwapChainDescriptor {
        width: window_size.width,
        height: window_size.height,
        usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
        format: format.texture_format(),
        present_mode,
    };

    device.create_swap_chain(surface, &descriptor)
}


fn uniform_index(index: usize, program: &crate::Program) -> usize {
    index - program.instances.len()
}

fn texture_index(index: usize, program: &crate::Program) -> usize {
    let mut remaining = (index - program.instances.len() - program.uniforms.len()) as i32;

    for (i, (texture, _)) in program.textures.iter().enumerate() {
        if remaining == 0 { return i; }

        remaining -= 1;

        if texture.sampler.is_some() {
            remaining -= 1;
        }

        if remaining < 0 {
            panic!("Tried to get a texture but a sampler is in that slot.");
        }
    }

    panic!("Tried to a get a texture but nothing is in that slot.");
}

impl ops::Deref for Renderer {
    type Target = InnerR;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.inner.try_borrow_unguarded().unwrap() }
    }
}
