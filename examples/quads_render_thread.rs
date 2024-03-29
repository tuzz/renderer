use renderer::{Renderer, RenderThread};
use std::sync::Arc;
use winit::{event, event_loop, window};

const A_POSITION: usize = 0;
const A_TEX_COORD: usize = 1;

const I_OFFSET: (usize, usize) = (0, 0);    // set 0, binding 0
const T_TEXTURE: (usize, usize) = (0, 1);   // set 0, binding 1
                                            // The next set begins after 4 bindings.

fn main() {
    // Compile the vertex and fragment shaders for this example to SPIR-V.
    renderer::Compiler::compile_shaders("examples/quads");

    // Create a winit window and a renderer for that window.
    let event_loop = event_loop::EventLoop::new().unwrap();
    let window = Arc::new(window::WindowBuilder::new().build(&event_loop).unwrap());
    let mut renderer = RenderThread::new(window.clone());

    // Read the compiled vertex and fragment shader from disk. When making
    // changes you need to run the example twice (once first to compile).
    let vert = include_bytes!("./quads/hello.vert.spirv").to_vec();
    let frag = include_bytes!("./quads/hello.frag.spirv").to_vec();

    // Load a texture from disk.
    let letter_f = include_bytes!("./quads/letter_f.png");
    let (image, width, height) = load_image(letter_f);

    // The format of the texture is RGBA with 8 bits per channel.
    let format = Renderer::rgba_u8();

    // Use linear filtering when sampling the texture.
    let filter = Renderer::linear_filtering();

    // The x, y position for each vertex of the singular quad.
    let a_position = renderer.attribute(A_POSITION, 2);

    // The texture coordinates for each vertex of the singular quad.
    let a_tex_coord = renderer.attribute(A_TEX_COORD, 2);

    // The x, y offset for all vertices of each instanced quad.
    let i_offset = renderer.instanced();

    // The texture binding for the fragment shader (layers=1, renderable=false, copyable=false, with_sampler=true).
    let t_texture = renderer.texture(width, height, 1, filter, format, false, false, true);

    // Create a shader program with some attributes, instanced attributes,
    // uniforms and textures. The attributes are indexed separately and the rest
    // are indexed collectively (numbers follow on). The order is important!
    let program = renderer.program(vert, frag, vec![
        a_position,                                         // attribute 0
        a_tex_coord,                                        // attribute 1
    ], vec![
        i_offset,                                           // set 0, binding 0
    ], vec![
        // no uniforms
    ], vec![
        (t_texture, Renderer::visible_to_fragment_shader()), // set 0, binding 1
    ]);

    // We've already pre-multiplied the rgb channels by alpha in our texture (below).
    let blend_mode = Renderer::pre_multiplied_blend();

    // We're going to render a triangle strip to reuse vertices (indexes not supported).
    let primitive = Renderer::triangle_strip_primitive();

    // We don't need to anti-alias the quads example because all lines align with pixels
    // There's currently no way to get the supported number of samples from WGPU. Currently:
    //
    // - Vulkan should work for 1, 2, 4, and 8 samlpes
    // - DX12 is limited to 1, 4 and 8 samples
    // - macOS and DX11 are limited to 1 and 4 samples
    //
    // When samples is set to 1, MSAA is switched off completely, incurring no overhead.
    let msaa_samples = 1;

    // We're going to render to the screen but you _could_ render to a texture, too.
    let target = RenderThread::screen_target();

    // Build the shader pipeline based on all the configuration above.
    let pipeline = renderer.pipeline(program, blend_mode, primitive, msaa_samples, vec![target]);
    let clear_color = Renderer::clear_color(0., 0., 0., 1.);

    // Set all the data that won't change per render. Quads are made of four x, y coordinates.
    renderer.set_attribute(pipeline, A_POSITION, vec![-0.1, -0.1, -0.1, 0.1, 0.1, -0.1, 0.1, 0.1]);
    renderer.set_attribute(pipeline, A_TEX_COORD, vec![0., 1., 0., 0., 1., 1., 1., 0.]);
    renderer.set_texture(pipeline, T_TEXTURE, vec![image]);

    // The renderer can also record raw frames by adding f_recording to your shaders.
    // This is very CPU and data intensive (2GB/s at 4K60) so it's recommended to:
    //
    // 1) Compress the raw frame data to disk:
    let compressor = renderer::Compressor::new("recorded_frames", None, 0, true);
    renderer.start_recording(vec![pipeline], Some(clear_color), 500., Box::new(move |video_frame| {
        compressor.compress_to_disk(video_frame);
    }));

    // 2) Decompress and process this data later:
    if renderer::Decompressor::can_run("recorded_frames") && renderer::FfmpegPipe::available() {
        println!("Creating a video of the last run of this example:");

        let decompressor = renderer::Decompressor::new("recorded_frames", true);
        let mut ffmpeg_pipe = renderer::FfmpegPipe::new(None, None, Some("recorded.mp4"), &[
            "-c:v", "libx264", "-r", "60", "-pix_fmt", "yuv420p", "-movflags", "+faststart",
        ]);

        decompressor.decompress_from_disk(Arc::new(|video_frame, _timestamp| {
            renderer::PngEncoder::encode_to_bytes(video_frame)
        }), Box::new(move |video_frame, result, timestamp| {
            let png = if let Ok(Ok(png)) = result { png } else { vec![] };
            ffmpeg_pipe.write(&video_frame, png, Some(timestamp));
        }));

        println!("Written recorded.mp4. Running the example again:");
    }

    // Alternatively, you could skip compression/decompression and write PNGs directly.
    // This is slower but might be fine for your use case. Bring your own concurrecncy.

    // Set the start position of each quad and its velocity in the x, y directions.
    let mut x1 = (0.3, 0.015);
    let mut y1 = (-0.3, 0.01);

    let mut x2 = (-0.5, 0.005);
    let mut y2 = (-0.1, 0.02);

    event_loop.run(move |event, window_target| {
        match event {
            event::Event::AboutToWait => {
                window.request_redraw();
            },
            event::Event::WindowEvent { event, .. } => match event {
                event::WindowEvent::RedrawRequested => {
                    // Update the x, y positions based on the x, y velocities.
                    // If the quad reaches the edge of the screen, reverse the direction.
                    x1.0 += x1.1; if x1.0 > 0.9 || x1.0 < -0.9 { x1.1 *= -1.; }
                    y1.0 += y1.1; if y1.0 > 0.9 || y1.0 < -0.9 { y1.1 *= -1.; }

                    x2.0 += x2.1; if x2.0 > 0.9 || x2.0 < -0.9 { x2.1 *= -1.; }
                    y2.0 += y2.1; if y2.0 > 0.9 || y2.0 < -0.9 { y2.1 *= -1.; }

                    // Ensure all of the RenderThread function calls for the last frame are done.
                    renderer.synchronize();

                    // Update the quad positions that _do_ change per render.
                    renderer.set_instanced(pipeline, I_OFFSET, vec![x1.0, y1.0, x2.0, y2.0]);

                    // Set the window's viewport to a square, surrounded by black borders.
                    let viewport = renderer.viewport(1., 1.); // e.g. (16., 9.)

                    // Render two instances, each comprised of four vertices.
                    renderer.render(pipeline, Some(clear_color), Some(viewport), (2, 4));
                    renderer.finish_frame();
                },
                event::WindowEvent::Resized(size) => {
                    renderer.resize_swap_chain(&size);
                },
                event::WindowEvent::CloseRequested => {
                    window_target.exit();
                },
                _ => {},
            },
            _ => {},
        }
    }).unwrap();
}

fn load_image(bytes: &[u8]) -> (Vec<u8>, u32, u32) {
    let mut decoder = png::Decoder::new(bytes);

    // Don't strip the alpha channel from the png.
    decoder.set_transformations(png::Transformations::IDENTITY);

    let mut reader = decoder.read_info().unwrap();
    let mut buffer = vec![0; reader.output_buffer_size()];

    let info = reader.next_frame(&mut buffer).unwrap();
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
