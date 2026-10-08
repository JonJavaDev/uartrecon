//! Utilitas tampilan (warna, banner, error).

/// Menampilkan pesan error ke stderr.
pub fn error(msg: &str) {
    eprintln!("error: {msg}");
}

/// Menampilkan banner aplikasi.
pub fn banner() {
    println!("UARTRecon — UART Recon & Analysis Toolkit");
    println!("read-only first · detect · capture · analyze · export");
}

/// Menampilkan header bagian.
pub fn header(title: &str) {
    println!("\n{title}");
    println!("{}", "─".repeat(title.len().max(20)));
}

/// Menampilkan baris "key : value".
pub fn kv(key: &str, value: &str) {
    println!("  {key:<12} : {value}");
}

/// Menampilkan peringatan.
pub fn warn(msg: &str) {
    eprintln!("warning: {msg}");
}
