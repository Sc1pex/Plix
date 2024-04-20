use eframe::{
    egui::{self, containers, widgets},
    wgpu,
};
use pollster::FutureExt;
use std::sync::mpsc;

use crate::{compute::Compute, texture::Texture};

#[derive(Clone)]
struct ExportData {
    width: u32,
    height: u32,
    duration: f32,

    shader: String,
}

pub struct Export {
    data: ExportData,
    state: State,

    start_export: mpsc::Sender<ExportData>,
    export_com: mpsc::Receiver<Msg>,

    thread_msg: String,
    _thread: std::thread::JoinHandle<()>,
}

#[derive(Debug, PartialEq)]
enum State {
    Waiting,
    Generating,
}

enum Msg {
    Info(String),
    Done,
}

impl Export {
    pub fn new(shader: String) -> Self {
        let (start_tx, start_rx) = mpsc::channel();
        let (com_tx, com_rx) = mpsc::channel();

        let thread = std::thread::spawn(move || export_thread(start_rx, com_tx));

        Self {
            data: ExportData {
                width: 800,
                height: 800,
                duration: 5.,

                shader,
            },
            state: State::Waiting,

            start_export: start_tx,
            export_com: com_rx,
            _thread: thread,

            thread_msg: String::new(),
        }
    }

    pub fn set_shader(&mut self, shader: String) {
        self.data.shader = shader;
    }

    pub fn render_save_ui(&mut self, ui: &mut egui::Ui) {
        containers::CollapsingHeader::new("Export")
            .default_open(true)
            .show(ui, |ui| match self.state {
                State::Waiting => self.render_waiting(ui),
                State::Generating => self.render_generating(ui),
            });
    }

    fn render_generating(&mut self, ui: &mut egui::Ui) {
        ui.label("Generating...");
        ui.label(self.thread_msg.as_str());

        if let Ok(msg) = self.export_com.try_recv() {
            match msg {
                Msg::Info(s) => self.thread_msg = s,
                Msg::Done => self.state = State::Waiting,
            }
        }
    }

    fn render_waiting(&mut self, ui: &mut egui::Ui) {
        ui.label("Resolution");
        ui.horizontal(|ui| {
            ui.add(widgets::DragValue::new(&mut self.data.width).prefix("width: "));
            ui.add(widgets::DragValue::new(&mut self.data.height).prefix("height: "));
        });
        ui.label("Duration");
        ui.add(widgets::DragValue::new(&mut self.data.duration).suffix(" seconds"));

        ui.add_space(20.0);
        if ui.button("Export").clicked() {
            self.state = State::Generating;
            let _ = self.start_export.send(self.data.clone());
        }
    }
}

fn export_thread(start: mpsc::Receiver<ExportData>, com: mpsc::Sender<Msg>) {
    loop {
        let data = start.recv().unwrap();
        export_thread_internal(data, com.clone()).block_on();

        com.send(Msg::Done).unwrap();
    }
}

async fn export_thread_internal(data: ExportData, com: mpsc::Sender<Msg>) {
    com.send(Msg::Info("Initlializing".into())).unwrap();
    let instance = wgpu::Instance::default();

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptionsBase::default())
        .await
        .unwrap();

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
            },
            None,
        )
        .await
        .unwrap();

    let align_width = data.width + 64 - data.width % 64;
    let size = (align_width * data.height * 4) as u64;
    let copy_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let texture = Texture::new(
        align_width,
        data.height,
        wgpu::TextureFormat::Rgba8Unorm,
        &device,
        wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::STORAGE_BINDING,
    );
    let texture_copy = texture.inner.as_image_copy();
    let mut compute = Compute::new(&device, &texture, &data.shader);

    let fps = 60.;
    let time_per_frame = 1. / fps;
    let frame_count = (data.duration * fps) as usize;
    com.send(Msg::Info(format!(
        "Starting to render {} frames",
        frame_count
    )))
    .unwrap();

    for frame in 0..frame_count {
        let t = time_per_frame * frame as f32;
        compute.update_time(&queue, t);
        compute.step(
            &device,
            &queue,
            Some((
                texture_copy,
                wgpu::ImageCopyBuffer {
                    buffer: &copy_buffer,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(align_width * 4),
                        rows_per_image: Some(data.height),
                    },
                },
                wgpu::Extent3d {
                    width: align_width,
                    height: data.height,
                    depth_or_array_layers: 1,
                },
            )),
        );

        let mut texture_data = Vec::<u8>::with_capacity(size as usize);

        let buffer_slice = copy_buffer.slice(..);
        let (sender, receiver) = flume::bounded(1);
        buffer_slice.map_async(wgpu::MapMode::Read, move |r| sender.send(r).unwrap());
        device.poll(wgpu::Maintain::wait()).panic_on_timeout();
        receiver.recv_async().await.unwrap().unwrap();
        {
            let view = buffer_slice.get_mapped_range();
            texture_data.extend_from_slice(&view[..]);
        }
        copy_buffer.unmap();

        let mut imgbuf = image::ImageBuffer::new(data.width, data.height);
        for (x, mut y, pixel) in imgbuf.enumerate_pixels_mut() {
            y = data.height - y - 1;
            let idx = ((x + y * align_width) * 4) as usize;
            *pixel = image::Rgba([
                texture_data[idx],
                texture_data[idx + 1],
                texture_data[idx + 2],
                texture_data[idx + 3],
            ]);
        }
        imgbuf
            .save(format!("output/tmp/image_{}.png", frame + 1))
            .unwrap();

        com.send(Msg::Info(format!(
            "Rendered frame {}/{}",
            frame, frame_count
        )))
        .unwrap();
    }
    make_video(com.clone(), frame_count);
}

fn make_video(com: mpsc::Sender<Msg>, frames: usize) {
    com.send(Msg::Info("Converting to video".into())).unwrap();
    let file_name = format!("output/{}.mp4", chrono::Utc::now().format("%Y%m%d_%H%M%S"));

    match std::process::Command::new("ffmpeg")
        .arg("-framerate")
        .arg("60")
        .arg("-i")
        .arg("output/tmp/image_%d.png")
        .arg("-c:v")
        .arg("libx264")
        .arg("-r")
        .arg("60")
        .arg("-frames:v")
        .arg(frames.to_string())
        .arg(&file_name)
        .output()
    {
        Ok(_) => com.send(Msg::Info(format!("Saved {}", file_name))).unwrap(),
        Err(e) => com
            .send(Msg::Info(format!("Error saving file: {}", e)))
            .unwrap(),
    }
}
