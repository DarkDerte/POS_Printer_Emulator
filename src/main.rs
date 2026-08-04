#![cfg_attr(windows, windows_subsystem = "windows")]

mod codepages;
mod parser;
mod printer;
mod render;
mod sample;
mod server;
mod state;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use eframe::egui;
use egui::{Color32, ColorImage, TextureHandle, TextureOptions};

use printer::RegResult;
use state::SharedState;

const MAX_TEXTURE_H: usize = 8192;

struct EmuApp {
    state: Arc<Mutex<SharedState>>,
    fonts: Arc<render::Fonts>,
    stop: Arc<AtomicBool>,
    server: Option<JoinHandle<()>>,
    port: u16,
    paper_width: usize,
    selected: Option<u64>,
    zoom: f32,
    tex: Option<(u64, TextureHandle)>,
    show_raw: bool,
    show_summary: bool,
    msg: Option<String>,
    needs_uac: bool,
    png_msg: Option<String>,
}

impl EmuApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let state = Arc::new(Mutex::new(SharedState::new()));
        let fonts = Arc::new(render::Fonts::load());
        let stop = Arc::new(AtomicBool::new(false));
        let port: u16 = 9100;
        let ctx = cc.egui_ctx.clone();
        let server = Some(server::spawn(
            state.clone(),
            fonts.clone(),
            port,
            stop.clone(),
            ctx,
        ));
        EmuApp {
            state,
            fonts,
            stop,
            server,
            port,
            paper_width: 576,
            selected: None,
            zoom: 1.0,
            tex: None,
            show_raw: false,
            show_summary: true,
            msg: None,
            needs_uac: false,
            png_msg: None,
        }
    }

    fn start_server(&mut self, ctx: &egui::Context) {
        if self.server.is_some() {
            return;
        }
        let stop = Arc::new(AtomicBool::new(false));
        let handle = server::spawn(
            self.state.clone(),
            self.fonts.clone(),
            self.port,
            stop.clone(),
            ctx.clone(),
        );
        self.stop = stop;
        self.server = Some(handle);
    }

    fn stop_server(&mut self) {
        if let Some(h) = self.server.take() {
            self.stop.store(true, Ordering::SeqCst);
            server::poke_to_stop(self.port);
            let _ = h.join();
        }
    }

    fn send_test(&mut self) {
        let data = sample::demo_receipt();
        match std::net::TcpStream::connect(("127.0.0.1", self.port)) {
            Ok(mut s) => {
                use std::io::Write;
                let _ = s.write_all(&data);
                let _ = s.shutdown(std::net::Shutdown::Both);
                self.msg = Some(format!(
                    "Documento de prueba enviado a 127.0.0.1:{}",
                    self.port
                ));
            }
            Err(e) => self.msg = Some(format!("No se pudo enviar el documento de prueba: {e}")),
        }
    }

    fn handle_reg_result(&mut self, r: RegResult) {
        match r {
            RegResult::Ok(m) => {
                self.msg = Some(m);
                self.needs_uac = false;
            }
            RegResult::NeedsAdmin(e) => {
                self.msg = Some(format!(
                    "Se requiere permisos de administrador para modificar impresoras. ({e})"
                ));
                self.needs_uac = true;
            }
        }
    }

    fn ensure_texture(&mut self, ctx: &egui::Context, id: u64) -> Option<TextureHandle> {
        if let Some((tid, t)) = &self.tex {
            if *tid == id {
                return Some(t.clone());
            }
        }
        let (w, h, rgba) = {
            let s = self.state.lock().unwrap();
            s.jobs
                .iter()
                .find(|j| j.id == id)
                .and_then(|j| j.page.as_ref())
                .map(|p| (p.width, p.height, p.rgba.clone()))
        }?;
        let (w, h, rgba) = render::downscale_rgba(w, h, &rgba, MAX_TEXTURE_H);
        let color = ColorImage::from_rgba_unmultiplied([w, h], &rgba);
        let tex = ctx.load_texture(format!("preview_{id}"), color, TextureOptions::NEAREST);
        self.tex = Some((id, tex.clone()));
        Some(tex)
    }

    fn save_png(&self) -> String {
        let s = self.state.lock().unwrap();
        let Some(job) = s.jobs.iter().find(|j| Some(j.id) == self.selected) else {
            return "No hay trabajo seleccionado".to_string();
        };
        let Some(page) = &job.page else {
            return "El trabajo no tiene contenido renderizable".to_string();
        };
        let path = std::env::temp_dir().join(format!("pos_emulator_job_{}.png", job.id));
        match image::save_buffer(
            &path,
            &page.rgba,
            page.width as u32,
            page.height as u32,
            image::ColorType::Rgba8,
        ) {
            Ok(()) => format!("PNG guardado en {}", path.display()),
            Err(e) => format!("Error al guardar PNG: {e}"),
        }
    }

    fn save_raw(&self) -> String {
        let s = self.state.lock().unwrap();
        let Some(job) = s.jobs.iter().find(|j| Some(j.id) == self.selected) else {
            return "No hay trabajo seleccionado".to_string();
        };
        let path = std::env::temp_dir().join(format!("pos_emulator_job_{}.bin", job.id));
        match std::fs::write(&path, &job.raw) {
            Ok(()) => format!("Datos crudos guardados en {}", path.display()),
            Err(e) => format!("Error al guardar: {e}"),
        }
    }
}

impl eframe::App for EmuApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.top_panel(ui);
        self.left_panel(ui);
        self.logs_panel(ui);
        self.central_panel(ui, &ctx);
        ctx.request_repaint();
    }
}

impl EmuApp {
    fn top_panel(&mut self, ui: &mut egui::Ui) {
        let dark = ui.visuals().dark_mode;
        let bg = if dark {
            Color32::from_rgb(28, 33, 44)
        } else {
            Color32::from_rgb(245, 247, 250)
        };
        egui::Panel::top("top")
            .frame(egui::Frame::default().fill(bg).inner_margin(egui::Margin::symmetric(12, 8)))
            .show(ui, |ui| {
                let running = self.server.is_some();
                ui.horizontal(|ui| {
                    egui::Frame::default()
                        .fill(Color32::from_rgb(30, 120, 220))
                        .corner_radius(6)
                        .inner_margin(egui::Margin::symmetric(8, 3))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("POS")
                                    .strong()
                                    .size(14.0)
                                    .color(Color32::WHITE),
                            );
                        });
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Emulador de Impresora POS").size(17.0).strong());
                    ui.add_space(16.0);
                    let (status_color, status_text) = if running {
                        (
                            Color32::from_rgb(40, 160, 90),
                            format!("●  ESCUCHANDO en 0.0.0.0:{}", self.port),
                        )
                    } else {
                        (Color32::from_rgb(150, 152, 158), "○  DETENIDO".to_string())
                    };
                    egui::Frame::default()
                        .fill(status_color)
                        .corner_radius(12)
                        .inner_margin(egui::Margin::symmetric(10, 3))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(status_text).size(12.0).color(Color32::WHITE),
                            );
                        });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if running {
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Detener servidor").color(Color32::WHITE),
                                    )
                                    .fill(Color32::from_rgb(200, 70, 60)),
                                )
                                .clicked()
                            {
                                self.stop_server();
                            }
                        } else if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Iniciar servidor").color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(40, 160, 90)),
                            )
                            .clicked()
                        {
                            self.start_server(ui.ctx());
                        }
                    });
                });
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label("Puerto:");
                    ui.add_enabled(!running, egui::DragValue::new(&mut self.port).range(1..=65535));
                    ui.separator();
                    ui.label("Papel:");
                    egui::ComboBox::from_id_salt("paper")
                        .selected_text(format!("{} px", self.paper_width))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.paper_width, 384, "58 mm (384 px)");
                            ui.selectable_value(&mut self.paper_width, 576, "80 mm (576 px)");
                            ui.selectable_value(&mut self.paper_width, 832, "112 mm (832 px)");
                        });
                    {
                        let mut s = self.state.lock().unwrap();
                        if s.paper_width != self.paper_width {
                            s.paper_width = self.paper_width;
                        }
                    }
                    ui.separator();
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Enviar documento de prueba")
                                    .color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(30, 120, 220)),
                        )
                        .clicked()
                    {
                        self.send_test();
                    }
                    ui.separator();
                    if ui.button("Registrar impresora en Windows").clicked() {
                        let r = printer::register_printer();
                        self.handle_reg_result(r);
                    }
                    if self.needs_uac
                        && ui.button("Reintentar como administrador (UAC)").clicked()
                    {
                        let r = printer::register_printer_uac();
                        self.handle_reg_result(r);
                    }
                    if ui.button("Quitar impresora").clicked() {
                        let r = printer::unregister_printer();
                        self.handle_reg_result(r);
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    let (n, bytes) = {
                        let s = self.state.lock().unwrap();
                        (s.jobs.len(), s.total_bytes)
                    };
                    ui.label(
                        egui::RichText::new(format!("Trabajos: {n}  •  Bytes totales: {bytes}"))
                            .weak(),
                    );
                    if let Some(m) = &self.msg {
                        ui.separator();
                        ui.colored_label(Color32::from_rgb(220, 180, 0), m);
                    }
                });
                {
                    let s = self.state.lock().unwrap();
                    if let Some(e) = &s.server_error {
                        ui.colored_label(Color32::RED, format!("Error del servidor: {e}"));
                    }
                }
            });
    }

    fn left_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("jobs")
            .resizable(true)
            .default_size(280.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Peticiones");
                    if ui.small_button("Vaciar").clicked() {
                        let mut s = self.state.lock().unwrap();
                        s.jobs.clear();
                        s.total_bytes = 0;
                        self.selected = None;
                        self.tex = None;
                    }
                });
                let jobs: Vec<(u64, String, String, usize)> = {
                    let s = self.state.lock().unwrap();
                    s.jobs
                        .iter()
                        .map(|j| (j.id, j.when.clone(), j.peer.clone(), j.size))
                        .collect()
                };
                ui.separator();
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    for (id, when, peer, size) in jobs.iter().rev() {
                        let label = format!("#{id}  {when}\n{peer}  ({size} B)");
                        if ui
                            .selectable_label(self.selected == Some(*id), label)
                            .clicked()
                        {
                            self.selected = Some(*id);
                            self.tex = None;
                        }
                    }
                    if jobs.is_empty() {
                        ui.weak("Sin peticiones todavía.");
                    }
                });
            });
    }

    fn logs_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("logs")
            .resizable(true)
            .default_size(140.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Registro");
                    if ui.small_button("Limpiar").clicked() {
                        let mut s = self.state.lock().unwrap();
                        s.logs.clear();
                    }
                });
                let logs: Vec<String> = {
                    let s = self.state.lock().unwrap();
                    s.logs.iter().rev().take(80).cloned().collect()
                };
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for l in logs {
                            ui.monospace(l);
                        }
                    });
            });
    }

    fn central_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        egui::CentralPanel::default_margins().show(ui, |ui| {
            let info: Option<(u64, String, String, usize)> = {
                let s = self.state.lock().unwrap();
                s.jobs
                    .iter()
                    .find(|j| Some(j.id) == self.selected)
                    .map(|j| (j.id, j.when.clone(), j.peer.clone(), j.size))
            };
            let Some((id, when, peer, size)) = info else {
                ui.centered_and_justified(|ui| {
                    ui.label("Selecciona una petición en el panel izquierdo para ver su preview.\n\nLas aplicaciones pueden imprimir conectándose a 0.0.0.0:9100 o a la impresora 'POS Printer Emulator' registrada en Windows.");
                });
                return;
            };
            ui.horizontal(|ui| {
                ui.heading(format!("Petición #{id}"));
                ui.separator();
                ui.label(format!("{when}  •  desde {peer}  •  {size} bytes"));
            });
            if let Some(p) = &self.png_msg {
                ui.colored_label(Color32::from_rgb(120, 200, 120), p.clone());
            }
            ui.horizontal(|ui| {
                ui.add(egui::Slider::new(&mut self.zoom, 0.25..=4.0).text("Zoom"));
                if ui.button("Guardar PNG").clicked() {
                    let m = self.save_png();
                    self.png_msg = Some(m);
                }
                if ui.button("Guardar datos crudos").clicked() {
                    let m = self.save_raw();
                    self.png_msg = Some(m);
                }
            });

            let texture = self.ensure_texture(ctx, id);
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    match texture {
                        Some(tex) => {
                            let size = tex.size_vec2();
                            let shown = egui::Vec2::new(size.x * self.zoom, size.y * self.zoom);
                            ui.add(
                                egui::Image::from_texture(&tex)
                                    .fit_to_exact_size(shown)
                                    .texture_options(TextureOptions::NEAREST),
                            );
                        }
                        None => {
                            ui.weak("Sin contenido renderizable (0 bytes recibidos).");
                        }
                    }
                });

            ui.separator();
            ui.checkbox(&mut self.show_raw, "Mostrar datos crudos (hexdump)");
            if self.show_raw {
                let hex = {
                    let s = self.state.lock().unwrap();
                    s.jobs
                        .iter()
                        .find(|j| j.id == id)
                        .map(|j| hexdump(&j.raw))
                        .unwrap_or_default()
                };
                egui::ScrollArea::vertical()
                    .id_salt("hexdump")
                    .max_height(220.0)
                    .show(ui, |ui| {
                        ui.monospace(hex);
                    });
            }
            ui.checkbox(&mut self.show_summary, "Mostrar resumen del comando");
            if self.show_summary {
                let summary: Vec<String> = {
                    let s = self.state.lock().unwrap();
                    s.jobs
                        .iter()
                        .find(|j| j.id == id)
                        .map(|j| j.summary.clone())
                        .unwrap_or_default()
                };
                egui::ScrollArea::vertical()
                    .id_salt("summary")
                    .max_height(180.0)
                    .show(ui, |ui| {
                        for l in &summary {
                            ui.monospace(format!("• {l}"));
                        }
                        if summary.is_empty() {
                            ui.weak("Sin observaciones.");
                        }
                    });
            }
        });
    }
}

fn hexdump(data: &[u8]) -> String {
    let mut out = String::new();
    for (i, chunk) in data.chunks(16).enumerate() {
        out.push_str(&format!("{:06x}  ", i * 16));
        for b in chunk {
            out.push_str(&format!("{b:02x} "));
        }
        for _ in chunk.len()..16 {
            out.push_str("   ");
        }
        out.push_str("  |");
        for &b in chunk {
            out.push(if (0x20..=0x7E).contains(&b) {
                b as char
            } else {
                '.'
            });
        }
        out.push_str("|\n");
    }
    out
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 840.0])
            .with_title("Emulador de Impresora POS"),
        ..Default::default()
    };
    eframe::run_native(
        "Emulador de Impresora POS",
        options,
        Box::new(|cc| Ok(Box::new(EmuApp::new(cc)))),
    )
}
