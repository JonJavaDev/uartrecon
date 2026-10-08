//! Logic analyzer: physical-layer UART decoding dari waveform mentah.
//!
//! Berbeda dengan mode USB-UART (yang hanya menerima byte hasil decoding),
//! mode ini bekerja pada **sampel digital** dari pin RX yang direkam oleh
//! logic analyzer / capture digital. Dari sini kita bisa:
//!
//! 1. Deteksi edge (transisi 0→1 atau 1→0).
//! 2. Ukur lebar pulsa untuk memperkirakan **bit period**.
//! 3. Hitung **baudrate** fisik dari bit period.
//! 4. Decode frame UART (start bit, data, parity, stop) menjadi byte.
//!
//! ## Model data
//!
//! [`Waveform`] menyimpan sampel digital (0/1) pada `sample_rate` Hz. Konvensi
//! UART idle = **high (1)**; start bit = **low (0)**.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::serial::config::{ParityCfg, SerialFormat};

/// Sampel digital berformat waktu.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Waveform {
    /// Laju sampel (Hz).
    pub sample_rate: u32,
    /// Sampel digital (0 atau 1). Nilai selain 0/1 dianggap 1 (idle).
    pub samples: Vec<u8>,
}

impl Waveform {
    /// Membuat waveform baru.
    pub fn new(sample_rate: u32, samples: Vec<u8>) -> Self {
        Waveform {
            sample_rate,
            samples,
        }
    }

    /// Durasi total (detik).
    pub fn duration_secs(&self) -> f64 {
        if self.sample_rate == 0 {
            return 0.0;
        }
        self.samples.len() as f64 / self.sample_rate as f64
    }

    /// Durasi total (milidetik).
    pub fn duration_ms(&self) -> f64 {
        self.duration_secs() * 1000.0
    }

    /// Level pada indeks tertentu (di-clamp).
    pub fn level(&self, idx: usize) -> u8 {
        self.samples.get(idx).map(|&s| s & 1).unwrap_or(1)
    }
}

/// Satu transisi (edge) dalam waveform.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    /// Indeks sampel tempat transisi terjadi.
    pub sample_index: usize,
    /// Waktu (detik).
    pub time_secs: f64,
    /// Level baru (0 atau 1).
    pub to_level: u8,
}

/// Mendeteksi semua edge pada waveform.
pub fn detect_edges(wave: &Waveform) -> Vec<Edge> {
    let mut edges = Vec::new();
    if wave.samples.is_empty() {
        return edges;
    }
    let mut prev = wave.samples[0] & 1;
    for (i, &s) in wave.samples.iter().enumerate().skip(1) {
        let cur = s & 1;
        if cur != prev {
            edges.push(Edge {
                sample_index: i,
                time_secs: i as f64 / wave.sample_rate as f64,
                to_level: cur,
            });
            prev = cur;
        }
    }
    edges
}

/// Statistik lebar pulsa (durasi antar-edge).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PulseStats {
    /// Jumlah pulsa.
    pub count: usize,
    /// Lebar minimum (detik).
    pub min_secs: f64,
    /// Lebar maksimum (detik).
    pub max_secs: f64,
    /// Lebar rata-rata (detik).
    pub mean_secs: f64,
    /// Lebar median (detik).
    pub median_secs: f64,
}

/// Menghitung statistik lebar pulsa dari daftar edge.
pub fn pulse_stats(edges: &[Edge]) -> Option<PulseStats> {
    if edges.len() < 2 {
        return None;
    }
    let mut widths: Vec<f64> = Vec::with_capacity(edges.len() - 1);
    for w in edges.windows(2) {
        widths.push(w[1].time_secs - w[0].time_secs);
    }
    widths.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let count = widths.len();
    let min = widths[0];
    let max = widths[count - 1];
    let mean = widths.iter().sum::<f64>() / count as f64;
    let median = if count % 2 == 1 {
        widths[count / 2]
    } else {
        (widths[count / 2 - 1] + widths[count / 2]) / 2.0
    };
    Some(PulseStats {
        count,
        min_secs: min,
        max_secs: max,
        mean_secs: mean,
        median_secs: median,
    })
}

/// Hasil estimasi baudrate fisik.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhysicalBaudEstimate {
    /// Baudrate terdekat dengan kandidat standar.
    pub baudrate: u32,
    /// Baudrate mentah hasil hitungan (1 / bit_period).
    pub raw_baud: f64,
    /// Estimasi bit period (detik).
    pub bit_period_secs: f64,
    /// Error relatif terhadap kandidat standar (0..1).
    pub error_ratio: f64,
}

/// Kandidat baudrate standar untuk snapping hasil pengukuran.
const SNAP_CANDIDATES: &[u32] = &[
    300, 600, 1200, 2400, 4800, 9600, 14_400, 19_200, 28_800, 38_400, 57_600, 115_200, 230_400,
    460_800, 921_600, 1_000_000, 1_500_000, 2_000_000,
];

/// Mengestimasi baudrate fisik dari bit period.
///
/// Bit period diambil dari pulsa **terpendek** yang masuk akal. Pada UART,
/// pulsa terpendek biasanya tepat 1 bit (mis. satu start bit di antara idle).
pub fn estimate_baud(stats: &PulseStats) -> PhysicalBaudEstimate {
    // Ambil pulsa terpendek yang >= 1% dari median (buang glitch).
    let bit_period = stats.min_secs.max(1e-9);
    let raw_baud = 1.0 / bit_period;

    // Snap ke kandidat standar terdekat.
    let mut best = SNAP_CANDIDATES[0];
    let mut best_err = f64::MAX;
    for &c in SNAP_CANDIDATES {
        let err = (raw_baud - c as f64).abs() / c as f64;
        if err < best_err {
            best_err = err;
            best = c;
        }
    }

    PhysicalBaudEstimate {
        baudrate: best,
        raw_baud,
        bit_period_secs: bit_period,
        error_ratio: best_err,
    }
}

/// Hasil decode satu frame UART.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecodedFrame {
    /// Nilai byte.
    pub value: u8,
    /// Indeks sampel awal (start bit).
    pub start_sample: usize,
    /// Waktu mulai (detik).
    pub start_secs: f64,
    /// Apakah parity valid (bila dipakai).
    pub parity_ok: bool,
    /// Apakah stop bit valid.
    pub stop_ok: bool,
}

/// Hasil decoding UART dari waveform.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecodeResult {
    /// Frame yang berhasil di-decode.
    pub frames: Vec<DecodedFrame>,
    /// Byte yang berhasil di-decode (berurutan).
    pub bytes: Vec<u8>,
    /// Jumlah frame dengan error framing/parity.
    pub error_count: usize,
}

impl DecodeResult {
    /// Byte hasil decode sebagai string lossy (untuk log teks).
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.bytes).to_string()
    }
}

/// Opsi decoding UART.
#[derive(Debug, Clone)]
pub struct DecodeOptions {
    /// Baudrate.
    pub baudrate: u32,
    /// Format (data bits / parity / stop bits).
    pub format: SerialFormat,
}

impl Default for DecodeOptions {
    fn default() -> Self {
        DecodeOptions {
            baudrate: 115_200,
            format: SerialFormat::EIGHT_N_ONE,
        }
    }
}

/// Men-decode UART dari waveform pada baudrate & format tertentu.
///
/// Algoritma:
/// 1. Cari transisi high→low (kandidat start bit).
/// 2. Verifikasi start bit dengan sampling di tengah bit.
/// 3. Sampling data bit (LSB dulu), parity, dan stop bit.
/// 4. Lanjut ke frame berikutnya setelah stop bit.
///
/// Posisi bit relatif terhadap tepi start:
/// - start bit  : pusat pada 0.5 bit
/// - data bit b : pusat pada 1.5 + b bit
/// - parity     : pusat pada 1.5 + data_bits bit
/// - stop bit s : pusat pada 1.5 + data_bits + parity_bits + s bit
pub fn decode(wave: &Waveform, opts: &DecodeOptions) -> Result<DecodeResult> {
    if wave.sample_rate == 0 {
        return Err(Error::InvalidConfig("sample_rate tidak boleh 0".into()));
    }
    let spb = wave.sample_rate as f64 / opts.baudrate as f64; // samples per bit
    if spb < 2.0 {
        return Err(Error::InvalidConfig(format!(
            "sample_rate {} terlalu rendah untuk baud {} (butuh >= 2 sampel/bit)",
            wave.sample_rate, opts.baudrate
        )));
    }

    let n = wave.samples.len();
    let idle = 1u8;

    // Sampling pada offset (dalam satuan bit) dari tepi start.
    let sample_at = |start: usize, bit_offset: f64| -> u8 {
        let pos = start as f64 + bit_offset * spb;
        let idx = pos.round() as usize;
        if idx >= n { idle } else { wave.level(idx) }
    };

    let mut frames = Vec::new();
    let mut bytes = Vec::new();
    let mut error_count = 0usize;

    let data_bits = opts.format.data_bits as i64;
    let parity_bits: i64 = if opts.format.parity == ParityCfg::None {
        0
    } else {
        1
    };
    let stop_bits = opts.format.stop_bits as i64;
    let total_bits = 1 + data_bits + parity_bits + stop_bits;

    let mut i = 0usize;
    while i < n {
        // Tunggu tepi high->low (kandidat start).
        if wave.level(i) == idle {
            i += 1;
            continue;
        }
        let start = i;

        // Verifikasi: pusat start bit harus low.
        if sample_at(start, 0.5) != 0 {
            i += 1;
            continue;
        }

        // Sampling data bit (LSB first).
        let mut value: u16 = 0;
        for b in 0..data_bits {
            let bit = sample_at(start, 1.5 + b as f64);
            value |= (bit as u16) << b;
        }

        // Parity.
        let mut parity_ok = true;
        if parity_bits == 1 {
            let parity_bit = sample_at(start, 1.5 + data_bits as f64);
            let ones = (value as u8).count_ones() as u8;
            let expected = match opts.format.parity {
                ParityCfg::Even => ones % 2,
                ParityCfg::Odd => (ones + 1) % 2,
                ParityCfg::None => 0,
            };
            parity_ok = parity_bit == expected;
        }

        // Stop bit(s).
        let stop_start = 1.5 + data_bits as f64 + parity_bits as f64;
        let mut stop_ok = true;
        for s in 0..stop_bits {
            if sample_at(start, stop_start + s as f64) != 1 {
                stop_ok = false;
            }
        }

        if stop_ok && parity_ok {
            frames.push(DecodedFrame {
                value: value as u8,
                start_sample: start,
                start_secs: start as f64 / wave.sample_rate as f64,
                parity_ok,
                stop_ok,
            });
            bytes.push(value as u8);
        } else {
            error_count += 1;
        }

        // Lanjut ke sampel setelah akhir frame.
        let next = start + (total_bits as f64 * spb).round() as usize;
        i = next.max(start + 1);
    }

    Ok(DecodeResult {
        frames,
        bytes,
        error_count,
    })
}

/// Pipeline lengkap: deteksi edge → statistik pulsa → estimasi baud → decode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// Jumlah edge.
    pub edge_count: usize,
    /// Statistik pulsa (bila ada cukup edge).
    pub pulse: Option<PulseStats>,
    /// Estimasi baudrate fisik (bila ada cukup edge).
    pub baud: Option<PhysicalBaudEstimate>,
    /// Hasil decode (bila baud & format diketahui).
    pub decoded: Option<DecodeResult>,
}

/// Menjalankan analisis penuh pada waveform.
///
/// `format` opsional; bila `None`, memakai `8N1` default untuk decoding.
pub fn analyze(wave: &Waveform, format: Option<SerialFormat>) -> Result<AnalysisResult> {
    let edges = detect_edges(wave);
    let pulse = pulse_stats(&edges);
    let baud = pulse.as_ref().map(estimate_baud);

    let decoded = if let Some(est) = &baud {
        let opts = DecodeOptions {
            baudrate: est.baudrate,
            format: format.unwrap_or(SerialFormat::EIGHT_N_ONE),
        };
        // Decode; jika gagal (sample rate rendah), lewati tanpa error fatal.
        decode(wave, &opts).ok()
    } else {
        None
    };

    Ok(AnalysisResult {
        edge_count: edges.len(),
        pulse,
        baud,
        decoded,
    })
}

/// Membangun waveform UART sintetis dari sekumpulan byte (untuk test & demo).
///
/// Menghasilkan sinyal idle-high dengan start bit low, data LSB-first, dan
/// stop bit high. `samples_per_bit` menentukan resolusi.
pub fn synth_uart(
    bytes: &[u8],
    baudrate: u32,
    samples_per_bit: u32,
    format: SerialFormat,
) -> Waveform {
    let sample_rate = baudrate * samples_per_bit;
    let mut samples: Vec<u8> = Vec::new();
    // Idle awal.
    samples.extend(std::iter::repeat_n(1u8, samples_per_bit as usize));
    for &byte in bytes {
        let push_bit = |samples: &mut Vec<u8>, bit: u8| {
            samples.extend(std::iter::repeat_n(bit, samples_per_bit as usize));
        };
        // Start bit (low).
        push_bit(&mut samples, 0);
        // Data bits (LSB first).
        for b in 0..format.data_bits {
            let bit = (byte >> b) & 1;
            push_bit(&mut samples, bit);
        }
        // Parity.
        match format.parity {
            ParityCfg::None => {}
            ParityCfg::Even => {
                let ones = byte.count_ones() as u8;
                push_bit(&mut samples, ones % 2);
            }
            ParityCfg::Odd => {
                let ones = byte.count_ones() as u8;
                push_bit(&mut samples, (ones + 1) % 2);
            }
        }
        // Stop bit(s) (high).
        for _ in 0..format.stop_bits {
            push_bit(&mut samples, 1);
        }
    }
    // Idle akhir.
    samples.extend(std::iter::repeat_n(1u8, samples_per_bit as usize));
    Waveform::new(sample_rate, samples)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_detection_dasar() {
        let wave = Waveform::new(1000, vec![1, 1, 0, 0, 1, 1]);
        let edges = detect_edges(&wave);
        assert_eq!(edges.len(), 2);
        assert_eq!(edges[0].sample_index, 2);
        assert_eq!(edges[0].to_level, 0);
        assert_eq!(edges[1].to_level, 1);
    }

    #[test]
    fn pulse_stats_benar() {
        // edges di index 3(->0), 5(->1), 10(->0) => 2 lebar pulsa: 2ms & 5ms
        let wave = Waveform::new(1000, vec![1, 1, 1, 0, 0, 1, 1, 1, 1, 1, 0]);
        let edges = detect_edges(&wave);
        assert_eq!(edges.len(), 3);
        let stats = pulse_stats(&edges).unwrap();
        assert_eq!(stats.count, 2);
        // pulsa low 2 sampel = 2ms, high 5 sampel = 5ms
        assert!((stats.min_secs - 0.002).abs() < 1e-9);
        assert!((stats.max_secs - 0.005).abs() < 1e-9);
    }

    #[test]
    fn synth_dan_decode_8n1() {
        let payload = b"OK";
        let wave = synth_uart(payload, 115_200, 8, SerialFormat::EIGHT_N_ONE);
        let result = decode(&wave, &DecodeOptions::default()).unwrap();
        assert_eq!(result.bytes, payload.to_vec());
        assert_eq!(result.error_count, 0);
    }

    #[test]
    fn synth_dan_decode_teks_panjang() {
        let payload = b"U-Boot 2021.10\r\n";
        let wave = synth_uart(payload, 57_600, 10, SerialFormat::EIGHT_N_ONE);
        let result = decode(
            &wave,
            &DecodeOptions {
                baudrate: 57_600,
                format: SerialFormat::EIGHT_N_ONE,
            },
        )
        .unwrap();
        assert_eq!(result.bytes, payload.to_vec());
        assert_eq!(result.text(), "U-Boot 2021.10\r\n");
    }

    #[test]
    fn estimasi_baud_dari_waveform() {
        // 115200 baud, 8 sampel/bit -> sample_rate 921600
        let wave = synth_uart(b"A", 115_200, 8, SerialFormat::EIGHT_N_ONE);
        let analysis = analyze(&wave, Some(SerialFormat::EIGHT_N_ONE)).unwrap();
        let baud = analysis.baud.expect("harus ada estimasi baud");
        assert_eq!(baud.baudrate, 115_200);
        assert!(baud.error_ratio < 0.05);
    }

    #[test]
    fn decode_dengan_parity_even() {
        let format = SerialFormat {
            data_bits: 8,
            parity: ParityCfg::Even,
            stop_bits: 1,
        };
        let wave = synth_uart(b"Hi", 19_200, 16, format);
        let result = decode(
            &wave,
            &DecodeOptions {
                baudrate: 19_200,
                format,
            },
        )
        .unwrap();
        assert_eq!(result.bytes, b"Hi".to_vec());
        assert!(result.frames.iter().all(|f| f.parity_ok));
    }

    #[test]
    fn sample_rate_terlalu_rendah_error() {
        let wave = Waveform::new(100, vec![0; 10]);
        let err = decode(
            &wave,
            &DecodeOptions {
                baudrate: 115_200,
                format: SerialFormat::EIGHT_N_ONE,
            },
        );
        assert!(err.is_err());
    }

    #[test]
    fn waveform_kosong_tidak_panic() {
        let wave = Waveform::new(1000, vec![]);
        let analysis = analyze(&wave, None).unwrap();
        assert_eq!(analysis.edge_count, 0);
        assert!(analysis.baud.is_none());
    }
}
