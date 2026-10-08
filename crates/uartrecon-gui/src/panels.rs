//! Rendering tiap tab GUI.

use egui::{Color32, RichText, Ui};

use uartrecon_core::detector::fingerprint::Confidence;

use crate::app::{ConnState, GuiApp, UiAction};

/// Warna untuk tingkat kepercayaan.
fn confidence_color(c: Confidence) -> Color32 {
    match c {
        Confidence::Detected => Color32::from_rgb(80, 200, 120),
        Confidence::Probable => Color32::from_rgb(230, 190, 80),
        Confidence::Unknown => Color32::GRAY,
    }
}

/// Panel kiri: kontrol koneksi.
pub fn control_panel(ui: &mut Ui, app: &mut GuiApp) -> UiAction {
    let mut action = UiAction::None;

    ui.heading("Koneksi");
    ui.separator();

    ui.horizontal(|ui| {
        ui.label("Port:");
        egui::ComboBox::from_id_salt("port_combo")
            .selected_text(
                app.selected_port
                    .clone()
                    .unwrap_or_else(|| "(tidak ada)".into()),
            )
            .show_ui(ui, |ui| {
                for p in app.ports.clone() {
                    let label = match p.usb_uart_chip() {
                        Some(chip) => format!("{} — {}", p.name, chip),
                        None => p.name.clone(),
                    };
                    ui.selectable_value(&mut app.selected_port, Some(p.name.clone()), label);
                }
            });
        if ui.button("↻").on_hover_text("Refresh port").clicked() {
            app.refresh_ports();
        }
    });

    ui.horizontal(|ui| {
        ui.label("Baud:");
        ui.add(egui::TextEdit::singleline(&mut app.baud_input).desired_width(80.0));
        ui.label("Format:");
        ui.add(egui::TextEdit::singleline(&mut app.format_input).desired_width(50.0));
    });

    ui.horizontal(|ui| {
        let connected = app.conn == ConnState::Connected;
        if connected {
            if ui.button("Disconnect").clicked() {
                action = UiAction::Disconnect;
            }
        } else {
            if ui.button("Connect").clicked()
                && let Some(port) = app.selected_port.clone()
                && let (Some(baud), Some(fmt)) = (
                    crate::app::parse_baud(&app.baud_input),
                    crate::app::parse_format(&app.format_input),
                )
            {
                action = UiAction::Connect(port, uartrecon_core::SerialConfig::new(baud, fmt));
            }
            if ui.button("Auto Scan").clicked()
                && let Some(port) = app.selected_port.clone()
            {
                action = UiAction::Scan(port);
            }
        }
    });

    ui.separator();
    ui.heading("Status");
    let (state_text, state_color) = match app.conn {
        ConnState::Connected => ("Connected", Color32::from_rgb(80, 200, 120)),
        ConnState::Busy => ("Busy...", Color32::from_rgb(230, 190, 80)),
        ConnState::Disconnected => ("Disconnected", Color32::GRAY),
    };
    ui.label(RichText::new(state_text).color(state_color).strong());
    ui.label(format!("RX: {} bytes", app.recorder.rx().len()));
    ui.label(format!("Confidence: {}/100", app.confidence));

    if let Some(fp) = &app.fingerprint {
        ui.separator();
        ui.heading("Device");
        device_summary(ui, fp);
    }

    action
}

/// Ringkasan device dari fingerprint.
pub fn device_summary(ui: &mut Ui, fp: &uartrecon_core::detector::Fingerprint) {
    egui::Grid::new("device_grid")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label("Bootloader");
            ui.label(
                RichText::new(format!(
                    "{} ({})",
                    fp.bootloader.name,
                    fp.bootloader.confidence.label()
                ))
                .color(confidence_color(fp.bootloader.confidence)),
            );
            ui.end_row();

            ui.label("OS");
            ui.label(
                RichText::new(format!("{} ({})", fp.os.name, fp.os.confidence.label()))
                    .color(confidence_color(fp.os.confidence)),
            );
            ui.end_row();

            ui.label("Shell");
            ui.label(
                RichText::new(format!(
                    "{} ({})",
                    fp.shell.name,
                    fp.shell.confidence.label()
                ))
                .color(confidence_color(fp.shell.confidence)),
            );
            ui.end_row();

            if let Some(ram) = fp.ram_bytes {
                ui.label("RAM");
                ui.label(uartrecon_core::util::human_size(ram));
                ui.end_row();
            }
            if let Some(storage) = fp.storage_bytes {
                ui.label("Storage");
                ui.label(format!(
                    "{} {}",
                    fp.storage_kind.as_deref().unwrap_or("storage"),
                    uartrecon_core::util::human_size(storage)
                ));
                ui.end_row();
            }
        });
}

/// Tab terminal.
pub fn terminal_tab(ui: &mut Ui, app: &mut GuiApp) -> UiAction {
    let mut action = UiAction::None;

    ui.horizontal(|ui| {
        ui.label("Mode:");
        for mode in [
            uartrecon_core::terminal::OutputMode::Ascii,
            uartrecon_core::terminal::OutputMode::Hex,
            uartrecon_core::terminal::OutputMode::Raw,
        ] {
            if ui
                .selectable_label(app.mode == mode, mode.label())
                .clicked()
            {
                app.mode = mode;
            }
        }
        ui.separator();
        if ui.button("Clear").clicked() {
            app.recorder = uartrecon_core::capture::Recorder::new();
            app.fingerprint = None;
            app.analysis = Default::default();
        }
        if ui.button("Fingerprint").clicked() {
            app.update_fingerprint();
        }
        if ui.button("Analisis").clicked() {
            app.analyze_buffer();
            app.tab = crate::app::Tab::Analysis;
        }
        if ui.button("Bootlog").clicked() {
            app.capture_bootlog();
        }
        ui.separator();
        // Statistik live.
        let rx = app.recorder.rx().len();
        let tx = app.recorder.tx().len();
        ui.label(
            RichText::new(format!("RX {rx} B · TX {tx} B"))
                .monospace()
                .color(Color32::from_rgb(120, 180, 255)),
        );
        let rate = app.rx_rate();
        if rate > 0.0 {
            ui.label(
                RichText::new(format!("{rate:.0} B/s"))
                    .monospace()
                    .color(Color32::from_rgb(80, 200, 120)),
            );
        }
    });
    ui.separator();

    let text = match app.mode {
        uartrecon_core::terminal::OutputMode::Ascii => {
            uartrecon_core::capture::exporter::to_text(app.recorder.rx())
        }
        uartrecon_core::terminal::OutputMode::Hex => {
            uartrecon_core::capture::exporter::to_hexdump(app.recorder.rx())
        }
        uartrecon_core::terminal::OutputMode::Raw => {
            String::from_utf8_lossy(app.recorder.rx()).to_string()
        }
    };

    // Tampilkan sebagai read-only agar tidak bisa di-edit tak sengaja.
    egui::ScrollArea::vertical()
        .stick_to_bottom(true)
        .auto_shrink([false, false])
        .max_height(ui.available_height() - 90.0)
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut text.as_str())
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY)
                    .code_editor(),
            );
        });

    ui.separator();

    // Quick commands (klik untuk mengisi input).
    ui.horizontal_wrapped(|ui| {
        ui.label("Quick:");
        for cmd in [
            "help",
            "uname -a",
            "cat /proc/mtd",
            "cat /proc/meminfo",
            "cat /proc/cpuinfo",
            "mount",
            "df -h",
            "ls /dev",
            "ps",
            "free",
        ] {
            if ui.small_button(cmd).clicked() {
                app.command_input = cmd.to_string();
            }
        }
    });

    ui.horizontal(|ui| {
        ui.label("Command:");
        let resp = ui.add(
            egui::TextEdit::singleline(&mut app.command_input)
                .hint_text("mis. cat /proc/mtd  (↑/↓ = riwayat)")
                .desired_width(340.0),
        );

        // Navigasi riwayat dengan panah.
        if resp.has_focus() {
            if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                app.history_prev();
            }
            if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                app.history_next();
            }
        }

        let send = ui.button("Send").clicked()
            || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
        if send && !app.command_input.trim().is_empty() {
            let cmd = std::mem::take(&mut app.command_input);
            app.push_history(&cmd);
            action = UiAction::Send(cmd);
        }
        if ui
            .button("Copy RX")
            .on_hover_text("Salin output RX ke clipboard")
            .clicked()
        {
            ui.ctx().copy_text(text.clone());
            app.show_toast("Output RX disalin ke clipboard.");
        }
    });

    action
}

/// Tab detection.
pub fn detection_tab(ui: &mut Ui, app: &mut GuiApp) -> UiAction {
    let mut action = UiAction::None;

    ui.horizontal(|ui| {
        if ui.button("Scan baudrate").clicked()
            && let Some(port) = app.selected_port.clone()
        {
            action = UiAction::Scan(port);
        }
        if ui.button("Fingerprint dari buffer").clicked() {
            app.update_fingerprint();
        }
    });
    ui.separator();

    if app.scan_report.is_empty() {
        ui.label("Belum ada hasil scan. Klik 'Scan baudrate'.");
    } else {
        ui.label(RichText::new("Ranking baudrate:").strong());
        for line in &app.scan_report {
            ui.monospace(line);
        }
        ui.separator();
        ui.label(format!("Status: {}", app.status));
    }

    if let Some(fp) = &app.fingerprint {
        ui.separator();
        ui.heading("Fingerprint");
        device_summary(ui, fp);
    }

    action
}

/// Tab sessions.
pub fn sessions_tab(ui: &mut Ui, app: &mut GuiApp) -> UiAction {
    let mut action = UiAction::None;

    ui.horizontal(|ui| {
        if ui.button("Refresh").clicked() {
            app.refresh_sessions("sessions");
        }
        ui.label(format!("{} sesi ditemukan", app.sessions.len()));
    });
    ui.separator();

    if app.sessions.is_empty() {
        ui.label("Belum ada sesi tersimpan di ./sessions");
        return action;
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        for entry in app.sessions.clone() {
            let name = entry
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&name).strong());
                    if ui.button("Muat").clicked() {
                        action = UiAction::LoadSession(entry.path.clone());
                    }
                });
                if let Some(md) = &entry.metadata {
                    ui.label(format!(
                        "  {} {}{}{}  ·  {}",
                        md.baudrate, md.data_bits, md.parity, md.stop_bits, md.timestamp_utc
                    ));
                }
            });
        }
    });

    action
}

/// Panel log di bawah.
pub fn log_panel(ui: &mut Ui, app: &GuiApp) {
    egui::ScrollArea::vertical()
        .stick_to_bottom(true)
        .max_height(120.0)
        .show(ui, |ui| {
            for line in &app.logs {
                ui.monospace(line);
            }
        });
}

/// Tab Analysis: analisis buffer RX (strings/entropy/signatures/stats/search).
pub fn analysis_tab(ui: &mut Ui, app: &mut GuiApp) -> UiAction {
    let action = UiAction::None;

    ui.horizontal(|ui| {
        if ui.button("🔍 Analisis buffer RX").clicked() {
            app.analyze_buffer();
        }
        if ui.button("Clear hasil").clicked() {
            app.analysis = Default::default();
        }
        ui.separator();
        ui.label(format!("RX: {} bytes", app.recorder.rx().len()));
    });
    ui.separator();

    // Sub-panel: search.
    ui.collapsing("🔎 Search", |ui| {
        ui.horizontal(|ui| {
            ui.label("Query:");
            ui.add(egui::TextEdit::singleline(&mut app.search.query).desired_width(200.0));
            ui.label("Mode:");
            egui::ComboBox::from_id_salt("search_mode")
                .selected_text(app.search.mode.clone())
                .show_ui(ui, |ui| {
                    for m in ["literal", "icase", "regex", "hex"] {
                        ui.selectable_value(&mut app.search.mode, m.to_string(), m);
                    }
                });
            if ui.button("Cari").clicked() {
                app.run_search();
            }
        });
        if let Some(err) = &app.search.error {
            ui.colored_label(Color32::from_rgb(220, 100, 100), err);
        }
        if !app.search.results.is_empty() {
            ui.label(format!("{} hasil:", app.search.results.len()));
            egui::ScrollArea::vertical()
                .id_salt("search_results")
                .max_height(120.0)
                .show(ui, |ui| {
                    for m in &app.search.results {
                        ui.monospace(format!("0x{:08X}  {}", m.offset, m.snippet));
                    }
                });
        }
    });

    let Some(cache) = analysis_cache_or_empty(app) else {
        ui.label("Belum ada hasil. Klik 'Analisis buffer RX'.");
        return action;
    };

    ui.label(
        RichText::new(format!("Sumber: {}", cache.source))
            .italics()
            .color(Color32::GRAY),
    );

    // Tabs internal hasil.
    ui.horizontal(|ui| {
        for (label, sel) in [
            ("Stats", AnalysisSection::Stats),
            ("Entropy", AnalysisSection::Entropy),
            ("Signatures", AnalysisSection::Signatures),
            ("Strings", AnalysisSection::Strings),
            ("Interesting", AnalysisSection::Interesting),
        ] {
            if ui
                .selectable_label(app.analysis_section == sel, label)
                .clicked()
            {
                app.analysis_section = sel;
            }
        }
    });
    ui.separator();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            render_analysis_section(ui, &cache);
        });

    action
}

/// Bagian hasil analisis yang ditampilkan.
pub use crate::app::AnalysisSection;

/// Mengambil cache analisis (clone) bila ada isinya.
fn analysis_cache_or_empty(app: &GuiApp) -> Option<crate::app::AnalysisCache> {
    if app.analysis.stats.is_some() {
        Some(app.analysis.clone())
    } else {
        None
    }
}

/// Merender satu bagian analisis.
fn render_analysis_section(ui: &mut Ui, cache: &crate::app::AnalysisCache) {
    match cache {
        _ if cache.stats.is_none() => {
            ui.label("Tidak ada data.");
        }
        _ => {
            // Render semua bagian secara berurutan (cukup ringkas).
            if let Some(s) = &cache.stats {
                ui.heading("Statistik");
                egui::Grid::new("stats_grid").num_columns(2).show(ui, |ui| {
                    ui.label("Total");
                    ui.label(s.total.to_string());
                    ui.end_row();
                    ui.label("Unique");
                    ui.label(s.unique_bytes.to_string());
                    ui.end_row();
                    ui.label("Printable");
                    ui.label(format!("{:.1}%", s.printable_ratio() * 100.0));
                    ui.end_row();
                    if let Some((b, c)) = s.most_common {
                        ui.label("Most common");
                        ui.label(format!("0x{b:02X} ({c}x)"));
                        ui.end_row();
                    }
                });
                ui.separator();
            }
            if let Some(e) = &cache.entropy {
                ui.heading("Entropy");
                ui.label(format!("{:.4} bit/byte — {}", e.overall, e.class.label()));
                ui.separator();
            }
            if let Some(sigs) = &cache.signatures {
                ui.heading(format!("Signatures ({})", sigs.len()));
                for s in sigs.iter().take(50) {
                    ui.monospace(format!(
                        "0x{:08X}  [{}] {}{}",
                        s.offset,
                        s.category,
                        s.name,
                        if s.weak { " (weak)" } else { "" }
                    ));
                }
                ui.separator();
            }
            if let Some(strs) = &cache.strings {
                ui.heading(format!("Strings ({})", strs.len()));
                for s in strs.iter().take(100) {
                    ui.monospace(format!("0x{:08X}  {}", s.offset, s.value));
                }
                ui.separator();
            }
            if let Some(int) = &cache.interesting {
                ui.heading(format!("Interesting ({})", int.len()));
                for (s, desc) in int.iter().take(50) {
                    ui.monospace(format!("0x{:08X}  [{}] {}", s.offset, desc, s.value));
                }
            }
        }
    }
}

/// Tab Firmware: muat file + analisis.
pub fn firmware_tab(ui: &mut Ui, app: &mut GuiApp) -> UiAction {
    let action = UiAction::None;

    ui.horizontal(|ui| {
        if ui.button("📂 Buka file firmware").clicked()
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("Semua file", &["*"])
                .add_filter("Binary", &["bin", "img", "fw", "raw"])
                .pick_file()
        {
            app.load_firmware(&path);
        }
        if ui.button("📄 Buka sesi sebagai firmware").clicked()
            && let Some(dir) = rfd::FileDialog::new().pick_folder()
        {
            let rx = dir.join("rx.raw");
            if rx.exists() {
                app.load_firmware(&rx);
            } else {
                app.error = Some("rx.raw tidak ditemukan di folder sesi.".to_string());
            }
        }
    });
    ui.separator();

    let Some(fw) = app.firmware.clone() else {
        ui.label("Belum ada firmware dimuat. Klik 'Buka file firmware'.");
        ui.add_space(8.0);
        ui.label(
            RichText::new(
                "Fitur: signature scanner (binwalk-like), entropy, strings, stats, search.",
            )
            .italics()
            .color(Color32::GRAY),
        );
        return action;
    };

    egui::Grid::new("fw_grid").num_columns(2).show(ui, |ui| {
        ui.label("File");
        ui.label(RichText::new(&fw.path).strong());
        ui.end_row();
        ui.label("Size");
        ui.label(uartrecon_core::util::human_size(fw.size as u64));
        ui.end_row();
        ui.label("SHA-256");
        ui.monospace(&fw.sha256);
        ui.end_row();
    });

    // Ekspor data firmware ke file.
    ui.horizontal(|ui| {
        if ui.button("💾 Ekspor raw").clicked()
            && let Some(path) = rfd::FileDialog::new()
                .set_file_name("firmware_export.bin")
                .save_file()
        {
            match std::fs::write(&path, &fw.data) {
                Ok(()) => app.log(format!("Firmware diekspor ke {}", path.display())),
                Err(e) => app.error = Some(format!("export gagal: {e}")),
            }
        }
        if ui.button("🔎 Analisis di tab Analysis").clicked() {
            // Salin data firmware ke buffer RX agar bisa dianalisis di tab Analysis.
            app.recorder = uartrecon_core::capture::Recorder::new();
            app.recorder.push_rx(&fw.data);
            app.analyze_buffer();
            app.tab = crate::app::Tab::Analysis;
        }
    });
    ui.separator();

    if let Some(sigs) = &fw.analysis.signatures {
        ui.heading(format!("Signatures ({})", sigs.len()));
        egui::ScrollArea::vertical()
            .id_salt("fw_sigs")
            .max_height(200.0)
            .show(ui, |ui| {
                for s in sigs {
                    ui.monospace(format!(
                        "0x{:08X}  [{}] {}{}",
                        s.offset,
                        s.category,
                        s.name,
                        if s.weak { " (weak)" } else { "" }
                    ));
                }
            });
    }
    if let Some(e) = &fw.analysis.entropy {
        ui.separator();
        ui.heading("Entropy");
        ui.label(format!("{:.4} bit/byte — {}", e.overall, e.class.label()));
    }
    if let Some(int) = &fw.analysis.interesting {
        ui.separator();
        ui.heading(format!("Interesting strings ({})", int.len()));
        egui::ScrollArea::vertical()
            .id_salt("fw_interesting")
            .max_height(200.0)
            .show(ui, |ui| {
                for (s, desc) in int {
                    ui.monospace(format!("0x{:08X}  [{}] {}", s.offset, desc, s.value));
                }
            });
    }

    action
}

/// Panel command suggestions (read-only, klik untuk isi command).
pub fn suggestions_panel(ui: &mut Ui, app: &mut GuiApp) {
    ui.collapsing("💡 Saran command (read-only)", |ui| {
        let cmds = crate::app::suggested_commands(app.fingerprint.as_ref());
        egui::ScrollArea::vertical()
            .id_salt("suggestions")
            .max_height(200.0)
            .show(ui, |ui| {
                let mut last_cat = "";
                for (cat, cmd, desc) in cmds {
                    if cat != last_cat {
                        ui.label(
                            RichText::new(cat)
                                .strong()
                                .color(Color32::from_rgb(120, 180, 255)),
                        );
                        last_cat = cat;
                    }
                    let resp = ui
                        .add(
                            egui::Button::new(
                                RichText::new(cmd)
                                    .monospace()
                                    .color(Color32::from_rgb(80, 200, 120)),
                            )
                            .fill(Color32::from_rgb(30, 35, 45)),
                        )
                        .on_hover_text(desc);
                    if resp.clicked() {
                        app.command_input = cmd.to_string();
                    }
                }
            });
    });
}
