use std::io::{Cursor, Write};

pub struct PngEncoder;

impl PngEncoder {
    pub fn encode_to_bytes(stream_frame: &crate::StreamFrame) -> Result<Vec<u8>, &'static str> {
        let mut bytes = vec![];

        let cursor = Cursor::new(&mut bytes);
        let result = Self::encode(stream_frame, cursor);

        result.map(|_| bytes)
    }

    pub fn encode<W: Write>(stream_frame: &crate::StreamFrame, writer: W) -> Result<(), &'static str> {
        if stream_frame.image_data.is_none() {
            return Err("StreamFrame could not be written because image_data is None.")
        }

        let mut png = png::Encoder::new(writer, stream_frame.width as u32, stream_frame.height as u32);

        png.set_depth(png::BitDepth::Eight);
        png.set_color(png::ColorType::RGBA);

        let mut png_writer = png.write_header().unwrap();
        let mut stream_writer = png_writer.stream_writer_with_size(stream_frame.unpadded_bytes_per_row);

        let image_data = stream_frame.image_data.as_ref().unwrap();

        image_data.bytes_fn(|bytes| {
            for chunk in bytes.chunks(stream_frame.padded_bytes_per_row) {
                stream_writer.write_all(&chunk[..stream_frame.unpadded_bytes_per_row]).unwrap();
            }
        });

        stream_writer.finish().unwrap();
        Ok(())
    }
}
