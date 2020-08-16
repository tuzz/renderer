use std::mem;

pub struct Texture {
    pub inner: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub size: (u32, u32),
    pub format: crate::Format,
}

impl Texture {
    pub fn new(device: &wgpu::Device, size: (u32, u32), filter_mode: crate::FilterMode, format: crate::Format) -> Self {
        let inner = create_texture(device, size, &format);
        let view = inner.create_default_view();
        let sampler = create_sampler(device, filter_mode);

        Self { inner, view, sampler, size, format }
    }

    pub fn set_data(&self, device: &wgpu::Device, data: &[u8]) -> wgpu::CommandBuffer {
        let buffer = device.create_buffer_with_data(data, wgpu::BufferUsage::COPY_SRC);
        let mut encoder = create_command_encoder(device);

        let buffer_copy = buffer_copy_view(&buffer, self.size);
        let texture_copy = texture_copy_view(&self.inner);

        encoder.copy_buffer_to_texture(buffer_copy, texture_copy, extent(self.size));
        encoder.finish()
    }

    pub fn create_bind_group(&self, device: &wgpu::Device, visibility: &crate::Visibility) -> (wgpu::BindGroup, wgpu::BindGroupLayout) {
        let l1 = texture_binding_layout(visibility, &self.format);
        let l2 = sampler_binding_layout(visibility);
        let descriptor = wgpu::BindGroupLayoutDescriptor { bindings: &[l1, l2], label: None };
        let layout = device.create_bind_group_layout(&descriptor);

        let b1 = texture_binding(&self.view);
        let b2 = sampler_binding(&self.sampler);
        let descriptor = wgpu::BindGroupDescriptor { layout: &layout, bindings: &[b1, b2], label: None };
        let bind_group = device.create_bind_group(&descriptor);

        (bind_group, layout)
    }
}

fn create_texture(device: &wgpu::Device, size: (u32, u32), format: &crate::Format) -> wgpu::Texture {
    let descriptor = wgpu::TextureDescriptor {
        size: extent(size),
        array_layer_count: 1,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: format.texture_format(),
        usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
        label: None,
    };

    device.create_texture(&descriptor)
}

fn extent((width, height): (u32, u32)) -> wgpu::Extent3d {
    wgpu::Extent3d { width, height, depth: 1 }
}

fn create_sampler(device: &wgpu::Device, filter_mode: crate::FilterMode) -> wgpu::Sampler {
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

fn texture_binding_layout(visibility: &crate::Visibility, format: &crate::Format) -> wgpu::BindGroupLayoutEntry {
    let ty = wgpu::BindingType::SampledTexture {
        multisampled: false,
        dimension: wgpu::TextureViewDimension::D2,
        component_type: format.component_type(),
    };

    wgpu::BindGroupLayoutEntry { binding: 0, visibility: visibility.shader_stage(), ty }
}

fn sampler_binding_layout(visibility: &crate::Visibility) -> wgpu::BindGroupLayoutEntry {
    let ty = wgpu::BindingType::Sampler { comparison: false };

    wgpu::BindGroupLayoutEntry { binding: 1, visibility: visibility.shader_stage(), ty }
}

fn texture_binding(texture_view: &wgpu::TextureView) -> wgpu::Binding {
    wgpu::Binding { binding: 0, resource: wgpu::BindingResource::TextureView(texture_view) }
}

fn sampler_binding(sampler: &wgpu::Sampler) -> wgpu::Binding {
    wgpu::Binding { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) }
}
