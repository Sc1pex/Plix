use crate::renderer::{Renderer, RendererCallback};
use eframe::egui_wgpu;
use eframe::{egui, CreationContext};

pub struct App {
    angle: f32,
}

impl App {
    pub fn new(cc: &CreationContext) -> Option<Self> {
        let wgpu_render_state = cc.wgpu_render_state.as_ref()?;

        wgpu_render_state
            .renderer
            .write()
            .callback_resources
            .insert(Renderer::new(wgpu_render_state, [10, 10]));

        Some(Self { angle: 0. })
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("Left").show(ctx, |ui| {
            ui.heading("Hello");
        });
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
                    self.custom_painting(ui);
                });
            });
    }
}

impl App {
    fn custom_painting(&mut self, ui: &mut egui::Ui) {
        let size = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::drag());

        self.angle += response.drag_motion().x * 0.01;
        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
            rect,
            RendererCallback {},
        ));
    }
}
