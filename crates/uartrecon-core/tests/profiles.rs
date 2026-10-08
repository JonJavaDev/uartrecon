//! Integration test: memuat semua profile TOML di direktori `profiles/`.

use std::path::PathBuf;

use uartrecon_core::profiles;

fn profiles_dir() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from("profiles"),
        PathBuf::from("../../profiles"),
        PathBuf::from("../../../profiles"),
    ];
    candidates.into_iter().find(|c| c.exists())
}

#[test]
fn semua_profile_toml_valid() {
    let dir = match profiles_dir() {
        Some(d) => d,
        None => {
            eprintln!("skip: direktori profiles/ tidak ditemukan");
            return;
        }
    };

    let loaded = profiles::load_dir(&dir).expect("load_dir harus sukses");
    assert!(
        loaded.len() >= 4,
        "harus ada minimal 4 profile, dapat {}",
        loaded.len()
    );

    // Setiap profile harus punya nama & deskripsi.
    for p in &loaded {
        assert!(!p.name.is_empty(), "profile tanpa nama: {:?}", p);
        assert!(
            !p.description.is_empty(),
            "profile tanpa deskripsi: {}",
            p.name
        );
    }

    // Profile yang diharapkan ada.
    let names: Vec<&str> = loaded.iter().map(|p| p.name.as_str()).collect();
    for expected in ["uboot", "linux", "busybox", "stb"] {
        assert!(
            names.contains(&expected),
            "profile '{expected}' tidak ditemukan"
        );
    }
}

#[test]
fn builtin_profile_konsisten() {
    let builtin = profiles::Profile::builtin();
    assert_eq!(builtin.len(), 3);
    for p in builtin {
        assert!(!p.suggested_baudrates.is_empty());
    }
}
