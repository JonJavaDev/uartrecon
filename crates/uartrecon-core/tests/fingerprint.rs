//! Integration test: fingerprinting perangkat dari data bootlog.

use uartrecon_core::analyzers::{linux, mtd, uboot};
use uartrecon_core::detector::fingerprint::{Confidence, fingerprint};

const UBOOT_LOG: &[u8] = b"U-Boot 2021.10 (Jan 01 2021)\r\nDRAM:  512 MiB\r\nNAND:  256 MiB\r\nHit any key to stop autoboot\r\n";

const LINUX_LOG: &[u8] = b"Starting kernel ...\r\nLinux version 5.10.0 (gcc version 9.3.0)\r\n/proc /sys /etc\r\nBusyBox v1.35.0\r\nlogin: root\r\nroot@stb:~# ";

const CFE_LOG: &[u8] = b"CFE version 1.0.37 for BCM96328\r\nBoot Strap: NAND\r\nDRAM:  64 MiB\r\n";

#[test]
fn fingerprint_uboot() {
    let fp = fingerprint(UBOOT_LOG);
    assert_eq!(fp.bootloader.confidence, Confidence::Detected);
    assert_eq!(fp.ram_bytes, Some(512 * 1024 * 1024));
    assert_eq!(fp.storage_bytes, Some(256 * 1024 * 1024));
    assert_eq!(fp.storage_kind.as_deref(), Some("NAND"));
}

#[test]
fn fingerprint_linux_busybox() {
    let fp = fingerprint(LINUX_LOG);
    assert_eq!(fp.os.confidence, Confidence::Detected);
    assert_eq!(fp.shell.name, "BusyBox");
    assert!(fp.shell.score >= 55);
}

#[test]
fn fingerprint_cfe() {
    let fp = fingerprint(CFE_LOG);
    assert_eq!(fp.bootloader.confidence, Confidence::Detected);
}

#[test]
fn analyzer_uboot() {
    let a = uboot::analyze(UBOOT_LOG);
    assert!(a.detected);
    assert_eq!(a.indicators.version.as_deref(), Some("2021.10"));
}

#[test]
fn analyzer_linux() {
    let a = linux::analyze(LINUX_LOG);
    assert!(a.detected);
    assert_eq!(a.confidence, Confidence::Detected);
    assert!(!linux::suggestions(&a).is_empty());
}

#[test]
fn parser_mtd_lengkap() {
    let log = b"dev:    size   erasesize  name\nmtd0: 00040000 00020000 \"bootloader\"\nmtd1: 00300000 00020000 \"kernel\"\nmtd2: 02000000 00020000 \"rootfs\"\n";
    let table = mtd::parse_any(log);
    assert_eq!(table.partitions.len(), 3);
    assert_eq!(table.partitions[2].name, "rootfs");
    assert_eq!(table.storage_kind.as_deref(), Some("NAND"));
}

#[test]
fn data_acak_tidak_menghasilkan_klaim_kuat() {
    let garbage: Vec<u8> = (0..500u32)
        .map(|i| (i.wrapping_mul(2654435761)) as u8)
        .collect();
    let fp = fingerprint(&garbage);
    // Tidak boleh mengklaim bootloader/OS secara kuat dari data acak.
    assert_ne!(fp.bootloader.confidence, Confidence::Detected);
    assert_ne!(fp.os.confidence, Confidence::Detected);
}
