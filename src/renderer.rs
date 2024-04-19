use crate::texture::Texture;
use bytemuck::{Pod, Zeroable};
use eframe::{
    egui_wgpu,
    wgpu::{self, util::DeviceExt},
};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
}
impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3],
        }
    }
}
const VERTICES: &[Vertex] = &[
    Vertex {
        position: [1., 1., 0.0],
    },
    Vertex {
        position: [-1., 1., 0.0],
    },
    Vertex {
        position: [-1., -1., 0.0],
    },
    Vertex {
        position: [1., -1., 0.0],
    },
];

#[rustfmt::skip]
const INDICES: &[u16] = &[
    0, 1, 2,
    0, 2, 3
];

pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
    shader: wgpu::ShaderModule,
    target_format: wgpu::ColorTargetState,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,

    pub texture: Texture,
    texture_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,
}

impl Renderer {
    pub fn new(render_state: &egui_wgpu::RenderState, dim: [u32; 2]) -> Self {
        let device = &render_state.device;

        let texture_format = wgpu::TextureFormat::Rgba8Unorm;
        let texture = Texture::new(dim[0], dim[1], texture_format, device);

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: texture.texture_binding_type(),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: texture.sampler_binding_type(),
                        count: None,
                    },
                ],
            });
        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: texture.texture_binding_resource(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: texture.sampler_binding_resource(),
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                std::fs::read_to_string("src/render.wgsl")
                    .expect("Shader not found")
                    .into(),
            ),
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let target_format: wgpu::ColorTargetState = render_state.target_format.into();
        let pipeline = Self::create_pipeline(
            target_format.clone(),
            &shader,
            &[&texture_bind_group_layout],
            device,
        );

        Self {
            pipeline,
            shader,
            target_format,

            vertex_buffer,
            index_buffer,

            texture,
            texture_bind_group,
            texture_bind_group_layout,
        }
    }

    pub fn reload_shader(&mut self, device: &wgpu::Device) {
        self.shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                std::fs::read_to_string("src/render.wgsl")
                    .expect("Shader not found")
                    .into(),
            ),
        });

        self.pipeline = Self::create_pipeline(
            self.target_format.clone(),
            &self.shader,
            &[&self.texture_bind_group_layout],
            device,
        );
    }

    pub fn check_resize(&mut self, device: &wgpu::Device, dim: [u32; 2]) -> bool {
        if self.texture.width != dim[0] || self.texture.height != dim[1] {
            self.texture = Texture::new(dim[0], dim[1], self.texture.format, device);

            self.texture_bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: self.texture.texture_binding_type(),
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: self.texture.sampler_binding_type(),
                            count: None,
                        },
                    ],
                });
            self.texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.texture.texture_binding_resource(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.texture.sampler_binding_resource(),
                    },
                ],
            });

            self.pipeline = Self::create_pipeline(
                self.target_format.clone(),
                &self.shader,
                &[&self.texture_bind_group_layout],
                device,
            );
            return true;
        }
        false
    }

    pub fn paint<'rp>(&'rp self, render_pass: &mut wgpu::RenderPass<'rp>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..INDICES.len() as u32, 0, 0..1)
    }

    fn create_pipeline(
        target_format: wgpu::ColorTargetState,
        shader: &wgpu::ShaderModule,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
        device: &wgpu::Device,
    ) -> wgpu::RenderPipeline {
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts,
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: "fs_main",
                targets: &[Some(target_format.clone())],
            }),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            primitive: wgpu::PrimitiveState::default(),
            multiview: None,
        })
    }
}
