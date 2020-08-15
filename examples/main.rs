use winit::{event, event_loop, window};

fn main() {
    //renderer::Compiler::compile_shaders("src/shaders");

    let event_loop = event_loop::EventLoop::new();
    let window = window::WindowBuilder::new().build(&event_loop).unwrap();
    let mut renderer = renderer::Renderer::new(&window);

    let vert = include_bytes!("../src/shaders/hello.vert.spirv");
    let frag = include_bytes!("../src/shaders/hello.frag.spirv");

    let a_position = renderer.attribute(0, 2);
    let program = renderer.program(vert, frag, vec![a_position]);
    let blend_mode = renderer.pre_multiplied_blend();
    let pipeline = renderer.pipeline(program, blend_mode);
    let clear_color = renderer.clear_color(0., 0., 0., 0.);

    event_loop.run(move |event, _, control_flow| {
        match event {
            event::Event::RedrawRequested(_) => {
                renderer.render(&pipeline, Some(clear_color), (1, 3));
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
