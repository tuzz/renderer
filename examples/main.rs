use winit::{event, event_loop, window};

fn main() {
    let event_loop = event_loop::EventLoop::new();
    let window = window::WindowBuilder::new().build(&event_loop).unwrap();
    let mut renderer = renderer::Renderer::new(&window);

    event_loop.run(move |event, _, control_flow| {
        match event {
            event::Event::RedrawRequested(_) => {
                // TODO
            },
            event::Event::MainEventsCleared => {
                window.request_redraw();
            },
            event::Event::WindowEvent { event, .. } => match event {
                event::WindowEvent::Resized(size) => {
                    renderer.resize(&size);
                },
                event::WindowEvent::ScaleFactorChanged { new_inner_size: size, .. } => {
                    renderer.resize(size);
                }
                event::WindowEvent::CloseRequested => {
                    *control_flow = event_loop::ControlFlow::Exit;
                },
                _ => {},
            },
            _ => {},
        }
    });
}
