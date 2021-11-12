pub struct Decompressor;

impl Decompressor {
    pub fn new(directory: &str, concurrent: bool, remove_after: bool) -> Self {
        Self
    }

    pub fn decompress_from_disk(&self, process_function: Box<dyn Fn(crate::StreamFrame)>) {

    }
}
