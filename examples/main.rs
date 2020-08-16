use winit::{event, event_loop, window};

const A_POSITION: usize = 0;
const A_COLOR: usize = 1;

const U_TEXTURE: usize = 0;

fn main() {
    renderer::Compiler::compile_shaders("src/shaders");

    let event_loop = event_loop::EventLoop::new();
    let window = window::WindowBuilder::new().build(&event_loop).unwrap();
    let mut renderer = renderer::Renderer::new(&window);

    let vert = include_bytes!("../src/shaders/hello.vert.spirv");
    let frag = include_bytes!("../src/shaders/hello.frag.spirv");

    let letter_f = include_bytes!("../src/images/letter_f.png");
    let (image, width, height) = load_image(letter_f);

    let a_position = renderer.attribute(A_POSITION, 2);
    let a_color = renderer.attribute(A_COLOR, 3);
    let filter_mode = renderer.linear_filtering();
    let u_texture = renderer.texture(width, height, filter_mode);
    let visibility = renderer.visible_to_fragment_shader();
    let program = renderer.program(vert, frag, vec![a_position, a_color], vec![(u_texture, visibility)]);
    let blend_mode = renderer.pre_multiplied_blend();
    let primitive = renderer.triangle_strip_primitive();
    let pipeline = renderer.pipeline(program, blend_mode, primitive);
    let clear_color = renderer.clear_color(0., 0., 0., 0.);

    renderer.set_attribute(&pipeline, A_POSITION, &[0., 1., -1., -1., 1., -1., 0., -0.]);
    renderer.set_attribute(&pipeline, A_COLOR, &[1., 0., 0., 0., 1., 0., 0., 0., 1., 1., 0., 0.]);
    renderer.set_texture(&pipeline, U_TEXTURE, &image);

    event_loop.run(move |event, _, control_flow| {
        match event {
            event::Event::RedrawRequested(_) => {
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

fn load_image(bytes: &[u8]) -> (Vec<u8>, u32, u32) {
    let mut decoder = png::Decoder::new(bytes);

    // Don't strip the alpha channel from the png.
    decoder.set_transformations(png::Transformations::IDENTITY);

    let (info, mut reader) = decoder.read_info().unwrap();
    let mut buffer = vec![0; info.buffer_size()];

    reader.next_frame(&mut buffer).unwrap();
    premultiply_alpha(&mut buffer);

    (buffer, info.width, info.height)
}

fn premultiply_alpha(buffer: &mut Vec<u8>) {
    for chunk in buffer.chunks_mut(4) {
        let alpha = (chunk[3] as f32) / 255.;

        chunk[0] = (chunk[0] as f32 * alpha).round() as u8;
        chunk[1] = (chunk[1] as f32 * alpha).round() as u8;
        chunk[2] = (chunk[2] as f32 * alpha).round() as u8;
    }
}
