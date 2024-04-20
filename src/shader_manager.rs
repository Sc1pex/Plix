use eframe::egui::{self, containers};
use notify::Watcher;
use std::sync::mpsc;

pub struct ShaderManager {
    shaders: Vec<String>,
    selected: String,

    app_tx: mpsc::Sender<String>,
    fs_rx: mpsc::Receiver<Result<notify::Event, notify::Error>>,
    _watcher: notify::RecommendedWatcher,
}

impl ShaderManager {
    pub fn new(app_tx: mpsc::Sender<String>) -> Option<Self> {
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::RecommendedWatcher::new(tx, notify::Config::default()).ok()?;
        watcher
            .watch(
                std::path::Path::new("shaders/"),
                notify::RecursiveMode::Recursive,
            )
            .ok()?;

        let mut s = Self {
            shaders: vec![],
            selected: String::new(),

            app_tx,
            fs_rx: rx,
            _watcher: watcher,
        };
        s.scan();
        Some(s)
    }

    pub fn update(&mut self) {
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
        if let Some(e) = fs_event {
            if matches!(
                e,
                notify::Event {
                    kind: notify::EventKind::Access(notify::event::AccessKind::Close(
                        notify::event::AccessMode::Write
                    )),
                    ..
                }
            ) {
                let path = e.paths.first().unwrap().to_str().unwrap();
                if path == self.selected {
                    self.app_tx.send(self.selected.clone()).unwrap();
                } else {
                    self.scan();
                }
            }
        }
    }

    pub fn scan(&mut self) {
        self.shaders.clear();

        let files = std::fs::read_dir("shaders").expect("Failed to read shaders directory");
        for file in files {
            let file = file.unwrap().path();
            if file.is_file() {
                self.shaders.push(file.to_str().unwrap().into());
            }
        }

        if !self.shaders.contains(&self.selected) {
            self.selected = self.shaders[0].clone();
        }
    }

    pub fn render_ui(&mut self, ui: &mut egui::Ui) {
        containers::CollapsingHeader::new("Select shader")
            .default_open(true)
            .show(ui, |ui| {
                self.render_shaders(ui);
                ui.add_space(20.0);
                if ui.button("Reload").clicked() {
                    self.scan();
                }
            });
    }

    pub fn selected(&self) -> &str {
        &self.selected
    }

    fn render_shaders(&mut self, ui: &mut egui::Ui) {
        for shader in self.shaders.iter() {
            let shader_name = shader.split_once("/").unwrap().1;
            let shader_name = shader_name.split_once(".").unwrap().0;
            if ui
                .selectable_label(self.selected == *shader, shader_name)
                .clicked()
            {
                self.selected = shader.into();
                self.app_tx.send(self.selected.clone()).unwrap();
            }
        }
    }
}
