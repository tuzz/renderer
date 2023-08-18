#![feature(extract_if)]

mod attribute;
mod blend_mode;
mod buffer;
mod clear_color;
mod filter_mode;
mod format;
mod instanced;
mod pipeline;
mod primitive;
mod program;
mod renderer;
mod render_pass;
mod target;
mod texture;
mod uniform;
mod video_frame;
mod video_recorder;
mod viewport;
mod visibility;

pub use attribute::*;
pub use blend_mode::*;
pub use buffer::*;
pub use clear_color::*;
pub use filter_mode::*;
pub use format::*;
pub use instanced::*;
pub use pipeline::*;
pub use primitive::*;
pub use program::*;
pub use renderer::*;
pub use render_pass::*;
pub use target::*;
pub use texture::*;
pub use uniform::*;
pub use video_frame::*;
pub use video_recorder::*;
pub use viewport::*;
pub use visibility::*;

#[cfg(feature="shader_compilation")] mod compiler;
#[cfg(feature="shader_compilation")] pub use compiler::*;

#[cfg(feature="frame_compression")] mod compressor;
#[cfg(feature="frame_compression")] pub use compressor::*;

#[cfg(feature="frame_compression")] mod decompressor;
#[cfg(feature="frame_compression")] pub use decompressor::*;

#[cfg(feature="frame_to_png")] mod png_encoder;
#[cfg(feature="frame_to_png")] pub use png_encoder::*;

#[cfg(feature="pipe_to_ffmpeg")] mod ffmpeg_pipe;
#[cfg(feature="pipe_to_ffmpeg")] pub use ffmpeg_pipe::*;
