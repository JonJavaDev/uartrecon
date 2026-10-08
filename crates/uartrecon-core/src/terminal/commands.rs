//! Model command dan klasifikasi risiko.
//!
//! Prinsip **read-only first**: command read-only diizinkan, command yang
//! mengubah state butuh konfirmasi, dan command destruktif diblokir.

use serde::{Deserialize, Serialize};

/// Tingkat risiko sebuah command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandRisk {
    /// Aman dijalankan (baca saja).
    ReadOnly,
    /// Mengubah state; butuh konfirmasi user.
    StateChanging,
    /// Berpotensi merusak; diblokir oleh default.
    Destructive,
}

/// Command internal tool (bukan command yang dikirim ke device).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    /// Menampilkan bantuan.
    Help,
    /// Info konfigurasi & hasil deteksi.
    Info,
    /// Scan port / baudrate.
    Scan,
    /// Buka koneksi.
    Connect,
    /// Tutup koneksi.
    Disconnect,
    /// Monitor data.
    Monitor,
    /// Mulai/hentikan dump.
    Dump,
    /// Rekam bootlog.
    Bootlog,
    /// Tampilkan hex.
    Hex,
    /// Kirim data ke device.
    Send,
    /// Saran command read-only.
    Suggest,
    /// Analisis capture.
    Analyze,
    /// Simpan sesi.
    Save,
    /// Ekspor.
    Export,
    /// Bersihkan layar.
    Clear,
    /// Kelola sesi.
    Session,
    /// Keluar.
    Exit,
}

impl Command {
    /// Semua command yang dikenal.
    pub const ALL: &'static [Command] = &[
        Command::Help,
        Command::Info,
        Command::Scan,
        Command::Connect,
        Command::Disconnect,
        Command::Monitor,
        Command::Dump,
        Command::Bootlog,
        Command::Hex,
        Command::Send,
        Command::Suggest,
        Command::Analyze,
        Command::Save,
        Command::Export,
        Command::Clear,
        Command::Session,
        Command::Exit,
    ];

    /// Nama command (untuk parsing).
    pub fn name(self) -> &'static str {
        match self {
            Command::Help => "help",
            Command::Info => "info",
            Command::Scan => "scan",
            Command::Connect => "connect",
            Command::Disconnect => "disconnect",
            Command::Monitor => "monitor",
            Command::Dump => "dump",
            Command::Bootlog => "bootlog",
            Command::Hex => "hex",
            Command::Send => "send",
            Command::Suggest => "suggest",
            Command::Analyze => "analyze",
            Command::Save => "save",
            Command::Export => "export",
            Command::Clear => "clear",
            Command::Session => "session",
            Command::Exit => "exit",
        }
    }

    /// Deskripsi singkat.
    pub fn description(self) -> &'static str {
        match self {
            Command::Help => "Tampilkan bantuan",
            Command::Info => "Info konfigurasi & hasil deteksi",
            Command::Scan => "Scan port / baudrate",
            Command::Connect => "Buka koneksi UART",
            Command::Disconnect => "Tutup koneksi UART",
            Command::Monitor => "Monitor data masuk",
            Command::Dump => "Mulai/hentikan dump",
            Command::Bootlog => "Rekam boot log",
            Command::Hex => "Tampilkan hex viewer",
            Command::Send => "Kirim data ke device",
            Command::Suggest => "Saran command read-only",
            Command::Analyze => "Analisis capture",
            Command::Save => "Simpan sesi",
            Command::Export => "Ekspor ke format lain",
            Command::Clear => "Bersihkan layar",
            Command::Session => "Kelola sesi",
            Command::Exit => "Keluar",
        }
    }

    /// Parse nama command.
    pub fn parse(s: &str) -> Option<Command> {
        Command::ALL.iter().copied().find(|c| c.name() == s)
    }
}

/// Mengklasifikasikan risiko sebuah command yang akan **dikirim ke device**.
///
/// Ini adalah garis pertahanan utama read-only-first: perintah destruktif
/// dikenali berdasarkan pola dan diblokir.
pub fn classify_risk(input: &str) -> CommandRisk {
    let cmd = input.trim().to_ascii_lowercase();
    if cmd.is_empty() {
        return CommandRisk::ReadOnly;
    }

    // Pola destruktif yang selalu diblokir.
    const DESTRUCTIVE: &[&str] = &[
        "flash_erase",
        "flash_eraseall",
        "mtd_debug erase",
        "nand erase",
        "nand write",
        "mtd write",
        "dd if=",
        "dd of=",
        "mkfs",
        "format ",
        "reboot",
        "reset",
        "rm -rf",
        "rm /",
        "fdisk",
        "parted",
        "sf write",
        "nand markbad",
        "ubiformat",
        "setenv",
        "saveenv",
        "fw_setenv",
        "erase ",
        "write ",
        "cp ",
        "mv ",
        ">",
        "tee ",
        "chmod ",
        "chown ",
    ];
    for pat in DESTRUCTIVE {
        if cmd.contains(pat) {
            return CommandRisk::Destructive;
        }
    }

    // Pola yang mengubah state (butuh konfirmasi).
    const STATE_CHANGING: &[&str] = &[
        "ifconfig",
        "ip link set",
        "ip addr add",
        "route add",
        "insmod",
        "rmmod",
        "modprobe",
        "mount",
        "umount",
        "kill",
        "killall",
        "systemctl",
        "service ",
        "start ",
        "stop ",
        "echo ",
        "gpio",
        "i2cset",
    ];
    for pat in STATE_CHANGING {
        if cmd.contains(pat) {
            return CommandRisk::StateChanging;
        }
    }

    CommandRisk::ReadOnly
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_command() {
        assert_eq!(Command::parse("help"), Some(Command::Help));
        assert_eq!(Command::parse("unknown"), None);
        assert_eq!(Command::ALL.len(), 17);
    }

    #[test]
    fn nama_roundtrip() {
        for c in Command::ALL {
            assert_eq!(Command::parse(c.name()), Some(*c));
        }
    }

    #[test]
    fn read_only_command() {
        assert_eq!(classify_risk("cat /proc/mtd"), CommandRisk::ReadOnly);
        assert_eq!(classify_risk("uname -a"), CommandRisk::ReadOnly);
        assert_eq!(classify_risk("ls /dev"), CommandRisk::ReadOnly);
    }

    #[test]
    fn destructive_diblokir() {
        assert_eq!(
            classify_risk("flash_erase /dev/mtd0 0 1"),
            CommandRisk::Destructive
        );
        assert_eq!(
            classify_risk("nand write 0x80000000 0 0x100000"),
            CommandRisk::Destructive
        );
        assert_eq!(classify_risk("reboot"), CommandRisk::Destructive);
        assert_eq!(
            classify_risk("setenv bootargs foo"),
            CommandRisk::Destructive
        );
    }

    #[test]
    fn state_changing_butuh_konfirmasi() {
        assert_eq!(
            classify_risk("ifconfig eth0 up"),
            CommandRisk::StateChanging
        );
        assert_eq!(
            classify_risk("insmod mymodule.ko"),
            CommandRisk::StateChanging
        );
    }

    #[test]
    fn kosong_readonly() {
        assert_eq!(classify_risk(""), CommandRisk::ReadOnly);
    }
}
