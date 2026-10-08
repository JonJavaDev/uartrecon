//! Helper i18n untuk GUI.

use std::sync::OnceLock;

use uartrecon_core::i18n::{Key, Lang, tr};

static CURRENT: OnceLock<std::sync::RwLock<Lang>> = OnceLock::new();

fn cell() -> &'static std::sync::RwLock<Lang> {
    CURRENT.get_or_init(|| std::sync::RwLock::new(Lang::default()))
}

/// Mengatur bahasa aktif.
pub fn set_lang(lang: Lang) {
    if let Ok(mut g) = cell().write() {
        *g = lang;
    }
}

/// Bahasa aktif.
pub fn lang() -> Lang {
    cell().read().map(|g| *g).unwrap_or_default()
}

/// Terjemahkan kunci sesuai bahasa aktif.
pub fn t(key: Key) -> &'static str {
    tr(lang(), key)
}
