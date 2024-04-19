use eframe::wgpu;

pub struct Texture {
    pub width: u32,
    pub height: u32,

    pub view: wgpu::TextureView,
    pub format: wgpu::TextureFormat,
    sampler: wgpu::Sampler,
}

impl Texture {
    pub fn new(
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        device: &wgpu::Device,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            width,
            height,

            view,
            format,
            sampler,
        }
    }

    pub fn texture_binding_type(&self) -> wgpu::BindingType {
        wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        }
    }
    pub fn sampler_binding_type(&self) -> wgpu::BindingType {
        wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)
    }

    pub fn texture_binding_resource(&self) -> wgpu::BindingResource {
        wgpu::BindingResource::TextureView(&self.view)
    }
    pub fn sampler_binding_resource(&self) -> wgpu::BindingResource {
        wgpu::BindingResource::Sampler(&self.sampler)
    }
}
