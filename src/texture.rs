use std::mem;

pub struct Texture {
    inner: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    size: (u32, u32),
}

impl Texture {
    pub fn new(device: &wgpu::Device, size: (u32, u32), filter_mode: &crate::FilterMode) -> Self {
        let inner = create_texture(device, size);
        let view = inner.create_default_view();
        let sampler = create_sampler(device, filter_mode);

        Self { inner, view, sampler, size }
    }

    pub fn set_data(&self, device: &wgpu::Device, data: &[u8]) -> wgpu::CommandBuffer {
        let buffer = device.create_buffer_with_data(data, wgpu::BufferUsage::COPY_SRC);
        let mut encoder = create_command_encoder(device);

        let buffer_copy = buffer_copy_view(&buffer, self.size);
        let texture_copy = texture_copy_view(&self.inner);

        encoder.copy_buffer_to_texture(buffer_copy, texture_copy, extent(self.size));
        encoder.finish()
    }
}

fn create_texture(device: &wgpu::Device, size: (u32, u32)) -> wgpu::Texture {
    let descriptor = wgpu::TextureDescriptor {
        size: extent(size),
        array_layer_count: 1,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
        label: None,
    };

    device.create_texture(&descriptor)
}

fn extent((width, height): (u32, u32)) -> wgpu::Extent3d {
    wgpu::Extent3d { width, height, depth: 1 }
}

fn create_sampler(device: &wgpu::Device, filter_mode: &crate::FilterMode) -> wgpu::Sampler {
    let descriptor = wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: filter_mode.to_wgpu(),
        min_filter: filter_mode.to_wgpu(),
        mipmap_filter: wgpu::FilterMode::Nearest,
        lod_min_clamp: 0.,
        lod_max_clamp: 0.,
        compare: wgpu::CompareFunction::Never,
    };

    device.create_sampler(&descriptor)
}

fn create_command_encoder(device: &wgpu::Device) -> wgpu::CommandEncoder {
    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None })
}

fn buffer_copy_view(buffer: &wgpu::Buffer, (width, height): (u32, u32)) -> wgpu::BufferCopyView {
    wgpu::BufferCopyView {
        buffer: buffer,
        offset: 0,
        bytes_per_row: mem::size_of::<f32>() as u32 * width,
        rows_per_image: height,
    }
}

fn texture_copy_view(texture: &wgpu::Texture) -> wgpu::TextureCopyView {
    wgpu::TextureCopyView {
        texture: texture,
        mip_level: 0,
        array_layer: 0,
        origin: wgpu::Origin3d::ZERO,
    }
}
