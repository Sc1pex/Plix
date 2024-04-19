use crate::renderer::{Renderer, RendererCallback};
use eframe::egui_wgpu;
use eframe::{egui, CreationContext};
use notify::Watcher;

pub struct App {
    fs_rx: std::sync::mpsc::Receiver<Result<notify::Event, notify::Error>>,
    _watcher: notify::RecommendedWatcher,
}

impl App {
    pub fn new(cc: &CreationContext) -> Option<Self> {
        let wgpu_render_state = cc.wgpu_render_state.as_ref()?;

        wgpu_render_state
            .renderer
            .write()
            .callback_resources
            .insert(Renderer::new(wgpu_render_state, [10, 10]));

        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = notify::RecommendedWatcher::new(tx, notify::Config::default()).ok()?;
        watcher
            .watch(
                std::path::Path::new("src/"),
                notify::RecursiveMode::Recursive,
            )
            .ok()?;

        Some(Self {
            fs_rx: rx,
            _watcher: watcher,
        })
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

        ctx.request_repaint();
    }
}

impl App {
    fn custom_painting(&mut self, ui: &mut egui::Ui) {
        let size = ui.available_size();
        let (_, rect) = ui.allocate_space(size);

        let fs_event = match self.fs_rx.try_recv() {
            Ok(e) => match e {
                Ok(e) => Some(e),
                Err(e) => {
                    println!("Fs watcher error: {}", e);
                    None
                }
            },
            Err(e) => {
                if matches!(e, std::sync::mpsc::TryRecvError::Disconnected) {
                    println!("Fs watcher channel closed")
                }
                None
            }
        };

        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
            rect,
            RendererCallback { fs_event },
        ));
    }
}
