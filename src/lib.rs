#[cfg(feature="shader_compilation")] mod compile;

use futures::executor;
use winit::{dpi, window};

pub struct Renderer {
    window_size: dpi::PhysicalSize<u32>,
    surface: wgpu::Surface,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    swap_chain: wgpu::SwapChain,
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

    pub fn resize(&mut self, new_size: &dpi::PhysicalSize<u32>) {
        self.window_size = *new_size;
        self.swap_chain = create_swap_chain(&new_size, &self.surface, &self.device);
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
    let descriptor = wgpu::SwapChainDescriptor {
        width: window_size.width,
        height: window_size.height,
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT, // Writes to the screen
        format: wgpu::TextureFormat::Bgra8UnormSrgb,  // Guaranteed to be supported
        present_mode: wgpu::PresentMode::Fifo,        // Enable vsync
    };

    device.create_swap_chain(surface, &descriptor)
}
