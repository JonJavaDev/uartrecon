//! Recorder: menyimpan stream RX dan TX secara terpisah.
//!
//! RX dan TX dipisah agar arah data tidak tercampur (penting untuk analisis
//! dan evidence tracking).

use std::time::Instant;

use serde::{Deserialize, Serialize};

/// Arah data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    /// Dari device ke host.
    Rx,
    /// Dari host ke device.
    Tx,
}

/// Satu potongan data yang terekam, dengan timestamp host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Chunk {
    /// Offset relatif terhadap awal sesi (milidetik).
    pub offset_ms: u64,
    /// Arah data.
    pub direction: Direction,
    /// Panjang dalam byte.
    pub len: usize,
}

/// Recorder menyimpan RX/TX terpisah + metadata timing.
///
/// Timestamp berasal dari host capture, **bukan** timestamp internal device.
#[derive(Debug, Clone)]
pub struct Recorder {
    rx: Vec<u8>,
    tx: Vec<u8>,
    events: Vec<Chunk>,
    started: Instant,
}

impl Default for Recorder {
    fn default() -> Self {
        Self::new()
    }
}

impl Recorder {
    /// Membuat recorder baru dan memulai penghitungan waktu.
    pub fn new() -> Self {
        Recorder {
            rx: Vec::new(),
            tx: Vec::new(),
            events: Vec::new(),
            started: Instant::now(),
        }
    }

    fn offset_ms(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }

    /// Merekam data RX.
    pub fn push_rx(&mut self, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        self.events.push(Chunk {
            offset_ms: self.offset_ms(),
            direction: Direction::Rx,
            len: data.len(),
        });
        self.rx.extend_from_slice(data);
    }

    /// Merekam data TX.
    pub fn push_tx(&mut self, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        self.events.push(Chunk {
            offset_ms: self.offset_ms(),
            direction: Direction::Tx,
            len: data.len(),
        });
        self.tx.extend_from_slice(data);
    }

    /// Byte RX.
    pub fn rx(&self) -> &[u8] {
        &self.rx
    }

    /// Byte TX.
    pub fn tx(&self) -> &[u8] {
        &self.tx
    }

    /// Event timing.
    pub fn events(&self) -> &[Chunk] {
        &self.events
    }

    /// Total byte yang terekam (RX + TX).
    pub fn total_bytes(&self) -> usize {
        self.rx.len() + self.tx.len()
    }

    /// Apakah tidak ada data sama sekali.
    pub fn is_empty(&self) -> bool {
        self.rx.is_empty() && self.tx.is_empty()
    }

    /// Menggabungkan RX dan TX menjadi satu buffer interleaved (RX dulu).
    ///
    /// Hanya untuk keperluan fingerprinting kasar; untuk analisis akurat
    /// gunakan [`Recorder::rx`] dan [`Recorder::tx`] secara terpisah.
    pub fn combined(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.rx.len() + self.tx.len());
        out.extend_from_slice(&self.rx);
        out.extend_from_slice(&self.tx);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rx_tx_terpisah() {
        let mut r = Recorder::new();
        r.push_rx(b"hello");
        r.push_tx(b"world");
        assert_eq!(r.rx(), b"hello");
        assert_eq!(r.tx(), b"world");
        assert_eq!(r.total_bytes(), 10);
    }

    #[test]
    fn events_tercatat() {
        let mut r = Recorder::new();
        r.push_rx(b"abc");
        r.push_tx(b"de");
        assert_eq!(r.events().len(), 2);
        assert_eq!(r.events()[0].direction, Direction::Rx);
        assert_eq!(r.events()[1].len, 2);
    }

    #[test]
    fn push_kosong_diabaikan() {
        let mut r = Recorder::new();
        r.push_rx(&[]);
        assert!(r.is_empty());
        assert_eq!(r.events().len(), 0);
    }
}
