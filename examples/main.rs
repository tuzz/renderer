use winit::{event, event_loop, window};

const A_POSITION: usize = 0;
const A_COLOR: usize = 1;

fn main() {
    renderer::Compiler::compile_shaders("src/shaders");

    let event_loop = event_loop::EventLoop::new();
    let window = window::WindowBuilder::new().build(&event_loop).unwrap();
    let mut renderer = renderer::Renderer::new(&window);

    let vert = include_bytes!("../src/shaders/hello.vert.spirv");
    let frag = include_bytes!("../src/shaders/hello.frag.spirv");

    let a_position = renderer.attribute(A_POSITION, 2);
    let a_color = renderer.attribute(A_COLOR, 3);
    let program = renderer.program(vert, frag, vec![a_position, a_color]);
    let blend_mode = renderer.pre_multiplied_blend();
    let primitive = renderer.triangle_strip_primitive();
    let pipeline = renderer.pipeline(program, blend_mode, primitive);
    let clear_color = renderer.clear_color(0., 0., 0., 0.);

    event_loop.run(move |event, _, control_flow| {
        match event {
            event::Event::RedrawRequested(_) => {
                renderer.set_attribute(&pipeline, A_POSITION, &[0., 1., -1., -1., 1., -1., 0., -0.]);
                renderer.set_attribute(&pipeline, A_COLOR, &[1., 0., 0., 0., 1., 0., 0., 0., 1., 1., 0., 0.]);

                renderer.render(&pipeline, Some(clear_color), (1, 4));
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
