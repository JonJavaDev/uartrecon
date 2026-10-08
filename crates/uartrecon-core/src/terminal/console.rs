//! Console buffer: menampung output untuk ditampilkan dalam mode ASCII/HEX/RAW.

use serde::{Deserialize, Serialize};

/// Mode tampilan output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputMode {
    /// Teks ASCII (non-printable di-escape).
    Ascii,
    /// Hex + ASCII (hexdump).
    Hex,
    /// Raw byte (tanpa transformasi).
    Raw,
}

impl OutputMode {
    /// Siklus mode berikutnya.
    pub fn next(self) -> Self {
        match self {
            OutputMode::Ascii => OutputMode::Hex,
            OutputMode::Hex => OutputMode::Raw,
            OutputMode::Raw => OutputMode::Ascii,
        }
    }

    /// Label.
    pub fn label(self) -> &'static str {
        match self {
            OutputMode::Ascii => "ASCII",
            OutputMode::Hex => "HEX",
            OutputMode::Raw => "RAW",
        }
    }
}

/// Buffer console yang menyimpan byte mentah dan menyediakan rendering.
#[derive(Debug, Clone)]
pub struct ConsoleBuffer {
    data: Vec<u8>,
    mode: OutputMode,
    max_bytes: usize,
}

impl Default for ConsoleBuffer {
    fn default() -> Self {
        Self::new(1024 * 1024)
    }
}

impl ConsoleBuffer {
    /// Membuat buffer dengan batas maksimum byte (untuk membatasi memori).
    pub fn new(max_bytes: usize) -> Self {
        ConsoleBuffer {
            data: Vec::new(),
            mode: OutputMode::Ascii,
            max_bytes,
        }
    }

    /// Menambahkan data.
    pub fn push(&mut self, data: &[u8]) {
        self.data.extend_from_slice(data);
        if self.data.len() > self.max_bytes {
            let excess = self.data.len() - self.max_bytes;
            self.data.drain(0..excess);
        }
    }

    /// Data mentah.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Mode aktif.
    pub fn mode(&self) -> OutputMode {
        self.mode
    }

    /// Mengubah mode.
    pub fn set_mode(&mut self, mode: OutputMode) {
        self.mode = mode;
    }

    /// Mengosongkan buffer.
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Render buffer sesuai mode aktif.
    pub fn render(&self) -> String {
        match self.mode {
            OutputMode::Ascii => crate::capture::exporter::to_text(&self.data),
            OutputMode::Hex => crate::capture::exporter::to_hexdump(&self.data),
            OutputMode::Raw => String::from_utf8_lossy(&self.data).to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_cycle() {
        assert_eq!(OutputMode::Ascii.next(), OutputMode::Hex);
        assert_eq!(OutputMode::Hex.next(), OutputMode::Raw);
        assert_eq!(OutputMode::Raw.next(), OutputMode::Ascii);
    }

    #[test]
    fn buffer_batasi_ukuran() {
        let mut b = ConsoleBuffer::new(10);
        b.push(b"0123456789ABCDEF");
        assert_eq!(b.data().len(), 10);
        assert_eq!(b.data(), b"6789ABCDEF");
    }

    #[test]
    fn render_ascii() {
        let mut b = ConsoleBuffer::new(100);
        b.push(b"hello");
        assert_eq!(b.render(), "hello");
    }

    #[test]
    fn render_hex() {
        let mut b = ConsoleBuffer::new(100);
        b.push(b"AB");
        b.set_mode(OutputMode::Hex);
        assert!(b.render().contains("41 42"));
    }

    #[test]
    fn clear_mengosongkan() {
        let mut b = ConsoleBuffer::new(100);
        b.push(b"data");
        b.clear();
        assert!(b.data().is_empty());
    }
}
