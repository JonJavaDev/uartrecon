//! Panel waveform viewer (logic analyzer).

use egui::{Color32, RichText, Ui};
use egui_plot::{Line, Plot, PlotPoints};

use crate::app::{GuiApp, UiAction};

/// Tab waveform: plot sinyal digital + hasil analisis.
pub fn waveform_tab(ui: &mut Ui, app: &mut GuiApp) -> UiAction {
    let action = UiAction::None;

    ui.horizontal(|ui| {
        if ui.button("Muat waveform (file)").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Waveform", &["raw", "bin", "csv", "txt"])
                .pick_file()
            {
                // Asumsi default 921600 Hz; user bisa ubah lewat slider di bawah.
                app.load_waveform_from_file(&path, 921_600);
            }
        }
        if ui.button("Demo waveform").clicked() {
            app.load_demo_waveform();
        }
    });
    ui.separator();

    let Some(view) = app.waveform.clone() else {
        ui.label("Belum ada waveform. Muat file atau pakai 'Demo waveform'.");
        return action;
    };

    // Info ringkas.
    ui.horizontal(|ui| {
        ui.label(format!("Sample rate: {} Hz", view.waveform.sample_rate));
        ui.label(format!("Sampel: {}", view.waveform.samples.len()));
        ui.label(format!("Durasi: {:.2} ms", view.waveform.duration_ms()));
        if let Some(src) = &view.source {
            ui.label(RichText::new(format!("Sumber: {src}")).italics());
        }
    });

    // Hasil analisis.
    if let Some(analysis) = &view.analysis {
        ui.separator();
        egui::Grid::new("wave_grid")
            .num_columns(2)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                ui.label("Edges");
                ui.label(format!("{}", analysis.edge_count));
                ui.end_row();

                if let Some(p) = &analysis.pulse {
                    ui.label("Pulsa (count)");
                    ui.label(format!("{}", p.count));
                    ui.end_row();
                    ui.label("Lebar min/max");
                    ui.label(format!(
                        "{:.2} / {:.2} µs",
                        p.min_secs * 1e6,
                        p.max_secs * 1e6
                    ));
                    ui.end_row();
                }

                if let Some(b) = &analysis.baud {
                    ui.label("Bit period");
                    ui.label(format!("{:.3} µs", b.bit_period_secs * 1e6));
                    ui.end_row();
                    ui.label("Baudrate (fisik)");
                    ui.label(
                        RichText::new(format!(
                            "{} (raw {:.0}, error {:.1}%)",
                            b.baudrate,
                            b.raw_baud,
                            b.error_ratio * 100.0
                        ))
                        .strong()
                        .color(Color32::from_rgb(80, 200, 120)),
                    );
                    ui.end_row();
                }

                if let Some(d) = &analysis.decoded {
                    ui.label("Decoded bytes");
                    ui.label(format!("{} ({} error)", d.bytes.len(), d.error_count));
                    ui.end_row();
                }
            });

        if let Some(d) = &view.analysis.as_ref().unwrap().decoded {
            ui.separator();
            ui.label(RichText::new("Decoded text:").strong());
            egui::ScrollArea::vertical()
                .max_height(120.0)
                .show(ui, |ui| {
                    ui.monospace(d.text());
                });
        }
    }

    ui.separator();
    ui.label(RichText::new("Waveform:").strong());

    // Plot sinyal (dibatasi jumlah sampel agar tetap responsif).
    let max_points = 20_000usize;
    let samples = &view.waveform.samples;
    let step = (samples.len() / max_points).max(1);
    let points: Vec<[f64; 2]> = samples
        .iter()
        .enumerate()
        .step_by(step)
        .map(|(i, &s)| [i as f64, (s & 1) as f64])
        .collect();

    let line = Line::new("RX".to_string(), PlotPoints::from(points))
        .color(Color32::from_rgb(90, 180, 255))
        .width(1.0);

    Plot::new("waveform_plot")
        .height(ui.available_height().max(150.0) - 10.0)
        .allow_zoom(true)
        .allow_drag(true)
        .show_axes([true, true])
        .y_axis_min_width(20.0)
        .show(ui, |plot_ui| {
            plot_ui.line(line);
        });

    action
}
