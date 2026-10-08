//! Formatter teks: laporan recon.

use crate::analyzers::linux::LinuxAnalysis;
use crate::analyzers::mtd::PartitionTable;
use crate::analyzers::uboot::UBootAnalysis;
use crate::detector::fingerprint::Fingerprint;
use crate::serial::config::SerialConfig;

/// Merender laporan recon teks dari hasil analisis.
pub fn render_report(
    config: SerialConfig,
    fp: &Fingerprint,
    uboot: &UBootAnalysis,
    linux: &LinuxAnalysis,
    partitions: Option<&PartitionTable>,
    sha256: Option<&str>,
) -> String {
    let mut out = String::new();
    out.push_str("RECON REPORT\n");
    out.push_str("------------------------\n\n");

    out.push_str("UART\n");
    out.push_str(&format!("  {}\n\n", config.label()));

    out.push_str("BOOTLOADER\n");
    out.push_str(&format!(
        "  {} ({}, {}/100)\n\n",
        fp.bootloader.name,
        fp.bootloader.confidence.label(),
        fp.bootloader.score
    ));

    if let Some(ram) = fp.ram_bytes {
        out.push_str("RAM\n");
        out.push_str(&format!("  {}\n\n", crate::util::human_size(ram)));
    }

    if let Some(storage) = fp.storage_bytes {
        out.push_str("STORAGE\n");
        out.push_str(&format!(
            "  {} {}\n\n",
            fp.storage_kind.as_deref().unwrap_or("storage"),
            crate::util::human_size(storage)
        ));
    }

    out.push_str("OS\n");
    out.push_str(&format!(
        "  {} ({}, {}/100)\n\n",
        fp.os.name,
        fp.os.confidence.label(),
        fp.os.score
    ));

    out.push_str("SHELL\n");
    out.push_str(&format!(
        "  {} ({}, {}/100)\n\n",
        fp.shell.name,
        fp.shell.confidence.label(),
        fp.shell.score
    ));

    out.push_str(&format!(
        "KERNEL\n  {}\n\n",
        if uboot.detected {
            "U-Boot detected"
        } else {
            "Detected"
        }
    ));
    out.push_str(&format!(
        "LINUX\n  {}\n\n",
        if linux.detected {
            format!("Embedded Linux ({})", linux.confidence.label())
        } else {
            "Not detected".to_string()
        }
    ));

    if let Some(table) = partitions
        && !table.is_empty()
    {
        out.push_str("PARTITIONS\n");
        out.push_str(&table.to_text());
        out.push('\n');
    }

    if let Some(hash) = sha256 {
        out.push_str("SHA256\n");
        out.push_str(&format!("  {hash}\n"));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detector::fingerprint;
    use crate::serial::config::SerialFormat;

    #[test]
    fn report_berisi_bagian_utama() {
        let log = b"U-Boot 2021.10\r\nDRAM: 512 MiB\r\nNAND: 256 MiB\r\nLinux version 5.10\r\nBusyBox\r\n";
        let fp = fingerprint::fingerprint(log);
        let uboot = crate::analyzers::uboot::analyze(log);
        let linux = crate::analyzers::linux::analyze(log);
        let report = render_report(
            SerialConfig::new(115_200, SerialFormat::EIGHT_N_ONE),
            &fp,
            &uboot,
            &linux,
            None,
            Some("deadbeef"),
        );
        assert!(report.contains("115200 8N1"));
        assert!(report.contains("U-Boot"));
        assert!(report.contains("512 MiB"));
        assert!(report.contains("deadbeef"));
    }
}
