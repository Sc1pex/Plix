use crate::texture::Texture;
use bytemuck::{Pod, Zeroable};
use eframe::wgpu::{self, util::DeviceExt};
use pollster::FutureExt;

pub struct Compute {
    pipeline: wgpu::ComputePipeline,
    compute_shader: wgpu::ShaderModule,

    data_bind_group: wgpu::BindGroup,
    data_bind_group_layout: wgpu::BindGroupLayout,
    data_buffer: wgpu::Buffer,
    data: ComputeDataUniform,

    texture_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,
}

impl Compute {
    pub fn new(device: &wgpu::Device, texture: &Texture) -> Self {
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                std::fs::read_to_string("src/compute.wgsl")
                    .expect("Compute shader not found")
                    .into(),
            ),
        });

        let data = ComputeDataUniform {
            width: texture.width,
            height: texture.height,

            t: 0.,
        };
        let data_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let data_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let data_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &data_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: data_buffer.as_entire_binding(),
            }],
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: texture.storage_texture_binding_type(),
                    count: None,
                }],
            });
        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &texture_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: texture.texture_binding_resource(),
            }],
        });

        let pipeline = Self::create_pipeline(
            device,
            &compute_shader,
            &[&data_bind_group_layout, &texture_bind_group_layout],
        );

        Self {
            pipeline,
            compute_shader,

            data_bind_group,
            data_bind_group_layout,
            data_buffer,
            data,

            texture_bind_group,
            texture_bind_group_layout,
        }
    }

    pub fn step(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        before_submit: Option<(
            wgpu::ImageCopyTexture,
            wgpu::ImageCopyBuffer,
            wgpu::Extent3d,
        )>,
    ) {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });

            cpass.set_pipeline(&self.pipeline);
            cpass.set_bind_group(0, &self.data_bind_group, &[]);
            cpass.set_bind_group(1, &self.texture_bind_group, &[]);
            cpass.dispatch_workgroups(self.data.width, self.data.height, 1);
        }

        if let Some((texture, buf, size)) = before_submit {
            encoder.copy_texture_to_buffer(texture, buf, size);
        }

        queue.submit(Some(encoder.finish()));
    }

    pub fn reload_shader(&mut self, device: &wgpu::Device) {
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        self.compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                std::fs::read_to_string("src/compute.wgsl")
                    .expect("Compute shader not found")
                    .into(),
            ),
        });

        if let Some(e) = device.pop_error_scope().block_on() {
            println!("Error in compute shader!!:{}", e);
            return;
        }

        self.pipeline = Self::create_pipeline(
            device,
            &self.compute_shader,
            &[
                &self.data_bind_group_layout,
                &self.texture_bind_group_layout,
            ],
        );
    }

    pub fn update_texture(&mut self, device: &wgpu::Device, texture: &Texture) {
        self.texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: texture.storage_texture_binding_type(),
                    count: None,
                }],
            });
        self.texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.texture_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: texture.texture_binding_resource(),
            }],
        });

        self.pipeline = Self::create_pipeline(
            device,
            &self.compute_shader,
            &[
                &self.data_bind_group_layout,
                &self.texture_bind_group_layout,
            ],
        );
    }

    pub fn create_pipeline(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
    ) -> wgpu::ComputePipeline {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts,
            push_constant_ranges: &[],
        });

        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: shader,
            entry_point: "main",
        })
    }

    pub fn update_texture_size(&mut self, queue: &wgpu::Queue, texture_dim: [u32; 2]) {
        self.data.width = texture_dim[0];
        self.data.height = texture_dim[1];
        queue.write_buffer(&self.data_buffer, 0, bytemuck::cast_slice(&[self.data]));
    }

    pub fn update_time(&mut self, queue: &wgpu::Queue, t: f32) {
        self.data.t = t;
        queue.write_buffer(&self.data_buffer, 0, bytemuck::cast_slice(&[self.data]));
    }
}

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy, Debug)]
struct ComputeDataUniform {
    width: u32,
    height: u32,

    t: f32,
}
