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
        if ui.button("Clear").clicked() {
            app.recorder = uartrecon_core::capture::Recorder::new();
            app.fingerprint = None;
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

    egui::ScrollArea::vertical()
        .stick_to_bottom(true)
        .auto_shrink([false, false])
        .max_height(ui.available_height() - 40.0)
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut text.as_str())
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY)
                    .code_editor(),
            );
        });

    ui.separator();
    ui.horizontal(|ui| {
        ui.label("Command:");
        let resp = ui.add(
            egui::TextEdit::singleline(&mut app.command_input)
                .hint_text("mis. cat /proc/mtd")
                .desired_width(300.0),
        );
        let send = ui.button("Send").clicked()
            || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
        if send && !app.command_input.trim().is_empty() {
            action = UiAction::Send(std::mem::take(&mut app.command_input));
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
