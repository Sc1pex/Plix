use crate::compute::Compute;
use crate::renderer::Renderer;
use eframe::{egui, emath, CreationContext};
use eframe::{egui_wgpu, wgpu};
use notify::Watcher;

pub struct App {
    fs_rx: std::sync::mpsc::Receiver<Result<notify::Event, notify::Error>>,
    _watcher: notify::RecommendedWatcher,
}

impl App {
    pub fn new(cc: &CreationContext) -> Option<Self> {
        let wgpu_render_state = cc.wgpu_render_state.as_ref()?;
        let renderer = Renderer::new(wgpu_render_state, [10, 10]);
        let compute = Compute::new(&wgpu_render_state.device, &renderer.texture);

        wgpu_render_state
            .renderer
            .write()
            .callback_resources
            .insert((renderer, compute));

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
            RendererCallback { fs_event, size },
        ));
    }
}

pub struct RendererCallback {
    fs_event: Option<notify::Event>,
    size: emath::Vec2,
}

impl RendererCallback {
    fn handle_fs(
        &self,
        renderer: &mut Renderer,
        compute: &mut Compute,
        device: &wgpu::Device,
    ) -> Option<()> {
        let event = self.fs_event.clone()?;
        if matches!(
            event,
            notify::Event {
                kind: notify::EventKind::Access(notify::event::AccessKind::Close(
                    notify::event::AccessMode::Write
                )),
                ..
            }
        ) {
            let path = event.paths.first()?.to_str()?;
            if path.contains("render.wgsl") {
                renderer.reload_shader(device);
            }
            if path.contains("compute.wgsl") {
                compute.reload_shader(device);
            }
        }

        Some(())
    }
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

        self.handle_fs(renderer, compute, device);
        if renderer.check_resize(device, [self.size.x as u32, self.size.y as u32]) {
            compute.update_texture(device, &renderer.texture);
            compute.update_data(queue, [renderer.texture.width, renderer.texture.height]);
        }

        compute.step(device, queue);

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
