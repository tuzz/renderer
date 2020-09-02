use std::{cell, ops, rc};
use wgpu::util::DeviceExt;

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
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = create_sampler(device, filter_mode);
        let inner = InnerT { texture, view, sampler, size, format, renderable, generation: 0 };

        Self { inner: rc::Rc::new(cell::RefCell::new(inner)) }
    }

    pub fn resize(&mut self, device: &wgpu::Device, new_size: (u32, u32)) {
        if self.size == new_size { return; }
        if new_size.0 == 0 || new_size.1 == 0 { return; }

        let mut inner = self.inner.borrow_mut();
        inner.size = new_size;
        inner.texture = create_texture(device, inner.size, &inner.format, inner.renderable);
        inner.view = inner.texture.create_view(&wgpu::TextureViewDescriptor::default());
        inner.generation += 1;
    }

    pub fn set_data<T: bytemuck::Pod>(&self, queue: &wgpu::Queue, data: &[T]) {
        let bytes = bytemuck::cast_slice(data);

        let texture_copy = texture_copy_view(&self.texture);
        let data_layout = texture_data_layout(&self.format, self.size);

        queue.write_texture(texture_copy, bytes, data_layout, extent(self.size));
    }

    pub fn texture_binding(&self, visibility: &crate::Visibility, id: u32) -> (wgpu::BindGroupEntry, wgpu::BindGroupLayoutEntry) {
        let layout = texture_binding_layout(id, visibility, &self.format);
        let binding = texture_binding(id, &self.view);

        (binding, layout)
    }

    pub fn sampler_binding(&self, visibility: &crate::Visibility, id: u32) -> (wgpu::BindGroupEntry, wgpu::BindGroupLayoutEntry) {
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
        anisotropy_clamp: None,
        border_color: None,
        lod_min_clamp: 0.,
        lod_max_clamp: 0.,
        compare: None,
        label: None,
    };

    device.create_sampler(&descriptor)
}

fn create_command_encoder(device: &wgpu::Device) -> wgpu::CommandEncoder {
    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None })
}

fn texture_data_layout(format: &crate::Format, (width, height): (u32, u32)) -> wgpu::TextureDataLayout {
    wgpu::TextureDataLayout {
        offset: 0,
        bytes_per_row: format.bytes_per_texel() * width,
        rows_per_image: height,
    }
}

fn texture_copy_view(texture: &wgpu::Texture) -> wgpu::TextureCopyView {
    wgpu::TextureCopyView {
        texture: texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
    }
}

fn texture_binding_layout(id: u32, visibility: &crate::Visibility, format: &crate::Format) -> wgpu::BindGroupLayoutEntry {
    let ty = wgpu::BindingType::SampledTexture {
        multisampled: false,
        dimension: wgpu::TextureViewDimension::D2,
        component_type: format.component_type(),
    };

    wgpu::BindGroupLayoutEntry { binding: id, visibility: visibility.shader_stage(), ty, count: None }
}

fn sampler_binding_layout(id: u32, visibility: &crate::Visibility) -> wgpu::BindGroupLayoutEntry {
    let ty = wgpu::BindingType::Sampler { comparison: false };

    wgpu::BindGroupLayoutEntry { binding: id, visibility: visibility.shader_stage(), ty, count: None }
}

fn texture_binding(id: u32, texture_view: &wgpu::TextureView) -> wgpu::BindGroupEntry {
    wgpu::BindGroupEntry { binding: id, resource: wgpu::BindingResource::TextureView(texture_view) }
}

fn sampler_binding(id: u32, sampler: &wgpu::Sampler) -> wgpu::BindGroupEntry {
    wgpu::BindGroupEntry { binding: id, resource: wgpu::BindingResource::Sampler(sampler) }
}

impl ops::Deref for Texture {
    type Target = InnerT;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.inner.try_borrow_unguarded().unwrap() }
    }
}
