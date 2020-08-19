use std::{cell, mem, ops, rc};

#[derive(Clone)]
pub struct Texture {
    pub inner: rc::Rc<cell::RefCell<InnerT>>,
}

pub struct InnerT {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub size: (u32, u32),
    pub format: crate::Format,
    pub renderable: bool,
    pub generation: u32,
}

impl Texture {
    pub fn new(device: &wgpu::Device, size: (u32, u32), filter_mode: crate::FilterMode, format: crate::Format, renderable: bool) -> Self {
        let texture = create_texture(device, size, &format, renderable);
        let view = texture.create_default_view();
        let sampler = create_sampler(device, filter_mode);
        let inner = InnerT { texture, view, sampler, size, format, renderable, generation: 0 };

        Self { inner: rc::Rc::new(cell::RefCell::new(inner)) }
    }

    pub fn resize(&mut self, device: &wgpu::Device, new_size: (u32, u32)) {
        if self.size == new_size { return; }

        let mut inner = self.inner.borrow_mut();
        inner.size = new_size;
        inner.texture = create_texture(device, inner.size, &inner.format, inner.renderable);
        inner.view = inner.texture.create_default_view();

        // Use generational indexing so pipelines know when they need to be recreated.
        inner.generation += 1;
    }

    pub fn set_data(&self, device: &wgpu::Device, data: &[u8]) -> wgpu::CommandBuffer {
        let buffer = device.create_buffer_with_data(data, wgpu::BufferUsage::COPY_SRC);
        let mut encoder = create_command_encoder(device);

        let buffer_copy = buffer_copy_view(&buffer, self.size);
        let texture_copy = texture_copy_view(&self.texture);

        encoder.copy_buffer_to_texture(buffer_copy, texture_copy, extent(self.size));
        encoder.finish()
    }

    pub fn texture_binding(&self, visibility: &crate::Visibility, id: u32) -> (wgpu::Binding, wgpu::BindGroupLayoutEntry) {
        let layout = texture_binding_layout(id, visibility, &self.format);
        let binding = texture_binding(id, &self.view);

        (binding, layout)
    }

    pub fn sampler_binding(&self, visibility: &crate::Visibility, id: u32) -> (wgpu::Binding, wgpu::BindGroupLayoutEntry) {
        let layout = sampler_binding_layout(id, visibility);
        let binding = sampler_binding(id, &self.sampler);

        (binding, layout)
    }
}

fn create_texture(device: &wgpu::Device, size: (u32, u32), format: &crate::Format, renderable: bool) -> wgpu::Texture {
    let mut usage = wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST;
    if renderable { usage |= wgpu::TextureUsage::OUTPUT_ATTACHMENT; }

    let descriptor = wgpu::TextureDescriptor {
        size: extent(size),
        array_layer_count: 1,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: format.texture_format(),
        usage,
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

fn texture_binding_layout(id: u32, visibility: &crate::Visibility, format: &crate::Format) -> wgpu::BindGroupLayoutEntry {
    let ty = wgpu::BindingType::SampledTexture {
        multisampled: false,
        dimension: wgpu::TextureViewDimension::D2,
        component_type: format.component_type(),
    };

    wgpu::BindGroupLayoutEntry { binding: id, visibility: visibility.shader_stage(), ty }
}

fn sampler_binding_layout(id: u32, visibility: &crate::Visibility) -> wgpu::BindGroupLayoutEntry {
    let ty = wgpu::BindingType::Sampler { comparison: false };

    wgpu::BindGroupLayoutEntry { binding: id, visibility: visibility.shader_stage(), ty }
}

fn texture_binding(id: u32, texture_view: &wgpu::TextureView) -> wgpu::Binding {
    wgpu::Binding { binding: id, resource: wgpu::BindingResource::TextureView(texture_view) }
}

fn sampler_binding(id: u32, sampler: &wgpu::Sampler) -> wgpu::Binding {
    wgpu::Binding { binding: id, resource: wgpu::BindingResource::Sampler(sampler) }
}

impl ops::Deref for Texture {
    type Target = InnerT;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.inner.try_borrow_unguarded().unwrap() }
    }
}
