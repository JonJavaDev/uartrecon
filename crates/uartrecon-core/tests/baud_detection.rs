//! Integration test: pipeline deteksi end-to-end pada data testdata.
//!
//! Test ini **tidak** membutuhkan hardware. Ia memakai [`MockSampleSource`]
//! yang mengembalikan data dari file `testdata/<baud>/capture.raw` pada
//! baudrate yang sesuai, lalu memverifikasi bahwa detektor memilih baudrate
//! dan format yang benar.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use uartrecon_core::detector::baudrate::{MockSampleSource, SampleSource, ScanOptions, scan};
use uartrecon_core::serial::config::{ParityCfg, SerialFormat};

/// Mencari direktori `testdata` (naik dari CWD).
fn testdata_dir() -> PathBuf {
    let candidates = [
        PathBuf::from("testdata"),
        PathBuf::from("../../testdata"),
        PathBuf::from("../../../testdata"),
    ];
    for c in candidates {
        if c.exists() {
            return c;
        }
    }
    panic!("direktori testdata tidak ditemukan");
}

/// Membaca dataset dan membangun mock yang mengembalikan data benar hanya
/// pada baudrate yang diharapkan; baudrate lain mengembalikan "garbage".
fn build_mock(expected_baud: u32, data: Vec<u8>) -> MockSampleSource {
    let garbage: Vec<u8> = (0..data.len())
        .map(|i| (i.wrapping_mul(137) ^ 0x5A) as u8)
        .collect();
    let mut map: HashMap<u32, Vec<u8>> = HashMap::new();
    for &b in uartrecon_core::detector::EMBEDDED_PRIORITY_BAUDRATES {
        if b == expected_baud {
            map.insert(b, data.clone());
        } else {
            map.insert(b, garbage.clone());
        }
    }
    MockSampleSource::new(map)
}

fn run_case(baud: u32, format: SerialFormat) {
    let dir = testdata_dir().join(baud.to_string());
    let raw_path = dir.join("capture.raw");
    let data = std::fs::read(&raw_path)
        .unwrap_or_else(|e| panic!("gagal membaca {}: {e}", raw_path.display()));

    let mut mock = build_mock(baud, data);
    let opts = ScanOptions {
        baudrates: uartrecon_core::detector::EMBEDDED_PRIORITY_BAUDRATES.to_vec(),
        duration: Duration::from_millis(1),
        format,
        min_bytes: 4,
    };
    let result = scan(&mut mock, &opts).expect("scan harus sukses");

    let best = result
        .best
        .clone()
        .unwrap_or_else(|| panic!("tidak ada kandidat terbaik untuk {baud}"));
    assert_eq!(
        best.baudrate, baud,
        "baudrate terdeteksi salah: dapat {} (harusnya {baud})",
        best.baudrate
    );
    assert!(
        result.is_reliable(50.0),
        "confidence terlalu rendah untuk {baud}: {}",
        result.confidence()
    );
}

#[test]
fn dataset_9600_8n1() {
    run_case(9600, SerialFormat::EIGHT_N_ONE);
}

#[test]
fn dataset_38400_8n1() {
    run_case(38400, SerialFormat::EIGHT_N_ONE);
}

#[test]
fn dataset_57600_8n1() {
    run_case(57600, SerialFormat::EIGHT_N_ONE);
}

#[test]
fn dataset_115200_8n1() {
    run_case(115_200, SerialFormat::EIGHT_N_ONE);
}

/// Memastikan bahwa pada data yang benar, format 8N1 menang atas format lain.
#[test]
fn format_8n1_menang() {
    let dir = testdata_dir().join("115200");
    let data = std::fs::read(dir.join("capture.raw")).unwrap();

    struct FormatMock {
        good: SerialFormat,
        data: Vec<u8>,
    }
    impl SampleSource for FormatMock {
        fn sample(
            &mut self,
            _baud: u32,
            format: SerialFormat,
            _d: Duration,
        ) -> uartrecon_core::Result<Option<Vec<u8>>> {
            if format == self.good {
                Ok(Some(self.data.clone()))
            } else {
                Ok(Some(
                    (0..self.data.len())
                        .map(|i| (i as u8).wrapping_mul(53))
                        .collect(),
                ))
            }
        }
    }

    let mut mock = FormatMock {
        good: SerialFormat::EIGHT_N_ONE,
        data,
    };
    let opts = uartrecon_core::detector::FormatScanOptions {
        baudrate: 115_200,
        formats: vec![
            SerialFormat::EIGHT_N_ONE,
            SerialFormat {
                data_bits: 8,
                parity: ParityCfg::Even,
                stop_bits: 1,
            },
        ],
        duration: Duration::from_millis(1),
        min_bytes: 4,
    };
    let result = uartrecon_core::detector::format::scan(&mut mock, &opts).unwrap();
    assert_eq!(result.best.unwrap().format.label(), "8N1");
}

/// Tanpa traffic, detektor harus jujur menyatakan tidak dapat menentukan config.
#[test]
fn tanpa_traffic_tidak_memaksa_hasil() {
    let mut mock = MockSampleSource::new([]);
    let result = scan(&mut mock, &ScanOptions::default()).unwrap();
    assert!(result.best.is_none());
    assert_eq!(result.confidence(), 0);
}
