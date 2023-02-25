use std::io::{Cursor, Write};

pub struct PngEncoder;

impl PngEncoder {
    pub fn encode_to_bytes(video_frame: &crate::VideoFrame) -> Result<Vec<u8>, &'static str> {
        let mut bytes = vec![];

        let cursor = Cursor::new(&mut bytes);
        let result = Self::encode(video_frame, cursor);

        result.map(|_| bytes)
    }

    pub fn encode<W: Write>(video_frame: &crate::VideoFrame, writer: W) -> Result<(), &'static str> {
        if video_frame.image_data.is_none() {
            return Err("VideoFrame could not be written because image_data is None.")
        }

        let mut png = png::Encoder::new(writer, video_frame.width as u32, video_frame.height as u32);

        png.set_depth(png::BitDepth::Eight);
        png.set_color(png::ColorType::Rgba);

        let mut png_writer = png.write_header().unwrap();
        let mut stream_writer = png_writer.stream_writer_with_size(video_frame.unpadded_bytes_per_row).unwrap();

        let image_data = video_frame.image_data.as_ref().unwrap();

        image_data.bytes_fn(|bytes| {
            for chunk in bytes.chunks(video_frame.padded_bytes_per_row) {
                stream_writer.write_all(&chunk[..video_frame.unpadded_bytes_per_row]).unwrap();
            }
        });

        stream_writer.finish().unwrap();
        Ok(())
    }
}
