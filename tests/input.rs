//! Input handling: plain files, zstd auto-detect, missing files.

fn tmp(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("nemasvg-test-{name}-{}.cast", std::process::id()))
}

fn minimal_cast() -> String {
    "{\"version\":3,\"term\":{\"cols\":40,\"rows\":8}}\n[0.1,\"o\",\"hi\"]\n".to_owned()
}

#[test]
fn plain_file_reads_back() {
    let path = tmp("plain");
    std::fs::write(&path, minimal_cast()).unwrap();
    let text = nemasvg::read_cast_text(path.to_str().unwrap()).unwrap();
    assert!(text.contains("\"version\":3"));
    std::fs::remove_file(&path).ok();
}

#[test]
fn zstd_magic_is_detected_not_extension() {
    // Compressed bytes under a non-.zst name must still decompress.
    let path = tmp("magic");
    let bytes = zstd::encode_all(minimal_cast().as_bytes(), 3).unwrap();
    assert!(bytes.starts_with(&[0x28, 0xB5, 0x2F, 0xFD]));
    std::fs::write(&path, bytes).unwrap();
    let text = nemasvg::read_cast_text(path.to_str().unwrap()).unwrap();
    assert!(text.contains("\"version\":3"));
    std::fs::remove_file(&path).ok();
}

#[test]
fn zstd_file_converts_end_to_end() {
    let path = tmp("e2e");
    let svg_path = tmp("e2e-svg");
    let bytes = zstd::encode_all(minimal_cast().as_bytes(), 3).unwrap();
    std::fs::write(&path, bytes).unwrap();
    let text = nemasvg::read_cast_text(path.to_str().unwrap()).unwrap();
    let svg = nemasvg::generate(&text, &nemasvg::Options::default()).unwrap();
    std::fs::write(&svg_path, &svg).unwrap();
    assert!(svg.contains("hi"));
    std::fs::remove_file(&path).ok();
    std::fs::remove_file(&svg_path).ok();
}

#[test]
fn non_utf8_is_rejected() {
    let path = tmp("binary");
    std::fs::write(&path, [0xff, 0xfe, 0x00]).unwrap();
    let err = nemasvg::read_cast_text(path.to_str().unwrap()).expect_err("must fail");
    assert!(err.to_string().contains("UTF-8"), "{err}");
    std::fs::remove_file(&path).ok();
}
