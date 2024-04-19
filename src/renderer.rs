use crate::{compute::Compute, texture::Texture};
use bytemuck::{Pod, Zeroable};
use eframe::{
    egui, egui_wgpu,
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

    texture: Texture,
    texture_bind_group: wgpu::BindGroup,
    compute: Compute,
}

impl Renderer {
    pub fn new(wgpu: &egui_wgpu::RenderState, dim: [u32; 2]) -> Self {
        let device = &wgpu.device;

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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let target_format: wgpu::ColorTargetState = wgpu.target_format.into();
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(target_format.clone())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let compute = Compute::new(wgpu, &texture.view, texture_format, dim);

        Self {
            pipeline,
            shader,
            target_format,

            vertex_buffer,
            index_buffer,

            texture,
            texture_bind_group,

            compute,
        }
    }

    pub fn check_resize(&mut self, device: &wgpu::Device, dim: [u32; 2]) -> bool {
        if self.texture.width != dim[0] || self.texture.height != dim[1] {
            self.texture = Texture::new(dim[0], dim[1], self.texture.format, device);

            let texture_bind_group_layout =
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
                layout: &texture_bind_group_layout,
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

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&texture_bind_group_layout],
                push_constant_ranges: &[],
            });

            self.pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &self.shader,
                    entry_point: "vs_main",
                    buffers: &[Vertex::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &self.shader,
                    entry_point: "fs_main",
                    targets: &[Some(self.target_format.clone())],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            });

            return true;
        }
        false
    }

    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, dim: [u32; 2]) {
        if self.check_resize(device, dim) {
            self.compute
                .update_texture(device, &self.texture.view, self.texture.format);
            self.compute.update_data(queue, dim);
        }
        self.compute.step(device, queue);
    }

    pub fn paint<'rp>(&'rp self, render_pass: &mut wgpu::RenderPass<'rp>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..INDICES.len() as u32, 0, 0..1)
    }
}

pub struct RendererCallback {}

impl egui_wgpu::CallbackTrait for RendererCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let renderer: &mut Renderer = resources.get_mut().unwrap();
        renderer.prepare(device, queue, screen_descriptor.size_in_pixels);
        Vec::new()
    }

    fn paint<'a>(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'a>,
        resources: &'a egui_wgpu::CallbackResources,
    ) {
        let renderer: &Renderer = resources.get().unwrap();
        renderer.paint(render_pass);
    }
}
