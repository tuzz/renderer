#![feature(drain_filter)]

mod attribute;
mod blend_mode;
mod buffer;
mod capture_stream;
mod clear_color;
mod ffmpeg_pipe;
mod filter_mode;
mod format;
mod instanced;
mod pipeline;
mod primitive;
mod program;
mod renderer;
mod render_pass;
mod stream_frame;
mod target;
mod texture;
mod uniform;
mod viewport;
mod visibility;

pub use attribute::*;
pub use blend_mode::*;
pub use buffer::*;
pub use capture_stream::*;
pub use clear_color::*;
pub use ffmpeg_pipe::*;
pub use filter_mode::*;
pub use format::*;
pub use instanced::*;
pub use pipeline::*;
pub use primitive::*;
pub use program::*;
pub use renderer::*;
pub use render_pass::*;
pub use stream_frame::*;
pub use target::*;
pub use texture::*;
pub use uniform::*;
pub use viewport::*;
pub use visibility::*;

#[cfg(feature="shader_compilation")] mod compiler;
#[cfg(feature="shader_compilation")] pub use compiler::*;

#[cfg(feature="capture_to_png")] mod png_encoder;
#[cfg(feature="capture_to_png")] pub use png_encoder::*;

#[cfg(feature="capture_compression")] mod compressor;
#[cfg(feature="capture_compression")] pub use compressor::*;

#[cfg(feature="capture_compression")] mod decompressor;
#[cfg(feature="capture_compression")] pub use decompressor::*;
