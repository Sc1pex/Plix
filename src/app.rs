use std::sync::mpsc;

use crate::compute::Compute;
use crate::export::Export;
use crate::renderer::Renderer;
use crate::shader_manager::ShaderManager;
use eframe::{egui, emath, CreationContext};
use eframe::{egui_wgpu, wgpu};

pub struct App {
    export: Export,
    shader_manager: ShaderManager,
    shader_manager_rx: mpsc::Receiver<String>,

    show_menu: bool,
}

impl App {
    pub fn new(cc: &CreationContext) -> Option<Self> {
        let (tx, rx) = mpsc::channel();
        let shader_manager = ShaderManager::new(tx)?;

        let wgpu_render_state = cc.wgpu_render_state.as_ref()?;
        let renderer = Renderer::new(wgpu_render_state, [10, 10]);
        let compute = Compute::new(
            &wgpu_render_state.device,
            &renderer.texture,
            shader_manager.selected(),
        );

        wgpu_render_state
            .renderer
            .write()
            .callback_resources
            .insert((renderer, compute));

        Some(Self {
            export: Export::new(),
            shader_manager,
            shader_manager_rx: rx,

            show_menu: true,
        })
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        let t = ctx.input(|i| i.time);
        ctx.input(|i| {
            if i.key_pressed(egui::Key::M) {
                self.show_menu = !self.show_menu;
            }
        });

        self.shader_manager.update();
        if self.show_menu {
            egui::SidePanel::left("Left").show(ctx, |ui| {
                self.shader_manager.render_ui(ui);
                ui.add_space(40.);
                self.export.render_save_ui(ui);
            });
        }

        egui::CentralPanel::default()
            .frame(egui::Frame {
                inner_margin: egui::Margin {
                    left: 0.,
                    right: 0.,
                    top: 0.,
                    bottom: 0.,
                },
                ..Default::default()
            })
            .show(ctx, |ui| {
                egui::Frame::canvas(ui.style()).show(ui, |ui| {
                    self.custom_painting(ui, t);
                });
            });

        ctx.request_repaint();
    }
}

impl App {
    fn custom_painting(&mut self, ui: &mut egui::Ui, t: f64) {
        let size = ui.available_size();
        let (_, rect) = ui.allocate_space(size);

        let reload_shader = self.shader_manager_rx.try_recv().ok();

        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
            rect,
            RendererCallback {
                reload_shader,
                size,
                t,
            },
        ));
    }
}

pub struct RendererCallback {
    reload_shader: Option<String>,
    size: emath::Vec2,

    t: f64,
}

impl egui_wgpu::CallbackTrait for RendererCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let (renderer, compute): &mut (Renderer, Compute) = resources.get_mut().unwrap();

        if let Some(s) = self.reload_shader.as_ref() {
            compute.reload_shader(device, s);
        }

        if renderer.check_resize(device, [self.size.x as u32, self.size.y as u32]) {
            compute.update_texture(device, &renderer.texture);
            compute.update_texture_size(queue, [renderer.texture.width, renderer.texture.height]);
        }
        compute.update_time(queue, self.t as f32);

        compute.step(device, queue, None);

        Vec::new()
    }

    fn paint<'a>(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'a>,
        resources: &'a egui_wgpu::CallbackResources,
    ) {
        let (renderer, _): &(Renderer, Compute) = resources.get().unwrap();
        renderer.paint(render_pass);
    }
}
