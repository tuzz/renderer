use crate::*;
use std::{cell, ops};
use std::sync::{atomic, Arc};
use futures::executor;
use winit::{dpi, window};

pub struct Renderer<'a> {
    pub inner: cell::RefCell<InnerR<'a>>,
}

pub struct InnerR<'a> {
    pub window_size: dpi::PhysicalSize<u32>,
    pub instance: wgpu::Instance,
    pub surface: wgpu::Surface<'a>,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub vsync: bool,
    pub frame: Option<wgpu::SurfaceTexture>,
    pub frame_view: Option<wgpu::TextureView>,
    pub commands: Vec<wgpu::CommandBuffer>,
    pub recorder: Option<crate::VideoRecorder>,
    pub flushes: atomic::AtomicU64,
}

impl<'a> Renderer<'a> {
    pub fn new(window: Arc<window::Window>) -> Self {
        let (instance, surface) = Self::create_surface(window.clone());
        Self::new_with_surface(window.inner_size(), instance, surface)
    }

    pub fn create_surface(window: Arc<window::Window>) -> (wgpu::Instance, wgpu::Surface<'a>) {
        let instance = get_instance();
        let surface = instance.create_surface(window).unwrap(); // Must be called in main thread.

        (instance, surface)
    }

    pub fn new_with_surface(window_size: dpi::PhysicalSize<u32>, instance: wgpu::Instance, surface: wgpu::Surface<'a>) -> Self {
        let adapter = get_adapter(&instance, &surface);
        let (device, queue) = get_device(&adapter);
        let vsync = true;

        configure_surface(&surface, &device, &window_size, vsync);

        let frame = Some(surface.get_current_texture().unwrap());
        let frame_view = Some(frame.as_ref().unwrap().texture.create_view(&wgpu::TextureViewDescriptor::default()));
        let commands = vec![];
        let recorder = None;
        let flushes = atomic::AtomicU64::new(0);
        let inner = InnerR { window_size, instance, surface, adapter, device, queue, vsync, frame, frame_view, commands, recorder, flushes };

        Self { inner: cell::RefCell::new(inner) }
    }

    pub fn resize_swap_chain(&self, new_size: &dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 { return; }

        let mut inner = self.inner.borrow_mut();

        inner.window_size = *new_size;
        inner.frame = None;
        inner.frame_view = None;

        configure_surface(&inner.surface, &inner.device, &new_size, inner.vsync);
    }

    pub fn resize_texture(&self, texture: &mut crate::Texture, new_size: (u32, u32, u32)) {
        texture.resize(&self.device, new_size);
    }

    pub fn render(&self, pipeline: &crate::Pipeline, clear_color: Option<crate::ClearColor>, viewport: Option<&crate::Viewport>, count: (u32, u32)) {
        self.render_to(&pipeline.targets, pipeline, clear_color, viewport, count);
    }

    // You can render to different targets than those specified when setting up
    // the pipeline but it will crash if the texture formats are different.

    pub fn render_to(&self, targets: &[crate::Target], pipeline: &crate::Pipeline, clear_color: Option<crate::ClearColor>, viewport: Option<&crate::Viewport>, count: (u32, u32)) {
        for target in targets {
            if let crate::Target::Screen = target {
                self._start_frame()
            }
        }

        let render_pass = crate::RenderPass::new(&self);
        let cbuffer = render_pass.render(targets, pipeline, &clear_color, viewport, count);

        self.inner.borrow_mut().commands.push(cbuffer);
    }

    fn _start_frame(&self) {
        if self.frame.is_some() { return; }

        let mut inner = self.inner.borrow_mut();
        let frame = inner.surface.get_current_texture().unwrap();

        inner.frame_view = Some(frame.texture.create_view(&wgpu::TextureViewDescriptor::default()));
        inner.frame = Some(frame);
    }

    pub fn finish_frame(&self) {
        self.flush();

        let mut inner = self.inner.borrow_mut();

        if let Some(recorder) = &mut inner.recorder {
            recorder.initiate_buffer_mapping();
            recorder.process_mapped_buffers();
            recorder.finish_frame();
        }

        if inner.frame.is_none() { return; }

        inner.frame.take().unwrap().present();
        inner.frame_view = None;
    }

    pub fn flush(&self) {
        self.queue.submit(self.inner.borrow_mut().commands.drain(..));
        self.flushes.fetch_add(1, atomic::Ordering::Relaxed);
    }

    pub fn set_attribute(&self, pipeline: &crate::Pipeline, location: usize, data: &[f32]) {
        let attribute = pipeline.program.attributes.iter().find(|a| a.location == location).unwrap();
        let flushes = self.flushes.load(atomic::Ordering::Relaxed);

        attribute.buffer.set_data(&self.device, &self.queue, data, flushes);
    }

    pub fn set_instanced(&self, pipeline: &crate::Pipeline, index_tuple: (usize, usize), data: &[f32]) {
        let index = index_tuple.0 * BINDINGS_PER_GROUP + index_tuple.1;

        let instanced = &pipeline.program.instances[index];
        let flushes = self.flushes.load(atomic::Ordering::Relaxed);

        instanced.buffer.set_data(&self.device, &self.queue, data, flushes);
    }

    pub fn set_uniform(&self, pipeline: &crate::Pipeline, index_tuple: (usize, usize), data: &[f32]) {
        let index = index_tuple.0 * BINDINGS_PER_GROUP + index_tuple.1;
        let relative_index = uniform_index(index, &pipeline.program);

        let (uniform, _) = &pipeline.program.uniforms[relative_index];
        let flushes = self.flushes.load(atomic::Ordering::Relaxed);

        uniform.buffer.set_data(&self.device, &self.queue, data, flushes);
    }

    pub fn set_texture<T: bytemuck::Pod>(&self, pipeline: &crate::Pipeline, index_tuple: (usize, usize), layers_data: &[&[T]]) {
        for (layer, data) in layers_data.iter().enumerate() {
            self.set_part_of_texture(pipeline, index_tuple, (0, 0, layer as u32), (0, 0), data);
        }
    }

    pub fn set_part_of_texture<T: bytemuck::Pod>(&self, pipeline: &crate::Pipeline, index_tuple: (usize, usize), offset: (u32, u32, u32), size: (u32, u32), data: &[T]) {
        let index = index_tuple.0 * BINDINGS_PER_GROUP + index_tuple.1;
        let relative_index = texture_index(index, &pipeline.program);

        let (texture, _) = &pipeline.program.textures[relative_index];
        texture.set_data(&self.queue, offset, size, data);
    }

    pub fn set_vsync(&self, boolean: bool) {
        let mut inner = self.inner.borrow_mut();

        inner.vsync = boolean;
        inner.frame = None;
        inner.frame_view = None;

        configure_surface(&inner.surface, &inner.device, &inner.window_size, boolean);
    }

    pub fn set_msaa_samples(&self, pipeline: &crate::Pipeline, msaa_samples: u32) {
        pipeline.set_msaa_samples(&self.device, msaa_samples);
    }

    pub fn start_recording(&self, pipelines: &[&crate::Pipeline], clear_color: Option<crate::ClearColor>, max_buffer_size_in_megabytes: f32, process_function: Box<dyn FnMut(crate::VideoFrame)>) {
        let max_size_in_bytes = (max_buffer_size_in_megabytes * 1024. * 1024.) as usize;
        let recorder = crate::VideoRecorder::new(&self, clear_color, max_size_in_bytes, process_function);
        self.inner.borrow_mut().recorder = Some(recorder);

        for (i, pipeline) in pipelines.iter().enumerate() {
            let is_last = i == pipelines.len() - 1;
            let position = if is_last { crate::RecordingPosition::Last } else { crate::RecordingPosition::NotLast };
            pipeline.set_stream_position(&self.device, position);
        }
    }

    pub fn stop_recording(&self, pipelines: &[&crate::Pipeline]) {
        self.inner.borrow_mut().recorder = None;

        for pipeline in pipelines {
            let position = crate::RecordingPosition::None;
            pipeline.set_stream_position(&self.device, position);
        }
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

    pub fn texture(&self, width: u32, height: u32, layers: u32, filter_mode: crate::FilterMode, format: crate::Format, renderable: bool, copyable: bool, with_sampler: bool) -> crate::Texture {
        crate::Texture::new(&self.device, (width, height, layers), filter_mode, format, 1, renderable, copyable, with_sampler)
    }

    pub fn program(&self, vert: &[u8], frag: &[u8], attributes: crate::Attributes, instances: crate::Instances, uniforms: crate::Uniforms, textures: crate::Textures) -> crate::Program {
        crate::Program::new(&self.device, vert, frag, attributes, instances, uniforms, textures)
    }

    pub fn viewport(&self, aspect_x: f32, aspect_y: f32) -> crate::Viewport {
        crate::Viewport::new(aspect_x, aspect_y, self.window_size.width as f32, self.window_size.height as f32)
    }

    pub fn screen_target() -> crate::Target {
        crate::Target::Screen
    }

    pub fn texture_target(texture: crate::Texture) -> crate::Target {
        crate::Target::Texture(texture)
    }

    pub fn bgra_u8() -> crate::Format {
        crate::Format::BgraU8
    }

    pub fn red_u8() -> crate::Format {
        crate::Format::RU8
    }

    pub fn rgba_u8() -> crate::Format {
        crate::Format::RgbaU8
    }

    pub fn rgba_f16() -> crate::Format {
        crate::Format::RgbaF16
    }

    pub fn rgba_f32() -> crate::Format {
        crate::Format::RgbaF32
    }

    pub fn linear_filtering() -> crate::FilterMode {
        crate::FilterMode::Linear
    }

    pub fn nearest_filtering() -> crate::FilterMode {
        crate::FilterMode::Nearest
    }

    pub fn visible_to_vertex_shader() -> crate::Visibility {
        crate::Visibility::VertexShader
    }

    pub fn visible_to_fragment_shader() -> crate::Visibility {
        crate::Visibility::FragmentShader
    }

    pub fn visible_to_both_shaders() -> crate::Visibility {
        crate::Visibility::BothShaders
    }

    pub fn additive_blend() -> crate::BlendMode {
        crate::BlendMode::additive()
    }

    pub fn pre_multiplied_blend() -> crate::BlendMode {
        crate::BlendMode::pre_multiplied_alpha()
    }

    pub fn triangle_primitive() -> crate::Primitive {
        crate::Primitive::Triangle
    }

    pub fn triangle_strip_primitive() -> crate::Primitive {
        crate::Primitive::TriangleStrip
    }

    pub fn clear_color(red: f32, green: f32, blue: f32, alpha: f32) -> crate::ClearColor {
        crate::ClearColor::new(red, green, blue, alpha)
    }
}

fn configure_surface(surface: &wgpu::Surface, device: &wgpu::Device, window_size: &dpi::PhysicalSize<u32>, vsync: bool) {
    let format = crate::Target::Screen.format();

    let present_mode = match vsync {
        true => wgpu::PresentMode::AutoVsync,
        false => wgpu::PresentMode::AutoNoVsync,
    };

    surface.configure(device, &wgpu::SurfaceConfiguration {
        width: window_size.width,
        height: window_size.height,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: format.texture_format(),
        view_formats: vec![format.texture_format()],
        present_mode,
        desired_maximum_frame_latency: 2,
        alpha_mode: wgpu::CompositeAlphaMode::Auto, // TODO: set an explicit alpha mode (check supported)
    });
}

fn get_instance() -> wgpu::Instance {
    let descriptor = wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
        flags: wgpu::InstanceFlags::default(),
        gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
    };

    wgpu::Instance::new(descriptor)
}

fn get_adapter(instance: &wgpu::Instance, surface: &wgpu::Surface) -> wgpu::Adapter {
    let options = wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: Some(surface)
    };

    let future = instance.request_adapter(&options);

    executor::block_on(future).unwrap()
}

fn get_device(adapter: &wgpu::Adapter) -> (wgpu::Device, wgpu::Queue) {
    let descriptor = wgpu::DeviceDescriptor {
        label: None,
        required_features: wgpu::Features::VERTEX_WRITABLE_STORAGE,
        required_limits: wgpu::Limits::default(),
    };

    let future = adapter.request_device(&descriptor, None);

    executor::block_on(future).unwrap()
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

impl<'a> ops::Deref for Renderer<'a> {
    type Target = InnerR<'a>;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.inner.try_borrow_unguarded().unwrap() }
    }
}
