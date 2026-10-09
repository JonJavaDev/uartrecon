//! Auto-login helper: deteksi prompt device & susun urutan login.
//!
//! Banyak perangkat embedded (router, STB, IP camera) menampilkan prompt
//! login di UART. Modul ini menyediakan **deteksi prompt** (login, password,
//! shell) dan **rencana login** yang bisa dijalankan oleh CLI.
//!
//! Semua logika di sini murni (tanpa I/O serial) sehingga bisa diuji dengan
//! data sintetis - sesuai prinsip "testable tanpa hardware".

use serde::{Deserialize, Serialize};

/// Jenis prompt yang terdeteksi pada output device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PromptKind {
    /// Prompt login (mis. `login:`, `LEDE login:`).
    Login,
    /// Prompt password (mis. `Password:`).
    Password,
    /// Shell siap (mis. `root@LEDE:/#`, `# `, `$ `).
    Shell,
    /// Prompt U-Boot (`STB-BOOT #`, `=> `).
    Uboot,
}

impl PromptKind {
    /// Label singkat.
    pub fn label(self) -> &'static str {
        match self {
            PromptKind::Login => "login",
            PromptKind::Password => "password",
            PromptKind::Shell => "shell",
            PromptKind::Uboot => "uboot",
        }
    }
}

/// Rencana login ke device via UART.
///
/// Berisi kredensial & pola prompt yang diharapkan. Dipakai CLI untuk
/// mengotomasi: tunggu prompt login -> kirim user -> tunggu password ->
/// kirim password -> tunggu shell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoginPlan {
    /// Username (opsional - sebagian device langsung shell).
    pub username: Option<String>,
    /// Password (opsional).
    pub password: Option<String>,
    /// Timeout menunggu tiap prompt (milidetik).
    pub timeout_ms: u64,
    /// Jeda setelah kirim kredensial (milidetik).
    pub delay_ms: u64,
    /// Line-ending saat mengirim kredensial: `cr`, `lf`, atau `crlf`.
    pub enter: String,
}

fn default_timeout() -> u64 {
    5000
}

fn default_delay() -> u64 {
    300
}

fn default_enter() -> String {
    "cr".to_string()
}

impl Default for LoginPlan {
    fn default() -> Self {
        LoginPlan {
            username: None,
            password: None,
            timeout_ms: default_timeout(),
            delay_ms: default_delay(),
            enter: default_enter(),
        }
    }
}

impl LoginPlan {
    /// Membuat rencana login dengan kredensial.
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        LoginPlan {
            username: Some(username.into()),
            password: Some(password.into()),
            ..LoginPlan::default()
        }
    }

    /// Byte line-ending sesuai konfigurasi.
    pub fn enter_bytes(&self) -> Vec<u8> {
        match self.enter.to_ascii_lowercase().as_str() {
            "lf" | "\\n" => vec![b'\n'],
            "crlf" | "\\r\\n" => vec![b'\r', b'\n'],
            _ => vec![b'\r'],
        }
    }

    /// Urutan langkah: (prompt yang ditunggu, data yang dikirim).
    ///
    /// Langkah dengan data `None` hanya "menunggu" (mis. tunggu prompt shell
    /// terakhir tanpa mengirim apa pun).
    pub fn steps(&self) -> Vec<(PromptKind, Option<Vec<u8>>)> {
        let mut steps = Vec::new();
        let enter = self.enter_bytes();
        if let Some(user) = &self.username {
            let mut data = user.clone().into_bytes();
            data.extend_from_slice(&enter);
            steps.push((PromptKind::Login, Some(data)));
            if let Some(pass) = &self.password {
                let mut pdata = pass.clone().into_bytes();
                pdata.extend_from_slice(&enter);
                steps.push((PromptKind::Password, Some(pdata)));
            }
        }
        // Langkah terakhir: tunggu shell siap (tanpa kirim apa pun).
        steps.push((PromptKind::Shell, None));
        steps
    }
}

/// Deteksi jenis prompt terakhir pada output device.
///
/// Memilih prompt yang **paling akhir** muncul di buffer (device bisa
/// menampilkan beberapa prompt berurutan). Mengembalikan `None` bila tidak ada.
pub fn detect_prompt(text: &str) -> Option<PromptKind> {
    let clean = strip_ansi(text);
    let lower = clean.to_ascii_lowercase();

    let mut best: Option<(usize, PromptKind)> = None;
    let mut consider = |idx: usize, kind: PromptKind| {
        if best.is_none() || idx >= best.as_ref().unwrap().0 {
            best = Some((idx, kind));
        }
    };

    // Prompt U-Boot.
    if let Some(i) = lower.rfind("=> ") {
        consider(i, PromptKind::Uboot);
    }
    for pat in ["stb-boot #", "u-boot #", "u-boot>", "uboot>"] {
        if let Some(i) = lower.rfind(pat) {
            consider(i, PromptKind::Uboot);
        }
    }

    // Prompt password.
    if let Some(i) = lower.rfind("password:") {
        consider(i, PromptKind::Password);
    }

    // Prompt login (mis. `login:`, `LEDE login:`, `Username:`).
    if let Some(i) = lower.rfind("login:") {
        consider(i, PromptKind::Login);
    }
    if let Some(i) = lower.rfind("username:") {
        consider(i, PromptKind::Login);
    }

    // Shell prompt: `root@LEDE:/#`, `# `, `$ `.
    if let Some(i) = shell_prompt_index(&clean) {
        consider(i, PromptKind::Shell);
    }

    best.map(|(_, k)| k)
}

/// Apakah output menandakan shell sudah siap (untuk device tanpa login).
pub fn is_shell_ready(text: &str) -> bool {
    let clean = strip_ansi(text);
    shell_prompt_index(&clean).is_some()
}

/// Mencari posisi prompt shell pada teks.
///
/// Mendukung pola umum: `user@host:/path#`, `user@host:~$`, diakhiri `# ` / `$ `.
fn shell_prompt_index(text: &str) -> Option<usize> {
    // Pola user@host:...$ atau user@host:...#
    let bytes = text.as_bytes();
    let mut result = None;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'@' {
            // Cari akhir baris, cek apakah ada '#' atau '$' setelahnya.
            if let Some(nl) = text[i..].find(['\n', '\r']) {
                let line = &text[i..i + nl];
                if line.contains('#') || line.contains('$') {
                    result = Some(i);
                }
            } else {
                let line = &text[i..];
                if line.contains('#') || line.contains('$') {
                    result = Some(i);
                }
            }
        }
    }
    if result.is_some() {
        return result;
    }

    // Fallback: baris terakhir diakhiri `# ` atau `$ `.
    let last_line = text
        .rsplit(['\n', '\r'])
        .find(|l| !l.trim().is_empty())
        .unwrap_or("");
    let trimmed = last_line.trim_end();
    if trimmed.ends_with('#') || trimmed.ends_with('$') {
        // Hindari salah deteksi prompt U-Boot `#`.
        if !trimmed.contains("BOOT") && !trimmed.contains("U-Boot") {
            return Some(text.rfind(trimmed).unwrap_or(0));
        }
    }
    None
}

/// Buang escape sequence ANSI dari teks.
pub fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&n) = chars.peek() {
                    chars.next();
                    if n.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deteksi_login_prompt() {
        assert_eq!(detect_prompt("LEDE login:"), Some(PromptKind::Login));
        assert_eq!(detect_prompt("Username: "), Some(PromptKind::Login));
    }

    #[test]
    fn deteksi_password_prompt() {
        assert_eq!(detect_prompt("Password: "), Some(PromptKind::Password));
    }

    #[test]
    fn deteksi_shell_prompt() {
        assert_eq!(detect_prompt("root@LEDE:/# "), Some(PromptKind::Shell));
        assert!(is_shell_ready("root@LEDE:/# "));
        assert!(is_shell_ready("$ "));
    }

    #[test]
    fn deteksi_uboot_prompt() {
        assert_eq!(detect_prompt("STB-BOOT # "), Some(PromptKind::Uboot));
        assert_eq!(detect_prompt("=> "), Some(PromptKind::Uboot));
        assert!(!is_shell_ready("STB-BOOT # "));
    }

    #[test]
    fn prompt_terakhir_menang() {
        // Login dulu, lalu password muncul terakhir.
        let text = "LEDE login: admin\nPassword: ";
        assert_eq!(detect_prompt(text), Some(PromptKind::Password));
    }

    #[test]
    fn strip_ansi_benar() {
        assert_eq!(strip_ansi("\x1b[32mOK\x1b[0m"), "OK");
        assert_eq!(strip_ansi("plain"), "plain");
    }

    #[test]
    fn plan_steps_lengkap() {
        let plan = LoginPlan::new("root", "toor");
        let steps = plan.steps();
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0].0, PromptKind::Login);
        assert_eq!(steps[0].1.as_deref(), Some(b"root\r".as_slice()));
        assert_eq!(steps[1].0, PromptKind::Password);
        assert_eq!(steps[2].0, PromptKind::Shell);
        assert!(steps[2].1.is_none());
    }

    #[test]
    fn plan_tanpa_kredensial() {
        let plan = LoginPlan::default();
        let steps = plan.steps();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].0, PromptKind::Shell);
    }

    #[test]
    fn enter_bytes_variasi() {
        let mut p = LoginPlan {
            enter: "lf".into(),
            ..LoginPlan::default()
        };
        assert_eq!(p.enter_bytes(), vec![b'\n']);
        p.enter = "crlf".into();
        assert_eq!(p.enter_bytes(), vec![b'\r', b'\n']);
        p.enter = "cr".into();
        assert_eq!(p.enter_bytes(), vec![b'\r']);
    }

    #[test]
    fn roundtrip_serde() {
        let plan = LoginPlan::new("admin", "admin");
        let text = toml::to_string_pretty(&plan).unwrap();
        let back: LoginPlan = toml::from_str(&text).unwrap();
        assert_eq!(plan, back);
    }
}
