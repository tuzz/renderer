pub struct Texture {
    inner: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
}

impl Texture {
    pub fn new(device: &wgpu::Device, bytes: &[u8], size: (u32, u32), filter_mode: &crate::FilterMode) -> Self {
        let inner = create_texture(device, size);
        let view = inner.create_default_view();
        let sampler = create_sampler(device, filter_mode);

        Self { inner, view, sampler }
    }
}

fn create_texture(device: &wgpu::Device, size: (u32, u32)) -> wgpu::Texture {
    let (width, height) = size;

    let descriptor = wgpu::TextureDescriptor {
        size: wgpu::Extent3d { width, height, depth: 1 },
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
