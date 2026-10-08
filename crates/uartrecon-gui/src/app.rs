//! State aplikasi GUI UARTRecon.

use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use uartrecon_core::capture::Recorder;
use uartrecon_core::detector::fingerprint::Fingerprint;
use uartrecon_core::logic::{self, Waveform};
use uartrecon_core::serial::config::{SerialConfig, SerialFormat};
use uartrecon_core::terminal::OutputMode;

/// Tab utama GUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    /// Terminal data UART.
    Terminal,
    /// Deteksi (scan baudrate & format).
    Detection,
    /// Waveform viewer (logic analyzer).
    Waveform,
    /// Browser sesi tersimpan.
    Sessions,
}

impl Tab {
    /// Semua tab.
    pub const ALL: &'static [Tab] = &[Tab::Terminal, Tab::Detection, Tab::Waveform, Tab::Sessions];

    /// Judul tab.
    pub fn title(self) -> &'static str {
        match self {
            Tab::Terminal => "Terminal",
            Tab::Detection => "Detection",
            Tab::Waveform => "Waveform",
            Tab::Sessions => "Sessions",
        }
    }
}

/// Status koneksi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnState {
    /// Belum terhubung.
    Disconnected,
    /// Sedang terhubung.
    Connected,
    /// Sedang melakukan operasi (scan).
    Busy,
}

/// Aksi yang diminta UI (diproses oleh event loop utama).
#[derive(Debug, Clone)]
pub enum UiAction {
    /// Tidak ada.
    None,
    /// Hubungkan ke port.
    Connect(String, SerialConfig),
    /// Putuskan koneksi.
    Disconnect,
    /// Mulai scan baudrate.
    Scan(String),
    /// Kirim teks ke device.
    Send(String),
    /// Simpan sesi.
    SaveSession(String),
    /// Muat sesi dari direktori.
    LoadSession(std::path::PathBuf),
}

/// Data waveform untuk ditampilkan di plot.
#[derive(Debug, Clone)]
pub struct WaveformView {
    /// Waveform aktif.
    pub waveform: Waveform,
    /// Hasil analisis (edge, baud, decode).
    pub analysis: Option<logic::AnalysisResult>,
    /// Path sumber (bila dimuat dari file).
    pub source: Option<String>,
}

/// State aplikasi GUI.
pub struct GuiApp {
    /// Tab aktif.
    pub tab: Tab,
    /// Status koneksi.
    pub conn: ConnState,
    /// Daftar port yang tersedia.
    pub ports: Vec<uartrecon_core::SerialPortInfo>,
    /// Port terpilih.
    pub selected_port: Option<String>,
    /// Baudrate input (string agar bisa diketik bebas).
    pub baud_input: String,
    /// Format input (mis. `8N1`).
    pub format_input: String,
    /// Konfigurasi aktif.
    pub config: SerialConfig,
    /// Mode tampilan terminal.
    pub mode: OutputMode,
    /// Buffer terminal (RX).
    pub recorder: Recorder,
    /// Baris log aplikasi.
    pub logs: Vec<String>,
    /// Hasil fingerprint terakhir.
    pub fingerprint: Option<Fingerprint>,
    /// Confidence deteksi.
    pub confidence: u8,
    /// Ranking baudrate dari scan terakhir (teks).
    pub scan_report: Vec<String>,
    /// Waveform view.
    pub waveform: Option<WaveformView>,
    /// Daftar sesi yang ditemukan.
    pub sessions: Vec<SessionEntry>,
    /// Status pesan (footer).
    pub status: String,
    /// Pesan error terakhir.
    pub error: Option<String>,
    /// Input command manual.
    pub command_input: String,
    /// Nama sesi untuk disimpan.
    pub session_name: String,
}

/// Entri sesi di browser.
#[derive(Debug, Clone)]
pub struct SessionEntry {
    /// Path direktori sesi.
    pub path: std::path::PathBuf,
    /// Metadata (bila berhasil dibaca).
    pub metadata: Option<uartrecon_core::capture::CaptureMetadata>,
}

impl GuiApp {
    /// Membuat app baru.
    pub fn new() -> Self {
        let mut app = GuiApp {
            tab: Tab::Terminal,
            conn: ConnState::Disconnected,
            ports: Vec::new(),
            selected_port: None,
            baud_input: "115200".to_string(),
            format_input: "8N1".to_string(),
            config: SerialConfig::default(),
            mode: OutputMode::Ascii,
            recorder: Recorder::new(),
            logs: vec!["UARTRecon GUI siap.".to_string()],
            fingerprint: None,
            confidence: 0,
            scan_report: Vec::new(),
            waveform: None,
            sessions: Vec::new(),
            status: "Siap.".to_string(),
            error: None,
            command_input: String::new(),
            session_name: uartrecon_core::capture::Session::default_name("gui"),
        };
        app.refresh_ports();
        app.refresh_sessions("sessions");
        app
    }

    /// Memuat ulang daftar port.
    pub fn refresh_ports(&mut self) {
        match uartrecon_core::serial::ports::list_ports() {
            Ok(list) => {
                self.ports = list;
                if self.selected_port.is_none()
                    && let Some(first) = self.ports.first()
                {
                    self.selected_port = Some(first.name.clone());
                }
                self.log(format!("{} serial port terdeteksi.", self.ports.len()));
            }
            Err(e) => {
                self.error = Some(format!("gagal enumerasi port: {e}"));
            }
        }
    }

    /// Memuat ulang daftar sesi dari direktori.
    pub fn refresh_sessions(&mut self, root: &str) {
        let mut entries = Vec::new();
        if let Ok(rd) = std::fs::read_dir(root) {
            for e in rd.flatten() {
                let path = e.path();
                if path.is_dir() {
                    let metadata = uartrecon_core::capture::Session::open(&path)
                        .ok()
                        .map(|s| s.metadata);
                    entries.push(SessionEntry { path, metadata });
                }
            }
        }
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        self.sessions = entries;
    }

    /// Menambahkan data RX.
    pub fn push_rx(&mut self, data: &[u8]) {
        self.recorder.push_rx(data);
    }

    /// Menambahkan baris log.
    pub fn log(&mut self, msg: impl Into<String>) {
        self.logs.push(msg.into());
        if self.logs.len() > 500 {
            self.logs.remove(0);
        }
    }

    /// Menjalankan fingerprinting pada data terekam.
    pub fn update_fingerprint(&mut self) {
        let data = self.recorder.rx();
        if !data.is_empty() {
            self.fingerprint = Some(uartrecon_core::detector::fingerprint::fingerprint(data));
            self.log("Fingerprint diperbarui.");
        } else {
            self.status = "Tidak ada data untuk fingerprint.".to_string();
        }
    }

    /// Memuat waveform dari file (format: CSV `index,level` atau biner).
    pub fn load_waveform_from_file(&mut self, path: &std::path::Path, sample_rate: u32) {
        match std::fs::read(path) {
            Ok(data) => {
                let samples: Vec<u8> = if data.iter().any(|&b| b == b',' || b == b'\n') {
                    // Coba parse CSV: kolom terakhir = level.
                    String::from_utf8_lossy(&data)
                        .lines()
                        .filter_map(|l| l.split(',').next_back())
                        .filter_map(|s| s.trim().parse::<u8>().ok())
                        .collect()
                } else {
                    data.iter().map(|&b| b & 1).collect()
                };
                let wave = Waveform::new(sample_rate, samples);
                let analysis = logic::analyze(&wave, None).ok();
                self.waveform = Some(WaveformView {
                    waveform: wave,
                    analysis,
                    source: Some(path.display().to_string()),
                });
                self.log(format!("Waveform dimuat: {}", path.display()));
            }
            Err(e) => {
                self.error = Some(format!("gagal membaca waveform: {e}"));
            }
        }
    }

    /// Membuat waveform demo (UART sintetis) untuk pengujian tanpa hardware.
    pub fn load_demo_waveform(&mut self) {
        let payload = b"U-Boot 2021.10\r\nDRAM: 512 MiB\r\n";
        let wave = logic::synth_uart(payload, 115_200, 8, SerialFormat::EIGHT_N_ONE);
        let analysis = logic::analyze(&wave, Some(SerialFormat::EIGHT_N_ONE)).ok();
        self.waveform = Some(WaveformView {
            waveform: wave,
            analysis,
            source: Some("(demo sintetis)".to_string()),
        });
        self.log("Waveform demo dimuat (115200 8N1).");
    }

    /// Ringkasan hasil scan dalam beberapa baris.
    pub fn set_scan_result(&mut self, result: &uartrecon_core::detector::DetectionResult) {
        let mut lines = Vec::new();
        for c in &result.baud_ranking {
            let mark = if Some(c.baudrate) == result.config.map(|cfg| cfg.baudrate) {
                " <--"
            } else {
                ""
            };
            lines.push(format!(
                "{:<9} {:>3.0}  ({} bytes){}",
                c.baudrate,
                c.score(),
                c.bytes_read,
                mark
            ));
        }
        self.scan_report = lines;
        match result.config {
            Some(cfg) => {
                self.config = cfg;
                self.baud_input = cfg.baudrate.to_string();
                self.format_input = cfg.format.label();
                self.confidence = result.confidence;
                self.status = format!(
                    "Terdeteksi {} (confidence {}/100)",
                    cfg.label(),
                    result.confidence
                );
                self.log(self.status.clone());
            }
            None => {
                self.status = "No reliable traffic detected.".to_string();
                self.log("Tidak ada traffic yang dapat diandalkan.");
            }
        }
    }

    /// Membaca channel serial dan memasukkan ke buffer (dipanggil tiap frame).
    pub fn drain_serial(&mut self, rx: &Receiver<Vec<u8>>) -> usize {
        let mut total = 0;
        loop {
            match rx.try_recv() {
                Ok(data) => {
                    total += data.len();
                    self.push_rx(&data);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    if self.conn == ConnState::Connected {
                        self.conn = ConnState::Disconnected;
                        self.log("Koneksi serial terputus.");
                    }
                    break;
                }
            }
        }
        if total > 0 && self.conn == ConnState::Connected {
            self.update_fingerprint();
        }
        total
    }
}

impl Default for GuiApp {
    fn default() -> Self {
        Self::new()
    }
}

/// Utilitas: parse baudrate dari input string.
pub fn parse_baud(s: &str) -> Option<u32> {
    s.trim().parse().ok()
}

/// Utilitas: parse format dari input string.
pub fn parse_format(s: &str) -> Option<SerialFormat> {
    SerialFormat::parse(s).ok()
}

/// Menghitung FPS sederhana untuk status bar.
pub struct FpsCounter {
    last: Instant,
    frames: u32,
    fps: f32,
}

impl Default for FpsCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl FpsCounter {
    /// Membuat counter baru.
    pub fn new() -> Self {
        FpsCounter {
            last: Instant::now(),
            frames: 0,
            fps: 0.0,
        }
    }

    /// Menandai satu frame; mengembalikan FPS terbaru.
    pub fn tick(&mut self) -> f32 {
        self.frames += 1;
        let elapsed = self.last.elapsed();
        if elapsed >= Duration::from_millis(500) {
            self.fps = self.frames as f32 / elapsed.as_secs_f32();
            self.frames = 0;
            self.last = Instant::now();
        }
        self.fps
    }
}
