use std::io::Write;

pub struct PngWriter;

impl PngWriter {
    pub fn write_png(filename: &str, stream_frame: &crate::StreamFrame) -> Result<(), String> {
        if stream_frame.image_data.is_none() {
            return Err(format!("Frame {} was dropped due to capture_stream memory limit.", stream_frame.frame_number));
        }

        let file = std::fs::File::create(filename).unwrap();
        let mut png = png::Encoder::new(file, stream_frame.width as u32, stream_frame.height as u32);

        png.set_depth(png::BitDepth::Eight);
        png.set_color(png::ColorType::RGBA);

        let mut writer = png.write_header().unwrap().into_stream_writer_with_size(stream_frame.unpadded_bytes_per_row);
        let image_data = stream_frame.image_data.as_ref().unwrap().slice(..).get_mapped_range();

        for chunk in image_data.chunks(stream_frame.padded_bytes_per_row) {
            writer.write_all(&chunk[..stream_frame.unpadded_bytes_per_row]).unwrap();
        }

        writer.finish().unwrap();
        Ok(())
    }
}
