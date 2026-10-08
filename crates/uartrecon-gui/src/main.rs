//! UARTRecon GUI (egui/eframe) entry point.
//!
//! GUI menggunakan `uartrecon-core` dan **tidak** menduplikasi logika.

mod app;
mod i18n;
mod panels;
mod waveform;

use std::sync::mpsc::{self, Receiver, Sender};

use eframe::egui;

use uartrecon_core::serial::connection::Connection;

use app::{ConnState, GuiApp, Tab, UiAction};

/// Handle ke worker serial yang sedang berjalan.
struct SerialWorker {
    rx: Receiver<Vec<u8>>,
    tx: Sender<WorkerCmd>,
}

/// Perintah ke worker serial.
enum WorkerCmd {
    /// Kirim data.
    Send(Vec<u8>),
    /// Tutup koneksi.
    Close,
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 780.0])
            .with_min_inner_size([880.0, 560.0])
            .with_title("UARTRecon — UART Recon & Analysis Toolkit")
            .with_app_id("uartrecon"),
        ..Default::default()
    };
    eframe::run_native(
        "UARTRecon",
        options,
        Box::new(|cc| {
            setup_theme(&cc.egui_ctx);
            // Muat bahasa dari config.
            let cfg = uartrecon_core::config::load();
            i18n::set_lang(uartrecon_core::i18n::Lang::parse(&cfg.language));
            Ok(Box::new(App::default()))
        }),
    )
}

/// Mengatur tampilan (dark theme, spacing, font size).
fn setup_theme(ctx: &egui::Context) {
    ctx.set_visuals(egui::Visuals::dark());
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
        style.spacing.interact_size.y = 22.0;
        use egui::{FontFamily, FontId, TextStyle};
        style.text_styles = [
            (
                TextStyle::Heading,
                FontId::new(20.0, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
            (
                TextStyle::Button,
                FontId::new(14.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Small,
                FontId::new(12.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Monospace,
                FontId::new(13.0, FontFamily::Monospace),
            ),
        ]
        .into();
    });
}

/// State top-level aplikasi (membungkus GuiApp + worker).
#[derive(Default)]
struct App {
    gui: GuiApp,
    worker: Option<SerialWorker>,
    fps: app::FpsCounter,
}

impl App {
    /// Memproses aksi dari UI.
    fn handle_action(&mut self, action: UiAction) {
        match action {
            UiAction::None => {}
            UiAction::Connect(port, config) => self.connect(port, config),
            UiAction::Disconnect => self.disconnect(),
            UiAction::Scan(port) => self.scan(port),
            UiAction::Send(text) => self.send(text),
            UiAction::SaveSession(name) => self.save_session(name),
            UiAction::LoadSession(path) => self.load_session(path),
            UiAction::Export(fmt) => self.export(fmt),
        }
    }

    /// Ekspor buffer RX ke format tertentu (pilih folder via dialog).
    fn export(&mut self, fmt: app::ExportFormat) {
        let dir = rfd::FileDialog::new().pick_folder();
        let Some(dir) = dir else {
            return;
        };
        match self.gui.export_buffer(fmt, &dir) {
            Ok(path) => {
                self.gui
                    .log(format!("Export {} -> {}", fmt.label(), path.display()));
                self.gui
                    .show_toast(format!("Export {} berhasil.", fmt.label()));
            }
            Err(e) => {
                self.gui.error = Some(format!("export gagal: {e}"));
                self.gui.log(format!("Export gagal: {e}"));
            }
        }
    }

    fn connect(&mut self, port: String, config: uartrecon_core::SerialConfig) {
        // Tutup worker lama bila ada.
        self.disconnect();

        match Connection::open(&port, config) {
            Ok(conn) => {
                let (data_tx, data_rx) = mpsc::channel::<Vec<u8>>();
                let (cmd_tx, cmd_rx) = mpsc::channel::<WorkerCmd>();

                std::thread::spawn(move || {
                    serial_worker(conn, data_tx, cmd_rx);
                });

                self.worker = Some(SerialWorker {
                    rx: data_rx,
                    tx: cmd_tx,
                });
                self.gui.conn = ConnState::Connected;
                self.gui.config = config;
                self.gui
                    .log(format!("Terhubung ke {port} ({})", config.label()));
                self.gui.status = format!("Connected: {port} {}", config.label());
            }
            Err(e) => {
                self.gui.error = Some(format!("gagal membuka {port}: {e}"));
                self.gui.log(format!("Gagal membuka {port}: {e}"));
            }
        }
    }

    fn disconnect(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = worker.tx.send(WorkerCmd::Close);
        }
        if self.gui.conn == ConnState::Connected {
            self.gui.log("Koneksi ditutup.");
        }
        self.gui.conn = ConnState::Disconnected;
    }

    fn scan(&mut self, port: String) {
        self.gui.conn = ConnState::Busy;
        self.gui.status = format!("Scanning {port}...");
        self.gui.log(format!("Memindai baudrate pada {port}..."));

        // Scan sinkron (durasi singkat). Untuk UX, ini memblokir sebentar.
        let mut source = uartrecon_core::detector::SerialSampleSource::new(&port);
        match uartrecon_core::detector::detect(
            &mut source,
            &uartrecon_core::detector::DetectOptions::default(),
        ) {
            Ok(result) => self.gui.set_scan_result(&result),
            Err(e) => {
                self.gui.error = Some(format!("scan gagal: {e}"));
                self.gui.log(format!("Scan gagal: {e}"));
            }
        }
        self.gui.conn = ConnState::Disconnected;
    }

    fn send(&mut self, text: String) {
        let Some(worker) = &self.worker else {
            self.gui.status = "Tidak terhubung.".to_string();
            return;
        };

        // Risk check read-only-first.
        match uartrecon_core::terminal::classify_risk(&text) {
            uartrecon_core::terminal::CommandRisk::Destructive => {
                self.gui.log(format!("DIBLOKIR (destructive): {text}"));
                self.gui.status = "Perintah destruktif diblokir.".to_string();
                return;
            }
            uartrecon_core::terminal::CommandRisk::StateChanging => {
                self.gui.log(format!("WARNING (state-changing): {text}"));
            }
            uartrecon_core::terminal::CommandRisk::ReadOnly => {}
        }

        let payload = format!("{text}\r\n").into_bytes();
        if worker.tx.send(WorkerCmd::Send(payload)).is_ok() {
            self.gui.recorder.push_tx(format!("{text}\r\n").as_bytes());
            self.gui.log(format!("TX: {text}"));
        }
    }

    fn save_session(&mut self, name: String) {
        let config = self.gui.config;
        let port = self
            .gui
            .selected_port
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        match uartrecon_core::capture::Session::create("sessions", name.clone(), port, config) {
            Ok(mut session) => match session.save(&self.gui.recorder) {
                Ok(()) => {
                    self.gui
                        .log(format!("Sesi disimpan: {}", session.paths.dir.display()));
                    self.gui.refresh_sessions("sessions");
                }
                Err(e) => self.gui.log(format!("Gagal menyimpan sesi: {e}")),
            },
            Err(e) => self.gui.log(format!("Gagal membuat sesi: {e}")),
        }
    }

    fn load_session(&mut self, path: std::path::PathBuf) {
        match uartrecon_core::capture::Session::open(&path) {
            Ok(session) => match session.read_rx() {
                Ok(rx) => {
                    self.gui.recorder = uartrecon_core::capture::Recorder::new();
                    self.gui.recorder.push_rx(&rx);
                    self.gui.update_fingerprint();
                    self.gui.log(format!(
                        "Sesi dimuat: {} ({} bytes RX)",
                        session.metadata.session,
                        rx.len()
                    ));
                    self.gui.tab = Tab::Terminal;
                }
                Err(e) => self.gui.log(format!("Gagal membaca RX: {e}")),
            },
            Err(e) => self.gui.log(format!("Gagal membuka sesi: {e}")),
        }
    }
}

/// Thread worker: baca serial & tangani perintah kirim/tutup.
fn serial_worker(mut conn: Connection, data_tx: Sender<Vec<u8>>, cmd_rx: Receiver<WorkerCmd>) {
    loop {
        // Baca data (timeout singkat).
        match conn.read_for(std::time::Duration::from_millis(50)) {
            Ok(data) if !data.is_empty() => {
                if data_tx.send(data).is_err() {
                    break;
                }
            }
            Ok(_) => {}
            Err(_) => break,
        }

        // Tangani perintah.
        loop {
            match cmd_rx.try_recv() {
                Ok(WorkerCmd::Send(bytes)) => {
                    let _ = conn.write(&bytes);
                }
                Ok(WorkerCmd::Close) => return,
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => return,
            }
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Tarik data serial.
        if let Some(worker) = &self.worker {
            self.gui.drain_serial(&worker.rx);
        }

        // Repaint berkelanjutan saat terhubung agar terminal live.
        if self.gui.conn == ConnState::Connected {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(50));
        }

        let mut pending = UiAction::None;

        // Panel atas: menu bar + judul + tab.
        egui::Panel::top("top").show(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Refresh port").clicked() {
                        self.gui.refresh_ports();
                    }
                    if ui.button("Refresh sesi").clicked() {
                        self.gui.refresh_sessions("sessions");
                        self.gui.show_toast("Daftar sesi diperbarui.");
                    }
                    ui.separator();
                    if ui.button("Keluar").clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Device", |ui| {
                    let connected = self.gui.conn == ConnState::Connected;
                    if ui
                        .add_enabled(!connected, egui::Button::new("Connect"))
                        .clicked()
                        && let Some(port) = self.gui.selected_port.clone()
                        && let (Some(baud), Some(fmt)) = (
                            app::parse_baud(&self.gui.baud_input),
                            app::parse_format(&self.gui.format_input),
                        )
                    {
                        pending =
                            UiAction::Connect(port, uartrecon_core::SerialConfig::new(baud, fmt));
                        ui.close();
                    }
                    if ui
                        .add_enabled(connected, egui::Button::new("Disconnect"))
                        .clicked()
                    {
                        pending = UiAction::Disconnect;
                        ui.close();
                    }
                    if ui
                        .add_enabled(!connected, egui::Button::new("Auto scan"))
                        .clicked()
                        && let Some(port) = self.gui.selected_port.clone()
                    {
                        pending = UiAction::Scan(port);
                        ui.close();
                    }
                });
                ui.menu_button("Bantuan", |ui| {
                    if ui.button("Doctor").clicked() {
                        self.gui
                            .log("Jalankan `uartrecon doctor` di CLI untuk cek environment.");
                        ui.close();
                    }
                    if ui.button("Tentang").clicked() {
                        self.gui
                            .log("UARTRecon v0.1.0 - UART Recon & Analysis Toolkit (MIT)");
                        self.gui
                            .log("read-only first - detect - capture - analyze - export");
                        ui.close();
                    }
                });

                // Menu bahasa.
                ui.menu_button(i18n::t(uartrecon_core::i18n::Key::Language), |ui| {
                    for l in uartrecon_core::i18n::Lang::ALL {
                        if ui.selectable_label(i18n::lang() == *l, l.name()).clicked() {
                            i18n::set_lang(*l);
                            // Simpan ke config.
                            let mut cfg = uartrecon_core::config::load();
                            cfg.language = l.code().to_string();
                            let _ = uartrecon_core::config::save(&cfg);
                            self.gui.log(format!("Bahasa diganti ke {}.", l.name()));
                            ui.close();
                        }
                    }
                });

                ui.separator();
                ui.heading("UARTRecon");
                ui.label(
                    egui::RichText::new("read-only first")
                        .italics()
                        .small()
                        .color(egui::Color32::from_rgb(80, 200, 120)),
                );

                // Indikator koneksi (kanan).
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (txt, col) = match self.gui.conn {
                        ConnState::Connected => {
                            ("● Connected", egui::Color32::from_rgb(80, 200, 120))
                        }
                        ConnState::Busy => ("● Busy", egui::Color32::from_rgb(230, 190, 80)),
                        ConnState::Disconnected => ("● Disconnected", egui::Color32::GRAY),
                    };
                    ui.label(egui::RichText::new(txt).color(col).strong());
                    if let Some(port) = &self.gui.selected_port {
                        ui.label(egui::RichText::new(port).monospace());
                    }
                });
            });

            // Baris tab.
            ui.horizontal(|ui| {
                for tab in Tab::ALL {
                    if ui
                        .selectable_label(self.gui.tab == *tab, format!("  {}  ", tab.title()))
                        .clicked()
                    {
                        self.gui.tab = *tab;
                    }
                }
            });
        });

        // Panel bawah: status + log.
        egui::Panel::bottom("bottom").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Status: {}", self.gui.status));
                ui.separator();
                ui.label(format!("{:.0} fps", self.fps.tick()));
                ui.separator();
                if let Some(err) = &self.gui.error {
                    ui.colored_label(egui::Color32::from_rgb(220, 100, 100), format!("⚠ {err}"));
                }
                if let Some(toast) = self.gui.active_toast() {
                    ui.separator();
                    ui.colored_label(egui::Color32::from_rgb(80, 200, 120), format!("✓ {toast}"));
                }
            });
            panels::log_panel(ui, &self.gui);
        });

        // Panel samping: kontrol + info + suggestions.
        egui::Panel::left("side")
            .resizable(true)
            .default_size(300.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let a = panels::control_panel(ui, &mut self.gui);
                    if !matches!(a, UiAction::None) {
                        pending = a;
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Sesi:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.gui.session_name)
                                .desired_width(140.0),
                        );
                        if ui.button("Simpan").clicked() {
                            pending = UiAction::SaveSession(self.gui.session_name.clone());
                        }
                    });

                    ui.separator();
                    ui.heading("Export");
                    ui.horizontal_wrapped(|ui| {
                        for fmt in app::ExportFormat::ALL {
                            if ui.button(fmt.label()).clicked() {
                                pending = UiAction::Export(*fmt);
                            }
                        }
                    });

                    ui.separator();
                    panels::suggestions_panel(ui, &mut self.gui);
                });
            });

        // Panel tengah: konten tab.
        egui::CentralPanel::default().show(ui, |ui| {
            let a = match self.gui.tab {
                Tab::Terminal => panels::terminal_tab(ui, &mut self.gui),
                Tab::Detection => panels::detection_tab(ui, &mut self.gui),
                Tab::Analysis => panels::analysis_tab(ui, &mut self.gui),
                Tab::Firmware => panels::firmware_tab(ui, &mut self.gui),
                Tab::Waveform => waveform::waveform_tab(ui, &mut self.gui),
                Tab::Sessions => panels::sessions_tab(ui, &mut self.gui),
            };
            if !matches!(a, UiAction::None) {
                pending = a;
            }
        });

        self.handle_action(pending);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.disconnect();
    }
}
