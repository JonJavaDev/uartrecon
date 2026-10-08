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
    /// Analisis buffer (strings/entropy/signatures/stats/search).
    Analysis,
    /// Analisis firmware (load file).
    Firmware,
    /// Waveform viewer (logic analyzer).
    Waveform,
    /// Browser sesi tersimpan.
    Sessions,
}

impl Tab {
    /// Semua tab.
    pub const ALL: &'static [Tab] = &[
        Tab::Terminal,
        Tab::Detection,
        Tab::Analysis,
        Tab::Firmware,
        Tab::Waveform,
        Tab::Sessions,
    ];

    /// Judul tab.
    pub fn title(self) -> &'static str {
        match self {
            Tab::Terminal => "Terminal",
            Tab::Detection => "Detection",
            Tab::Analysis => "Analysis",
            Tab::Firmware => "Firmware",
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
    /// Ekspor buffer ke format tertentu.
    Export(ExportFormat),
}

/// Format ekspor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// Teks (lossy UTF-8).
    Txt,
    /// Hexdump.
    Hex,
    /// JSON (metadata + analisis).
    Json,
    /// CSV (byte per baris).
    Csv,
    /// Raw biner.
    Raw,
}

impl ExportFormat {
    /// Semua format.
    pub const ALL: &[ExportFormat] = &[
        ExportFormat::Txt,
        ExportFormat::Hex,
        ExportFormat::Json,
        ExportFormat::Csv,
        ExportFormat::Raw,
    ];

    /// Label.
    pub fn label(self) -> &'static str {
        match self {
            ExportFormat::Txt => "TXT",
            ExportFormat::Hex => "HEX",
            ExportFormat::Json => "JSON",
            ExportFormat::Csv => "CSV",
            ExportFormat::Raw => "RAW",
        }
    }

    /// Ekstensi file.
    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Txt => "txt",
            ExportFormat::Hex => "hex",
            ExportFormat::Json => "json",
            ExportFormat::Csv => "csv",
            ExportFormat::Raw => "raw",
        }
    }
}

/// Hasil analisis buffer (di-cache agar tidak dihitung tiap frame).
#[derive(Debug, Clone, Default)]
pub struct AnalysisCache {
    /// Statistik byte.
    pub stats: Option<uartrecon_core::analyzers::Stats>,
    /// Entropi.
    pub entropy: Option<uartrecon_core::analyzers::EntropyReport>,
    /// Signatures.
    pub signatures: Option<Vec<uartrecon_core::analyzers::Signature>>,
    /// Strings.
    pub strings: Option<Vec<uartrecon_core::analyzers::ExtractedString>>,
    /// Strings menarik.
    pub interesting: Option<Vec<(uartrecon_core::analyzers::ExtractedString, &'static str)>>,
    /// Sumber data (untuk label).
    pub source: String,
}

/// Hasil pencarian dalam buffer.
#[derive(Debug, Clone, Default)]
pub struct SearchState {
    /// Query.
    pub query: String,
    /// Mode (literal/regex/hex/icase).
    pub mode: String,
    /// Hasil.
    pub results: Vec<uartrecon_core::capture::SearchMatch>,
    /// Pesan error (bila ada).
    pub error: Option<String>,
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
    /// Riwayat command.
    pub command_history: Vec<String>,
    /// Indeks riwayat (untuk navigasi up/down).
    pub history_index: Option<usize>,
    /// Nama sesi untuk disimpan.
    pub session_name: String,
    /// Cache hasil analisis buffer.
    pub analysis: AnalysisCache,
    /// State pencarian.
    pub search: SearchState,
    /// Data firmware yang dimuat (untuk tab Firmware).
    pub firmware: Option<FirmwareView>,
    /// Pesan toast sementara.
    pub toast: Option<(String, Instant)>,
    /// Bagian analisis yang aktif.
    pub analysis_section: AnalysisSection,
    /// Pelacak laju RX (byte terakhir & waktu).
    pub rate_tracker: RateTracker,
}

/// Pelacak laju data RX (untuk statistik live).
#[derive(Debug, Clone)]
pub struct RateTracker {
    last_bytes: usize,
    last_time: Instant,
    rate: f64,
}

impl Default for RateTracker {
    fn default() -> Self {
        RateTracker {
            last_bytes: 0,
            last_time: Instant::now(),
            rate: 0.0,
        }
    }
}

impl RateTracker {
    /// Memperbarui laju berdasarkan total byte saat ini.
    pub fn update(&mut self, total_bytes: usize) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_time).as_secs_f64();
        if dt >= 0.5 {
            let db = total_bytes.saturating_sub(self.last_bytes) as f64;
            // Smoothing (EMA).
            self.rate = if self.rate == 0.0 {
                db / dt
            } else {
                self.rate * 0.6 + (db / dt) * 0.4
            };
            self.last_bytes = total_bytes;
            self.last_time = now;
        }
    }

    /// Laju terakhir (byte/detik).
    pub fn rate(&self) -> f64 {
        self.rate
    }
}

/// Bagian hasil analisis yang ditampilkan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnalysisSection {
    /// Statistik.
    #[default]
    Stats,
    /// Entropi.
    Entropy,
    /// Signatures.
    Signatures,
    /// Strings.
    Strings,
    /// Strings menarik.
    Interesting,
}

/// Data firmware yang dimuat untuk analisis.
#[derive(Debug, Clone)]
pub struct FirmwareView {
    /// Path file.
    pub path: String,
    /// Data mentah.
    pub data: Vec<u8>,
    /// Ukuran.
    pub size: usize,
    /// SHA-256.
    pub sha256: String,
    /// Cache analisis.
    pub analysis: AnalysisCache,
}

/// Entri sesi di browser.
#[derive(Debug, Clone)]
pub struct SessionEntry {
    /// Path direktori sesi.
    pub path: std::path::PathBuf,
    /// Metadata (bila berhasil dibaca).
    pub metadata: Option<uartrecon_core::capture::CaptureMetadata>,
}

/// Command read-only yang disarankan, dengan kategori.
pub fn suggested_commands(
    fingerprint: Option<&Fingerprint>,
) -> Vec<(&'static str, &'static str, &'static str)> {
    // (kategori, command, deskripsi)
    let mut out: Vec<(&'static str, &'static str, &'static str)> = vec![
        ("System", "uname -a", "Informasi kernel"),
        ("System", "cat /proc/version", "Versi kernel"),
        ("System", "cat /etc/os-release", "Distro/OS"),
        ("CPU", "cat /proc/cpuinfo", "Informasi CPU"),
        ("CPU", "cat /proc/loadavg", "Beban sistem"),
        ("Memory", "cat /proc/meminfo", "Informasi memori"),
        ("Memory", "free", "Penggunaan memori"),
        ("Storage", "cat /proc/mtd", "Partisi MTD"),
        ("Storage", "cat /proc/partitions", "Partisi blok"),
        ("Storage", "mount", "Filesystem ter-mount"),
        ("Storage", "df -h", "Penggunaan disk"),
        ("Devices", "ls /dev", "Device node"),
        ("Devices", "ls /sys/class/net", "Interface jaringan"),
        ("Network", "ip addr", "Alamat IP"),
        ("Network", "cat /proc/net/route", "Routing table"),
        ("Process", "ps", "Daftar proses"),
        ("Config", "cat /proc/cmdline", "Kernel command line"),
        ("Config", "env", "Environment variable"),
    ];

    // Tambahan kontekstual bila U-Boot terdeteksi.
    if let Some(fp) = fingerprint
        && fp.bootloader.confidence != uartrecon_core::detector::fingerprint::Confidence::Unknown
    {
        out.push((
            "Bootloader",
            "printenv",
            "Tampilkan U-Boot environment (read-only)",
        ));
        out.push(("Bootloader", "bdinfo", "Board info (read-only)"));
    }
    out
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
            command_history: Vec::new(),
            history_index: None,
            session_name: uartrecon_core::capture::Session::default_name("gui"),
            analysis: AnalysisCache::default(),
            search: SearchState::default(),
            firmware: None,
            toast: None,
            analysis_section: AnalysisSection::default(),
            rate_tracker: RateTracker::default(),
        };
        app.refresh_ports();
        app.refresh_sessions("sessions");
        app
    }

    /// Menampilkan toast sementara.
    pub fn show_toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), Instant::now()));
    }

    /// Toast yang masih aktif (< 4 detik).
    pub fn active_toast(&self) -> Option<&str> {
        self.toast.as_ref().and_then(|(msg, t)| {
            if t.elapsed() < Duration::from_secs(4) {
                Some(msg.as_str())
            } else {
                None
            }
        })
    }

    /// Menjalankan analisis lengkap pada buffer RX saat ini.
    pub fn analyze_buffer(&mut self) {
        use uartrecon_core::analyzers::{entropy, signatures, stats, strings};
        let data = self.recorder.rx();
        if data.is_empty() {
            self.show_toast("Buffer kosong — belum ada data RX.");
            return;
        }
        let extracted = strings::extract_ascii(data, 4);
        self.analysis = AnalysisCache {
            stats: Some(stats::compute(data)),
            entropy: Some(entropy::analyze_default(data)),
            signatures: Some(signatures::scan(data)),
            interesting: Some(strings::find_interesting(&extracted)),
            strings: Some(extracted),
            source: format!("buffer RX ({} bytes)", data.len()),
        };
        self.log(format!("Analisis buffer selesai ({} bytes).", data.len()));
        self.show_toast("Analisis buffer selesai.");
    }

    /// Memuat firmware dari file dan menganalisisnya.
    pub fn load_firmware(&mut self, path: &std::path::Path) {
        use uartrecon_core::analyzers::{entropy, signatures, stats, strings};
        match std::fs::read(path) {
            Ok(data) => {
                let extracted = strings::extract_ascii(&data, 4);
                let analysis = AnalysisCache {
                    stats: Some(stats::compute(&data)),
                    entropy: Some(entropy::analyze_default(&data)),
                    signatures: Some(signatures::scan(&data)),
                    interesting: Some(strings::find_interesting(&extracted)),
                    strings: Some(extracted),
                    source: path.display().to_string(),
                };
                let sha256 = uartrecon_core::util::hash_sha256(&data);
                self.firmware = Some(FirmwareView {
                    path: path.display().to_string(),
                    size: data.len(),
                    sha256,
                    data,
                    analysis,
                });
                self.log(format!("Firmware dimuat: {}", path.display()));
                self.show_toast("Firmware dimuat & dianalisis.");
            }
            Err(e) => {
                self.error = Some(format!("gagal membaca firmware: {e}"));
                self.log(format!("Gagal membaca firmware: {e}"));
            }
        }
    }

    /// Menjalankan pencarian pada buffer RX.
    pub fn run_search(&mut self) {
        use uartrecon_core::capture::search::{self, SearchMode, SearchOptions};
        let data = self.recorder.rx();
        if data.is_empty() {
            self.search.error = Some("Buffer kosong.".to_string());
            return;
        }
        let mode = match self.search.mode.to_ascii_lowercase().as_str() {
            "regex" => SearchMode::Regex,
            "hex" => SearchMode::Hex,
            "icase" => SearchMode::LiteralInsensitive,
            _ => SearchMode::Literal,
        };
        let opts = SearchOptions {
            mode,
            context: 16,
            max_results: 1000,
        };
        match search::search(data, &self.search.query, &opts) {
            Ok(results) => {
                self.search.results = results;
                self.search.error = None;
                let n = self.search.results.len();
                self.show_toast(format!("{n} hasil ditemukan."));
            }
            Err(e) => {
                self.search.error = Some(e.to_string());
            }
        }
    }

    /// Menambahkan command ke riwayat.
    pub fn push_history(&mut self, cmd: &str) {
        if cmd.trim().is_empty() {
            return;
        }
        if self.command_history.last().map(|s| s.as_str()) != Some(cmd) {
            self.command_history.push(cmd.to_string());
        }
        if self.command_history.len() > 200 {
            self.command_history.remove(0);
        }
        self.history_index = None;
    }

    /// Navigasi riwayat (up = lebih lama).
    pub fn history_prev(&mut self) {
        if self.command_history.is_empty() {
            return;
        }
        let idx = match self.history_index {
            None => self.command_history.len() - 1,
            Some(0) => 0,
            Some(i) => i - 1,
        };
        self.history_index = Some(idx);
        self.command_input = self.command_history[idx].clone();
    }

    /// Navigasi riwayat (down = lebih baru).
    pub fn history_next(&mut self) {
        if let Some(i) = self.history_index {
            if i + 1 < self.command_history.len() {
                self.history_index = Some(i + 1);
                self.command_input = self.command_history[i + 1].clone();
            } else {
                self.history_index = None;
                self.command_input.clear();
            }
        }
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
        self.rate_tracker.update(self.recorder.rx().len());
    }

    /// Laju RX saat ini (byte/detik).
    pub fn rx_rate(&self) -> f64 {
        self.rate_tracker.rate()
    }

    /// Menyimpan buffer RX sebagai bootlog (txt + json) ke folder sessions.
    pub fn capture_bootlog(&mut self) {
        use uartrecon_core::analyzers::bootlog;
        let data = self.recorder.rx();
        if data.is_empty() {
            self.show_toast("Buffer kosong — belum ada data untuk bootlog.");
            return;
        }
        let log = bootlog::parse(data, 0);
        let dir = std::path::Path::new("sessions").join(&self.session_name);
        if std::fs::create_dir_all(&dir).is_err() {
            self.show_toast("Gagal membuat folder sessions.");
            return;
        }
        let txt = dir.join("bootlog.txt");
        let raw = dir.join("bootlog.raw");
        let _ = std::fs::write(&txt, log.to_text());
        let _ = std::fs::write(&raw, data);
        self.log(format!("Bootlog disimpan: {}", txt.display()));
        self.show_toast("Bootlog disimpan.");
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

    /// Menyimpan buffer RX ke file dengan format tertentu.
    ///
    /// Mengembalikan path file yang ditulis.
    pub fn export_buffer(
        &mut self,
        format: ExportFormat,
        dir: &std::path::Path,
    ) -> std::io::Result<std::path::PathBuf> {
        use uartrecon_core::capture::exporter;
        let data = self.recorder.rx();
        std::fs::create_dir_all(dir)?;
        let name = format!("{}_export.{}", self.session_name, format.extension());
        let path = dir.join(name);

        match format {
            ExportFormat::Txt => std::fs::write(&path, exporter::to_text(data))?,
            ExportFormat::Hex => std::fs::write(&path, exporter::to_hexdump(data))?,
            ExportFormat::Csv => std::fs::write(&path, exporter::to_csv(data))?,
            ExportFormat::Raw => std::fs::write(&path, data)?,
            ExportFormat::Json => {
                let fp = self.fingerprint.clone();
                let sha = uartrecon_core::util::hash_sha256(data);
                let json = serde_json::json!({
                    "session": self.session_name,
                    "port": self.selected_port,
                    "config": self.config,
                    "rx_bytes": data.len(),
                    "sha256": sha,
                    "fingerprint": fp,
                    "text": exporter::to_text(data),
                });
                std::fs::write(
                    &path,
                    serde_json::to_string_pretty(&json).unwrap_or_default(),
                )?;
            }
        }
        Ok(path)
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
