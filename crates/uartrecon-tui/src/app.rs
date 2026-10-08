//! State aplikasi TUI.

use uartrecon_core::capture::Recorder;
use uartrecon_core::detector::fingerprint::Fingerprint;
use uartrecon_core::serial::config::SerialConfig;
use uartrecon_core::terminal::OutputMode;

/// Panel aktif di TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    /// Panel terminal (data UART).
    Terminal,
    /// Panel info device.
    Info,
    /// Panel log.
    Log,
    /// Panel waveform (logic analyzer).
    Waveform,
}

impl Panel {
    /// Panel berikutnya.
    pub fn next(self) -> Self {
        match self {
            Panel::Terminal => Panel::Info,
            Panel::Info => Panel::Log,
            Panel::Log => Panel::Waveform,
            Panel::Waveform => Panel::Terminal,
        }
    }

    /// Panel sebelumnya.
    pub fn prev(self) -> Self {
        match self {
            Panel::Terminal => Panel::Waveform,
            Panel::Info => Panel::Terminal,
            Panel::Log => Panel::Info,
            Panel::Waveform => Panel::Log,
        }
    }

    /// Judul panel.
    pub fn title(self) -> &'static str {
        match self {
            Panel::Terminal => "Terminal",
            Panel::Info => "Device Info",
            Panel::Log => "Log",
            Panel::Waveform => "Waveform",
        }
    }
}

/// State aplikasi TUI.
pub struct App {
    /// Nama port.
    pub port: String,
    /// Konfigurasi UART aktif.
    pub config: SerialConfig,
    /// Confidence deteksi (0..100).
    pub confidence: u8,
    /// Mode tampilan.
    pub mode: OutputMode,
    /// Buffer data (dari core console buffer).
    pub buffer: uartrecon_core::terminal::ConsoleBuffer,
    /// Recorder untuk capture.
    pub recorder: Recorder,
    /// Fingerprint terakhir (bila ada).
    pub fingerprint: Option<Fingerprint>,
    /// Baris log aplikasi.
    pub logs: Vec<String>,
    /// Panel aktif.
    pub panel: Panel,
    /// Offset scroll (0 = paling bawah / auto-scroll).
    pub scroll: u16,
    /// Apakah harus keluar.
    pub should_quit: bool,
    /// Apakah auto-scroll aktif.
    pub follow: bool,
    /// Waveform untuk panel logic analyzer.
    pub waveform: Option<WaveformView>,
}

/// Data waveform untuk panel TUI.
#[derive(Debug, Clone)]
pub struct WaveformView {
    /// Waveform.
    pub waveform: uartrecon_core::logic::Waveform,
    /// Hasil analisis.
    pub analysis: Option<uartrecon_core::logic::AnalysisResult>,
    /// Sumber.
    pub source: String,
}

impl App {
    /// Membuat app baru untuk sebuah port & config.
    pub fn new(port: impl Into<String>, config: SerialConfig, confidence: u8) -> Self {
        App {
            port: port.into(),
            config,
            confidence,
            mode: OutputMode::Ascii,
            buffer: uartrecon_core::terminal::ConsoleBuffer::new(2 * 1024 * 1024),
            recorder: Recorder::new(),
            fingerprint: None,
            logs: vec!["UARTRecon TUI siap.".to_string()],
            panel: Panel::Terminal,
            scroll: 0,
            should_quit: false,
            follow: true,
            waveform: None,
        }
    }

    /// Membuat waveform demo sintetis untuk panel logic analyzer.
    pub fn load_demo_waveform(&mut self) {
        use uartrecon_core::logic;
        use uartrecon_core::serial::config::SerialFormat;
        let payload = b"U-Boot 2021.10\r\nDRAM: 512 MiB\r\nNAND: 256 MiB\r\nLinux version 5.10\r\nBusyBox\r\nlogin: ";
        let wave = logic::synth_uart(payload, 115_200, 8, SerialFormat::EIGHT_N_ONE);
        let analysis = logic::analyze(&wave, Some(SerialFormat::EIGHT_N_ONE)).ok();
        self.waveform = Some(WaveformView {
            waveform: wave,
            analysis,
            source: "(demo sintetis 115200 8N1)".to_string(),
        });
        self.log("Waveform demo dimuat (logic analyzer).");
    }

    /// Menambahkan data RX.
    pub fn push_rx(&mut self, data: &[u8]) {
        self.buffer.push(data);
        self.recorder.push_rx(data);
    }

    /// Menambahkan baris log.
    pub fn log(&mut self, msg: impl Into<String>) {
        self.logs.push(msg.into());
        if self.logs.len() > 1000 {
            self.logs.remove(0);
        }
    }

    /// Menjalankan fingerprinting terhadap data yang terekam.
    pub fn update_fingerprint(&mut self) {
        let data = self.recorder.rx();
        if !data.is_empty() {
            self.fingerprint = Some(uartrecon_core::detector::fingerprint::fingerprint(data));
        }
    }

    /// Scroll ke atas.
    pub fn scroll_up(&mut self, n: u16) {
        self.follow = false;
        self.scroll = self.scroll.saturating_add(n);
    }

    /// Scroll ke bawah.
    pub fn scroll_down(&mut self, n: u16) {
        self.scroll = self.scroll.saturating_sub(n);
        if self.scroll == 0 {
            self.follow = true;
        }
    }
}
