//! Integration test: logic analyzer (physical-layer UART decoding).
//!
//! Test memakai waveform sintetis yang dibangun oleh `synth_uart`, sehingga
//! tidak memerlukan hardware logic analyzer.

use uartrecon_core::logic::{self, DecodeOptions, Waveform};
use uartrecon_core::serial::config::{ParityCfg, SerialFormat};

#[test]
fn physical_baud_detection_akurat() {
    for &baud in &[9600u32, 19_200, 38_400, 57_600, 115_200] {
        let wave = logic::synth_uart(b"HELLO", baud, 8, SerialFormat::EIGHT_N_ONE);
        let analysis = logic::analyze(&wave, Some(SerialFormat::EIGHT_N_ONE)).unwrap();
        let est = analysis.baud.expect("harus ada estimasi baud");
        assert_eq!(
            est.baudrate, baud,
            "baudrate fisik salah: dapat {} (harusnya {baud})",
            est.baudrate
        );
        assert!(
            est.error_ratio < 0.02,
            "error terlalu besar untuk {baud}: {}",
            est.error_ratio
        );
    }
}

#[test]
fn decode_teks_utuh() {
    let payload = b"U-Boot 2021.10\r\nDRAM: 512 MiB\r\nLinux version 5.10\r\nBusyBox\r\nlogin: ";
    let wave = logic::synth_uart(payload, 115_200, 8, SerialFormat::EIGHT_N_ONE);
    let result = logic::decode(&wave, &DecodeOptions::default()).unwrap();
    assert_eq!(result.bytes, payload.to_vec());
    assert_eq!(result.error_count, 0);
}

#[test]
fn decode_dengan_berbagai_format() {
    let formats = [
        SerialFormat::EIGHT_N_ONE,
        SerialFormat {
            data_bits: 8,
            parity: ParityCfg::Even,
            stop_bits: 1,
        },
        SerialFormat {
            data_bits: 8,
            parity: ParityCfg::Odd,
            stop_bits: 1,
        },
        SerialFormat {
            data_bits: 8,
            parity: ParityCfg::None,
            stop_bits: 2,
        },
        SerialFormat {
            data_bits: 7,
            parity: ParityCfg::Even,
            stop_bits: 1,
        },
    ];

    for fmt in formats {
        // 7-bit hanya bisa encode 0..127.
        let payload: Vec<u8> = if fmt.data_bits == 7 {
            b"Hello".to_vec()
        } else {
            b"Test123".to_vec()
        };
        let wave = logic::synth_uart(&payload, 19_200, 16, fmt);
        let result = logic::decode(
            &wave,
            &DecodeOptions {
                baudrate: 19_200,
                format: fmt,
            },
        )
        .unwrap();
        assert_eq!(
            result.bytes,
            payload,
            "decode gagal untuk format {}",
            fmt.label()
        );
    }
}

#[test]
fn waveform_dari_file_csv() {
    // Simulasi file CSV `index,level`.
    let wave = logic::synth_uart(b"AB", 9600, 4, SerialFormat::EIGHT_N_ONE);
    let csv: String = wave
        .samples
        .iter()
        .enumerate()
        .map(|(i, &s)| format!("{i},{s}\n"))
        .collect();

    // Parse ulang seperti CLI melakukannya.
    let samples: Vec<u8> = csv
        .lines()
        .filter_map(|l| l.split(',').next_back())
        .filter_map(|s| s.trim().parse::<u8>().ok())
        .collect();
    let wave2 = Waveform::new(wave.sample_rate, samples);
    let result = logic::decode(
        &wave2,
        &DecodeOptions {
            baudrate: 9600,
            format: SerialFormat::EIGHT_N_ONE,
        },
    )
    .unwrap();
    assert_eq!(result.bytes, b"AB".to_vec());
}

#[test]
fn waveform_kosong_aman() {
    let wave = Waveform::new(1_000_000, vec![]);
    let analysis = logic::analyze(&wave, None).unwrap();
    assert_eq!(analysis.edge_count, 0);
    assert!(analysis.baud.is_none());
    assert!(analysis.decoded.is_none());
}

/// Mencari direktori `testdata` (naik dari CWD).
fn testdata_dir() -> std::path::PathBuf {
    let candidates = [
        std::path::PathBuf::from("testdata"),
        std::path::PathBuf::from("../../testdata"),
        std::path::PathBuf::from("../../../testdata"),
    ];
    candidates
        .into_iter()
        .find(|c| c.exists())
        .expect("direktori testdata tidak ditemukan")
}

#[test]
fn waveform_dari_testdata_raw() {
    let path = testdata_dir().join("waveform").join("demo.raw");
    let Ok(data) = std::fs::read(&path) else {
        eprintln!("skip: {} tidak ada", path.display());
        return;
    };
    let samples: Vec<u8> = data.iter().map(|&b| b & 1).collect();
    let wave = Waveform::new(921_600, samples);

    let analysis = logic::analyze(&wave, Some(SerialFormat::EIGHT_N_ONE)).unwrap();
    let est = analysis.baud.expect("harus ada estimasi baud");
    assert_eq!(est.baudrate, 115_200);
    assert!(est.error_ratio < 0.02);

    let decoded = analysis.decoded.expect("harus ada hasil decode");
    assert!(decoded.text().contains("U-Boot"));
    assert!(decoded.text().contains("BusyBox"));
}
