use winit::{event, event_loop, window};

const A_POSITION: usize = 0;
const A_TEX_COORD: usize = 1;

const I_OFFSET: usize = 0;
const T_TEXTURE: usize = 1;

fn main() {
    renderer::Compiler::compile_shaders("src/shaders");

    let event_loop = event_loop::EventLoop::new();
    let window = window::WindowBuilder::new().build(&event_loop).unwrap();
    let mut renderer = renderer::Renderer::new(&window);

    let vert = include_bytes!("../src/shaders/hello.vert.spirv");
    let frag = include_bytes!("../src/shaders/hello.frag.spirv");

    let letter_f = include_bytes!("../src/images/letter_f.png");
    let (image, width, height) = load_image(letter_f);
    let filter = renderer.linear_filtering();
    let format = renderer.rgba_u8();

    let a_position = renderer.attribute(A_POSITION, 2);
    let a_tex_coord = renderer.attribute(A_TEX_COORD, 2);
    let i_offset = renderer.instance(2);
    let t_texture = renderer.texture(width, height, filter, format, false);

    let program = renderer.program(vert, frag, vec![
        a_position,                                         // attribute 0
        a_tex_coord,                                        // attribute 1
    ], vec![
        i_offset,                                           // set 0
    ], vec![
        // no uniforms
    ], vec![
        (t_texture, renderer.visible_to_fragment_shader()), // set 1
    ]);

    let blend_mode = renderer.pre_multiplied_blend();
    let primitive = renderer.triangle_strip_primitive();
    let target = renderer.screen_target();
    let pipeline = renderer.pipeline(program, blend_mode, primitive, target);
    let clear_color = renderer.clear_color(0., 0., 0., 0.);

    renderer.set_attribute(&pipeline, A_POSITION, &[-0.1, -0.1, -0.1, 0.1, 0.1, -0.1, 0.1, 0.1]);
    renderer.set_attribute(&pipeline, A_TEX_COORD, &[0., 1., 0., 0., 1., 1., 1., 0.]);
    renderer.set_texture(&pipeline, T_TEXTURE, &image);

    let mut x1 = (0.3, 0.015);
    let mut y1 = (-0.3, 0.01);

    let mut x2 = (-0.5, 0.005);
    let mut y2 = (-0.1, 0.02);

    event_loop.run(move |event, _, control_flow| {
        match event {
            event::Event::RedrawRequested(_) => {
                x1.0 += x1.1; if x1.0 > 0.9 || x1.0 < -0.9 { x1.1 *= -1.; }
                y1.0 += y1.1; if y1.0 > 0.9 || y1.0 < -0.9 { y1.1 *= -1.; }

                x2.0 += x2.1; if x2.0 > 0.9 || x2.0 < -0.9 { x2.1 *= -1.; }
                y2.0 += y2.1; if y2.0 > 0.9 || y2.0 < -0.9 { y2.1 *= -1.; }

                renderer.set_instanced(&pipeline, I_OFFSET, &[x1.0, y1.0, x2.0, y2.0]);
                renderer.render(&pipeline, Some(clear_color), (2, 4));
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
