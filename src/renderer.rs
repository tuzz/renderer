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
        let mut swap_chain = create_swap_chain(&window_size, &surface, &device);
        let frame = Some(swap_chain.get_current_frame().unwrap());
        let commands = vec![];
        let inner = InnerR { window_size, instance, surface, adapter, device, queue, swap_chain, frame, commands };

        Self { inner: cell::RefCell::new(inner) }
    }

    pub fn resize_swap_chain(&self, new_size: &dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 { return; }

        let mut inner = self.inner.borrow_mut();

        inner.window_size = *new_size;
        inner.swap_chain = create_swap_chain(&new_size, &inner.surface, &inner.device);
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
        let targets = targets.iter().map(|target| {
            match target {
                crate::Target::Texture(texture) => &texture.view,
                crate::Target::Screen => {
                    self.start_frame();
                    &self.frame.as_ref().unwrap().output.view
                },
            }
        }).collect::<Vec<_>>();

        let cbuffer = crate::RenderPass::render(&self.device, &targets, pipeline, &clear_color, viewport, count);
        self.inner.borrow_mut().commands.push(cbuffer);
    }

    pub fn start_frame(&self) {
        if self.frame.is_some() { return; }

        let mut inner = self.inner.borrow_mut();
        inner.frame = Some(inner.swap_chain.get_current_frame().unwrap());
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

    pub fn pipeline(&self, program: crate::Program, blend_mode: crate::BlendMode, primitive: crate::Primitive, targets: Vec<crate::Target>) -> crate::Pipeline {
        crate::Pipeline::new(&self.device, program, blend_mode, primitive, targets)
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

fn create_swap_chain(window_size: &dpi::PhysicalSize<u32>, surface: &wgpu::Surface, device: &wgpu::Device) -> wgpu::SwapChain {
    let format = crate::Target::Screen.format();

    let descriptor = wgpu::SwapChainDescriptor {
        width: window_size.width,
        height: window_size.height,
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        format: format.texture_format(),
        present_mode: wgpu::PresentMode::Fifo,
    };

    device.create_swap_chain(surface, &descriptor)
}


fn uniform_index(index: usize, program: &crate::Program) -> usize {
    index - program.instances.len()
}

fn texture_index(index: usize, program: &crate::Program) -> usize {
    (index - program.instances.len() - program.uniforms.len()) / 2
}

impl ops::Deref for Renderer {
    type Target = InnerR;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.inner.try_borrow_unguarded().unwrap() }
    }
}
