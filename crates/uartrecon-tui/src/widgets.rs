//! Widget rendering untuk TUI.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use uartrecon_core::detector::fingerprint::Confidence;
use uartrecon_core::terminal::OutputMode;

use crate::app::{App, Panel};

/// Merender seluruh UI.
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(5),    // body
            Constraint::Length(3), // footer/help
        ])
        .split(area);

    draw_header(frame, chunks[0], app);

    // Body: panel kiri (terminal/waveform) + kanan (info/log).
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(chunks[1]);

    if app.panel == Panel::Waveform {
        draw_waveform(frame, body[0], app);
    } else {
        draw_terminal(frame, body[0], app);
    }
    draw_side(frame, body[1], app);

    draw_footer(frame, chunks[2], app);
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let title = format!(
        " UARTRecon  │  {}  │  {}  │  confidence {}/100  │  mode {} ",
        app.port,
        app.config.label(),
        app.confidence,
        app.mode.label()
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(mode_color(app.mode)))
        .title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let hint = Paragraph::new(Line::from(vec![
        Span::styled("read-only first", Style::default().fg(Color::Green)),
        Span::raw("  ·  Ctrl+C keluar  ·  Tab ganti panel  ·  h/l mode  ·  j/k scroll"),
    ]));
    frame.render_widget(hint, inner);
}

fn draw_terminal(frame: &mut Frame, area: Rect, app: &App) {
    let active = app.panel == Panel::Terminal;
    let border_color = if active {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(format!(" {} ", Panel::Terminal.title()));

    let text = app.buffer.render();
    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0));
    frame.render_widget(paragraph, area);
}

/// Merender panel waveform (logic analyzer).
fn draw_waveform(frame: &mut Frame, area: Rect, app: &App) {
    let active = app.panel == Panel::Waveform;
    let border_color = if active {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(format!(" {} ", Panel::Waveform.title()));

    let mut lines: Vec<Line> = Vec::new();

    if let Some(view) = &app.waveform {
        lines.push(Line::from(Span::styled(
            view.source.clone(),
            Style::default().fg(Color::DarkGray),
        )));
        if let Some(analysis) = &view.analysis {
            if let Some(b) = &analysis.baud {
                lines.push(Line::from(vec![
                    Span::styled("Bit period : ", Style::default().fg(Color::Gray)),
                    Span::raw(format!("{:.3} µs", b.bit_period_secs * 1e6)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("Baud fisik : ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("{} (err {:.1}%)", b.baudrate, b.error_ratio * 100.0),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            }
            lines.push(Line::from(vec![
                Span::styled("Edges      : ", Style::default().fg(Color::Gray)),
                Span::raw(analysis.edge_count.to_string()),
            ]));
            if let Some(d) = &analysis.decoded {
                lines.push(Line::from(vec![
                    Span::styled("Decoded    : ", Style::default().fg(Color::Gray)),
                    Span::raw(format!("{} bytes ({} err)", d.bytes.len(), d.error_count)),
                ]));
            }
        }
        lines.push(Line::from(""));

        // Sparkline digital: baris atas = level high, bawah = level low.
        let width = area.width.saturating_sub(4) as usize;
        let samples = &view.waveform.samples;
        if width > 0 && !samples.is_empty() {
            let step = (samples.len() / width).max(1);
            let mut high = String::new();
            let mut low = String::new();
            for i in (0..samples.len()).step_by(step).take(width) {
                if samples[i] & 1 == 1 {
                    high.push('▀');
                    low.push(' ');
                } else {
                    high.push(' ');
                    low.push('▄');
                }
            }
            lines.push(Line::from(Span::styled(
                high,
                Style::default().fg(Color::Cyan),
            )));
            lines.push(Line::from(Span::styled(
                low,
                Style::default().fg(Color::Blue),
            )));
        }

        if let Some(d) = view.analysis.as_ref().and_then(|a| a.decoded.as_ref()) {
            lines.push(Line::from(""));
            let preview: String = d.text().chars().take(300).collect();
            for l in preview.lines().take(6) {
                lines.push(Line::from(l.to_string()));
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            "Tekan 'w' untuk memuat waveform demo (logic analyzer).",
            Style::default().fg(Color::DarkGray),
        )));
    }

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn draw_side(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    draw_info(frame, chunks[0], app);
    draw_log(frame, chunks[1], app);
}

fn draw_info(frame: &mut Frame, area: Rect, app: &App) {
    let active = app.panel == Panel::Info;
    let border_color = if active {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(format!(" {} ", Panel::Info.title()));

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled("Port   : ", Style::default().fg(Color::Gray)),
        Span::raw(app.port.clone()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Config : ", Style::default().fg(Color::Gray)),
        Span::raw(app.config.label()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("RX     : ", Style::default().fg(Color::Gray)),
        Span::raw(format!("{} bytes", app.recorder.rx().len())),
    ]));

    if let Some(fp) = &app.fingerprint {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("BOOTLOADER: ", Style::default().fg(Color::Gray)),
            confidence_span(&fp.bootloader.name, fp.bootloader.confidence),
        ]));
        lines.push(Line::from(vec![
            Span::styled("OS        : ", Style::default().fg(Color::Gray)),
            confidence_span(&fp.os.name, fp.os.confidence),
        ]));
        lines.push(Line::from(vec![
            Span::styled("SHELL     : ", Style::default().fg(Color::Gray)),
            confidence_span(&fp.shell.name, fp.shell.confidence),
        ]));
        if let Some(ram) = fp.ram_bytes {
            lines.push(Line::from(vec![
                Span::styled("RAM       : ", Style::default().fg(Color::Gray)),
                Span::raw(uartrecon_core::util::human_size(ram)),
            ]));
        }
        if let Some(storage) = fp.storage_bytes {
            lines.push(Line::from(vec![
                Span::styled("STORAGE   : ", Style::default().fg(Color::Gray)),
                Span::raw(format!(
                    "{} {}",
                    fp.storage_kind.as_deref().unwrap_or("storage"),
                    uartrecon_core::util::human_size(storage)
                )),
            ]));
        }
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "(tekan 'f' untuk fingerprint)",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn draw_log(frame: &mut Frame, area: Rect, app: &App) {
    let active = app.panel == Panel::Log;
    let border_color = if active {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(format!(" {} ", Panel::Log.title()));

    let lines: Vec<Line> = app
        .logs
        .iter()
        .rev()
        .take(area.height.saturating_sub(2) as usize)
        .rev()
        .map(|l| Line::from(l.clone()))
        .collect();
    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let text = format!(
        " Panel: {}  │  Mode: {}  │  {}  ",
        app.panel.title(),
        app.mode.label(),
        if app.follow { "follow" } else { "scroll" }
    );
    frame.render_widget(Paragraph::new(text).block(block), area);
}

fn confidence_span(name: &str, confidence: Confidence) -> Span<'static> {
    let (color, label) = match confidence {
        Confidence::Detected => (Color::Green, "Detected"),
        Confidence::Probable => (Color::Yellow, "Probable"),
        Confidence::Unknown => (Color::DarkGray, "Unknown"),
    };
    Span::styled(
        format!("{name} ({label})"),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )
}

/// Warna default untuk mode (dipakai di header).
pub fn mode_color(mode: OutputMode) -> Color {
    match mode {
        OutputMode::Ascii => Color::Green,
        OutputMode::Hex => Color::Cyan,
        OutputMode::Raw => Color::Magenta,
    }
}
