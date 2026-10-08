//! Language pack sederhana (Indonesia / English).
//!
//! Pemakaian:
//! ```ignore
//! use uartrecon_core::i18n::{Lang, Key, tr};
//! let l = Lang::Id;
//! println!("{}", tr(l, Key::Connected));
//! ```

/// Bahasa yang didukung.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Lang {
    /// Bahasa Indonesia.
    #[default]
    Id,
    /// English.
    En,
}

impl Lang {
    /// Kode bahasa (untuk config).
    pub fn code(self) -> &'static str {
        match self {
            Lang::Id => "id",
            Lang::En => "en",
        }
    }

    /// Nama tampilan.
    pub fn name(self) -> &'static str {
        match self {
            Lang::Id => "Bahasa Indonesia",
            Lang::En => "English",
        }
    }

    /// Parse dari kode (id/en, fallback ke Id).
    pub fn parse(s: &str) -> Lang {
        match s.trim().to_ascii_lowercase().as_str() {
            "en" | "eng" | "english" => Lang::En,
            _ => Lang::Id,
        }
    }

    /// Semua bahasa.
    pub const ALL: &'static [Lang] = &[Lang::Id, Lang::En];
}

/// Kunci teks yang bisa diterjemahkan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    // Umum
    Connected,
    Disconnected,
    Busy,
    Ready,
    Error,
    Warning,
    Cancel,
    Yes,
    No,
    Save,
    Exit,

    // Tab / panel
    TabTerminal,
    TabDetection,
    TabAnalysis,
    TabFirmware,
    TabWaveform,
    TabSessions,
    Language,

    // Terminal
    TerminalTitle,
    CopyOutput,
    CopyHex,
    SaveTxt,
    Clear,
    Fingerprint,
    Analyze,
    Bootlog,
    Send,
    Command,
    QuickCommands,
    ConnectedTo,
    DisconnectedMsg,

    // Detection
    ScanBaudrate,
    DetectionTitle,

    // Sessions
    SessionsTitle,
    RefreshSessions,

    // Flash
    FlashLede,
    FlashPlan,
    FlashCheck,
    FlashRun,
}

/// Mengambil teks terjemahan untuk sebuah kunci.
pub fn tr(lang: Lang, key: Key) -> &'static str {
    match lang {
        Lang::Id => id(key),
        Lang::En => en(key),
    }
}

fn id(key: Key) -> &'static str {
    match key {
        Key::Connected => "Terhubung",
        Key::Disconnected => "Terputus",
        Key::Busy => "Sibuk",
        Key::Ready => "Siap",
        Key::Error => "Kesalahan",
        Key::Warning => "Peringatan",
        Key::Cancel => "Batal",
        Key::Yes => "Ya",
        Key::No => "Tidak",
        Key::Save => "Simpan",
        Key::Exit => "Keluar",

        Key::TabTerminal => "Terminal",
        Key::TabDetection => "Deteksi",
        Key::TabAnalysis => "Analisis",
        Key::TabFirmware => "Firmware",
        Key::TabWaveform => "Waveform",
        Key::TabSessions => "Sesi",
        Key::Language => "Bahasa",

        Key::TerminalTitle => "Terminal UART",
        Key::CopyOutput => "Salin Output",
        Key::CopyHex => "Salin Hex",
        Key::SaveTxt => "Simpan TXT",
        Key::Clear => "Bersihkan",
        Key::Fingerprint => "Fingerprint",
        Key::Analyze => "Analisis",
        Key::Bootlog => "Bootlog",
        Key::Send => "Kirim",
        Key::Command => "Perintah",
        Key::QuickCommands => "Cepat",
        Key::ConnectedTo => "Terhubung ke",
        Key::DisconnectedMsg => "Koneksi ditutup",

        Key::ScanBaudrate => "Scan baudrate",
        Key::DetectionTitle => "Deteksi",

        Key::SessionsTitle => "Sesi tersimpan",
        Key::RefreshSessions => "Muat ulang",

        Key::FlashLede => "Flash LEDE",
        Key::FlashPlan => "Rencana",
        Key::FlashCheck => "Periksa",
        Key::FlashRun => "Jalankan",
    }
}

fn en(key: Key) -> &'static str {
    match key {
        Key::Connected => "Connected",
        Key::Disconnected => "Disconnected",
        Key::Busy => "Busy",
        Key::Ready => "Ready",
        Key::Error => "Error",
        Key::Warning => "Warning",
        Key::Cancel => "Cancel",
        Key::Yes => "Yes",
        Key::No => "No",
        Key::Save => "Save",
        Key::Exit => "Exit",

        Key::TabTerminal => "Terminal",
        Key::TabDetection => "Detection",
        Key::TabAnalysis => "Analysis",
        Key::TabFirmware => "Firmware",
        Key::TabWaveform => "Waveform",
        Key::TabSessions => "Sessions",
        Key::Language => "Language",

        Key::TerminalTitle => "UART Terminal",
        Key::CopyOutput => "Copy Output",
        Key::CopyHex => "Copy Hex",
        Key::SaveTxt => "Save TXT",
        Key::Clear => "Clear",
        Key::Fingerprint => "Fingerprint",
        Key::Analyze => "Analyze",
        Key::Bootlog => "Bootlog",
        Key::Send => "Send",
        Key::Command => "Command",
        Key::QuickCommands => "Quick",
        Key::ConnectedTo => "Connected to",
        Key::DisconnectedMsg => "Connection closed",

        Key::ScanBaudrate => "Scan baudrate",
        Key::DetectionTitle => "Detection",

        Key::SessionsTitle => "Saved sessions",
        Key::RefreshSessions => "Refresh",

        Key::FlashLede => "Flash LEDE",
        Key::FlashPlan => "Plan",
        Key::FlashCheck => "Check",
        Key::FlashRun => "Run",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lang_parse() {
        assert_eq!(Lang::parse("en"), Lang::En);
        assert_eq!(Lang::parse("EN"), Lang::En);
        assert_eq!(Lang::parse("id"), Lang::Id);
        assert_eq!(Lang::parse("xyz"), Lang::Id);
    }

    #[test]
    fn tr_berbeda_per_bahasa() {
        assert_eq!(tr(Lang::Id, Key::Connected), "Terhubung");
        assert_eq!(tr(Lang::En, Key::Connected), "Connected");
    }

    #[test]
    fn code_roundtrip() {
        for l in Lang::ALL {
            assert_eq!(Lang::parse(l.code()), *l);
        }
    }
}
