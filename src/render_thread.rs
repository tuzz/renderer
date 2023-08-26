use std::{thread, sync};
use winit::{dpi, window};

pub struct RenderThread {
    fn_sender: Option<crossbeam_channel::Sender<FunctionCall>>,
    rv_receiver: Option<crossbeam_channel::Receiver<ReturnValue>>,
    _thread: thread::JoinHandle<()>,
}

enum FunctionCall {
    ResizeSwapChain { new_size: dpi::PhysicalSize<u32> },
    ResizeTexture { texture: TextureRef, new_size: (u32, u32, u32) },
    StartFrame,
    FinishFrame,
    Flush,
    SetVsync { boolean: bool },
    AdapterInfo,
    Attribute { location: usize, size: u32 },
    Instanced,
    Uniform,
    Texture { width: u32, height: u32, layers: u32, filter_mode: crate::FilterMode, format: crate::Format, renderable: bool, copyable: bool, with_sampler: bool },
    Program { vert: Vec<u8>, frag: Vec<u8>, attributes: Vec<AttributeRef>, instances: Vec<InstancedRef>, uniforms: Vec<(UniformRef, Vis)>, textures: Vec<(TextureRef, Vis)> },
}

type Vis = crate::Visibility;

enum ReturnValue {
    FrameStarted(bool),
    AdapterInfo(wgpu::AdapterInfo),
    AttributeRef(AttributeRef),
    InstancedRef(InstancedRef),
    UniformRef(UniformRef),
    TextureRef(TextureRef),
    ProgramRef(ProgramRef),
}

#[derive(Clone, Copy)] pub struct AttributeRef(usize);
#[derive(Clone, Copy)] pub struct InstancedRef(usize);
#[derive(Clone, Copy)] pub struct UniformRef(usize);
#[derive(Clone, Copy)] pub struct TextureRef(usize);
#[derive(Clone, Copy)] pub struct ProgramRef(usize);
#[derive(Clone, Copy)] pub enum TargetRef { Screen, TextureRef(TextureRef) }

impl RenderThread {
    pub fn new(window: sync::Arc<window::Window>) -> Self {
        let (fn_sender, fn_receiver) = crossbeam_channel::unbounded::<FunctionCall>();
        let (rv_sender, rv_receiver) = crossbeam_channel::bounded::<ReturnValue>(1);

        let _thread = thread::spawn(move || {
            let renderer = crate::Renderer::new(&window);

            let mut attributes: Vec<crate::Attribute> = vec![];
            let mut instances: Vec<crate::Instanced> = vec![];
            let mut uniforms: Vec<crate::Uniform> = vec![];
            let mut textures: Vec<crate::Texture> = vec![];
            let mut programs: Vec<crate::Program> = vec![];

            while let Ok(message) = fn_receiver.recv() {
                match message {
                    FunctionCall::ResizeSwapChain { new_size } => {
                        let _: () = renderer.resize_swap_chain(&new_size);
                    }
                    FunctionCall::ResizeTexture { texture, new_size } => {
                        let _: () = renderer.resize_texture(&mut textures[texture.0], new_size);
                    },
                    FunctionCall::StartFrame => {
                        rv_sender.send(ReturnValue::FrameStarted(renderer.start_frame())).unwrap();
                    },
                    FunctionCall::FinishFrame => {
                        let _: () = renderer.finish_frame();
                    },
                    FunctionCall::Flush => {
                        let _: () = renderer.flush();
                    },
                    FunctionCall::SetVsync { boolean } => {
                        let _: () = renderer.set_vsync(boolean);
                    },
                    FunctionCall::AdapterInfo => {
                        rv_sender.send(ReturnValue::AdapterInfo(renderer.adapter_info())).unwrap();
                    },
                    FunctionCall::Attribute { location, size } => {
                        attributes.push(renderer.attribute(location, size));
                        rv_sender.send(ReturnValue::AttributeRef(AttributeRef(attributes.len()))).unwrap();
                    },
                    FunctionCall::Instanced => {
                        instances.push(renderer.instanced());
                        rv_sender.send(ReturnValue::InstancedRef(InstancedRef(instances.len()))).unwrap();
                    },
                    FunctionCall::Uniform => {
                        uniforms.push(renderer.uniform());
                        rv_sender.send(ReturnValue::UniformRef(UniformRef(uniforms.len()))).unwrap();
                    },
                    FunctionCall::Texture { width, height, layers, filter_mode, format, renderable, copyable, with_sampler } => {
                        textures.push(renderer.texture(width, height, layers, filter_mode, format, renderable, copyable, with_sampler));
                        rv_sender.send(ReturnValue::TextureRef(TextureRef(textures.len()))).unwrap();
                    }
                    FunctionCall::Program { vert, frag, attributes: a, instances: i, uniforms: u, textures: t } => {
                        let attributes = a.into_iter().map(|r| attributes[r.0].clone()).collect::<Vec<_>>();
                        let instances = i.into_iter().map(|r| instances[r.0].clone()).collect::<Vec<_>>();
                        let uniforms = u.into_iter().map(|(r, v)| (uniforms[r.0].clone(), v)).collect::<Vec<_>>();
                        let textures = t.into_iter().map(|(r, v)| (textures[r.0].clone(), v)).collect::<Vec<_>>();

                        programs.push(renderer.program(&vert, &frag, attributes, instances, uniforms, textures));
                        rv_sender.send(ReturnValue::ProgramRef(ProgramRef(programs.len()))).unwrap();
                    }
                }
            }
        });

        Self { fn_sender: Some(fn_sender), rv_receiver: Some(rv_receiver), _thread }
    }

    pub fn resize_swap_chain(&self, new_size: &dpi::PhysicalSize<u32>) {
        let function_call = FunctionCall::ResizeSwapChain { new_size: *new_size };
        self.fn_sender.as_ref().unwrap().send(function_call).unwrap();
    }

    pub fn resize_texture(&self, texture: TextureRef, new_size: (u32, u32, u32)) {
        let function_call = FunctionCall::ResizeTexture { texture, new_size };
        self.fn_sender.as_ref().unwrap().send(function_call).unwrap();
    }

    pub fn start_frame(&self) -> bool {
        let function_call = FunctionCall::StartFrame;
        self.fn_sender.as_ref().unwrap().send(function_call).unwrap();

        let return_value = self.rv_receiver.as_ref().unwrap().recv().unwrap();
        if let ReturnValue::FrameStarted(b) = return_value { b } else { unreachable!() }
    }

    pub fn finish_frame(&self) {
        let function_call = FunctionCall::FinishFrame;
        self.fn_sender.as_ref().unwrap().send(function_call).unwrap();
    }

    pub fn flush(&self) {
        let function_call = FunctionCall::Flush;
        self.fn_sender.as_ref().unwrap().send(function_call).unwrap();
    }

    pub fn set_vsync(&self, boolean: bool) {
        let function_call = FunctionCall::SetVsync { boolean };
        self.fn_sender.as_ref().unwrap().send(function_call).unwrap();
    }

    pub fn adapter_info(&self) -> wgpu::AdapterInfo {
        let function_call = FunctionCall::AdapterInfo;
        self.fn_sender.as_ref().unwrap().send(function_call).unwrap();

        let return_value = self.rv_receiver.as_ref().unwrap().recv().unwrap();
        if let ReturnValue::AdapterInfo(i) = return_value { i } else { unreachable!() }
    }

    pub fn attribute(&self, location: usize, size: u32) -> AttributeRef {
        let function_call = FunctionCall::Attribute { location, size };
        self.fn_sender.as_ref().unwrap().send(function_call).unwrap();

        let return_value = self.rv_receiver.as_ref().unwrap().recv().unwrap();
        if let ReturnValue::AttributeRef(r) = return_value { r } else { unreachable!() }
    }

    pub fn instanced(&self) -> InstancedRef {
        let function_call = FunctionCall::Instanced;
        self.fn_sender.as_ref().unwrap().send(function_call).unwrap();

        let return_value = self.rv_receiver.as_ref().unwrap().recv().unwrap();
        if let ReturnValue::InstancedRef(r) = return_value { r } else { unreachable!() }
    }

    pub fn uniform(&self) -> UniformRef {
        let function_call = FunctionCall::Uniform;
        self.fn_sender.as_ref().unwrap().send(function_call).unwrap();

        let return_value = self.rv_receiver.as_ref().unwrap().recv().unwrap();
        if let ReturnValue::UniformRef(r) = return_value { r } else { unreachable!() }
    }

    pub fn texture(&self, width: u32, height: u32, layers: u32, filter_mode: crate::FilterMode, format: crate::Format, renderable: bool, copyable: bool, with_sampler: bool) -> TextureRef {
        let function_call = FunctionCall::Texture { width, height, layers, filter_mode, format, renderable, copyable, with_sampler };
        self.fn_sender.as_ref().unwrap().send(function_call).unwrap();

        let return_value = self.rv_receiver.as_ref().unwrap().recv().unwrap();
        if let ReturnValue::TextureRef(r) = return_value { r } else { unreachable!() }
    }

    pub fn program(&self, vert: &[u8], frag: &[u8], attributes: Vec<AttributeRef>, instances: Vec<InstancedRef>, uniforms: Vec<(UniformRef, Vis)>, textures: Vec<(TextureRef, Vis)>) -> ProgramRef {
        let vert = vert.iter().copied().collect();
        let frag = frag.iter().copied().collect();

        let function_call = FunctionCall::Program { vert, frag, attributes, instances, uniforms, textures };
        self.fn_sender.as_ref().unwrap().send(function_call).unwrap();

        let return_value = self.rv_receiver.as_ref().unwrap().recv().unwrap();
        if let ReturnValue::ProgramRef(r) = return_value { r } else { unreachable!() }
    }

    pub fn screen_target(&self) -> TargetRef {
        TargetRef::Screen
    }

    pub fn texture_target(texture: TextureRef) -> TargetRef {
        TargetRef::TextureRef(texture)
    }

    pub fn join(&mut self) {
        self.fn_sender.take();
        self.rv_receiver.take();
    }
}
