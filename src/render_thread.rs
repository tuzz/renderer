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
    Attribute { location: usize, size: u32 },
    Instanced,
    Uniform,
    Texture { width: u32, height: u32, layers: u32, filter_mode: crate::FilterMode, format: crate::Format, renderable: bool, copyable: bool, with_sampler: bool },
}

enum ReturnValue {
    AttributeRef(AttributeRef),
    InstancedRef(InstancedRef),
    UniformRef(UniformRef),
    TextureRef(TextureRef),
}

pub struct AttributeRef(usize);
pub struct InstancedRef(usize);
pub struct UniformRef(usize);
pub struct TextureRef(usize);

impl RenderThread {
    pub fn new(window: sync::Arc<window::Window>) -> Self {
        let (fn_sender, fn_receiver) = crossbeam_channel::unbounded::<FunctionCall>();
        let (rv_sender, rv_receiver) = crossbeam_channel::bounded::<ReturnValue>(1);

        let _thread = thread::spawn(move || {
            let renderer = crate::Renderer::new(&window);

            let mut attributes: Vec<crate::Attribute> = vec![];
            let mut instanced: Vec<crate::Instanced> = vec![];
            let mut uniforms: Vec<crate::Uniform> = vec![];
            let mut textures: Vec<crate::Texture> = vec![];

            while let Ok(message) = fn_receiver.recv() {
                match message {
                    FunctionCall::ResizeSwapChain { new_size } => {
                        let _: () = renderer.resize_swap_chain(&new_size);
                    }
                    FunctionCall::ResizeTexture { texture, new_size } => {
                        let _: () = renderer.resize_texture(&mut textures[texture.0], new_size);
                    },
                    FunctionCall::Attribute { location, size } => {
                        attributes.push(renderer.attribute(location, size));
                        rv_sender.send(ReturnValue::AttributeRef(AttributeRef(attributes.len()))).unwrap();
                    },
                    FunctionCall::Instanced => {
                        instanced.push(renderer.instanced());
                        rv_sender.send(ReturnValue::InstancedRef(InstancedRef(instanced.len()))).unwrap();
                    },
                    FunctionCall::Uniform => {
                        uniforms.push(renderer.uniform());
                        rv_sender.send(ReturnValue::UniformRef(UniformRef(uniforms.len()))).unwrap();
                    },
                    FunctionCall::Texture { width, height, layers, filter_mode, format, renderable, copyable, with_sampler } => {
                        textures.push(renderer.texture(width, height, layers, filter_mode, format, renderable, copyable, with_sampler));
                        rv_sender.send(ReturnValue::TextureRef(TextureRef(textures.len()))).unwrap();
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

    pub fn join(&mut self) {
        self.fn_sender.take();
        self.rv_receiver.take();
    }
}
