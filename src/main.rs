use app::App;
use eframe::egui;

mod app;
mod compute;
mod export;
mod renderer;
mod shader_manager;
mod texture;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder {
            decorations: Some(false),
            ..Default::default()
        },
        ..Default::default()
    };
    eframe::run_native(
        "Plix",
        options,
        Box::new(|cc| Box::new(App::new(cc).unwrap())),
    )
}
