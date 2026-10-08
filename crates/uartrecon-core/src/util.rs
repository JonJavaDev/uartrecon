//! Utilitas kecil tanpa dependency tambahan (timestamp, tanggal, hashing).

use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

/// Waktu Unix (detik) saat ini, atau `0` bila jam sistem sebelum epoch.
pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Konversi epoch (detik) menjadi `(tahun, bulan, hari)` UTC.
///
/// Memakai algoritma "civil from days" (Howard Hinnant) agar tidak perlu
/// dependency kalender seperti `chrono`.
pub fn date_ymd(epoch_secs: u64) -> (i64, u32, u32) {
    let days = (epoch_secs / 86_400) as i64;
    civil_from_days(days)
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}

/// Menghitung hash SHA-256 dan mengembalikannya sebagai hex huruf kecil.
pub fn hash_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Format ukuran byte menjadi string yang mudah dibaca (mis. `512 MiB`).
pub fn human_size(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;
    if bytes >= GIB {
        format!("{:.2} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.0} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.0} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tanggal_epoch_nol() {
        assert_eq!(date_ymd(0), (1970, 1, 1));
    }

    #[test]
    fn tanggal_diketahui() {
        // 2021-01-01 00:00:00 UTC = 1609459200
        assert_eq!(date_ymd(1_609_459_200), (2021, 1, 1));
    }

    #[test]
    fn hash_stabil() {
        assert_eq!(
            hash_sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn ukuran_manusiawi() {
        assert_eq!(human_size(512 * 1024 * 1024), "512 MiB");
        assert_eq!(human_size(1024), "1 KiB");
    }
}
