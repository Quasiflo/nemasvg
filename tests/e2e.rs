//! End-to-end: real asciinema recordings in `fixtures/` convert to SVG.
//!
//! Each fixture has a committed `.svg` golden. Tests regenerate the SVG
//! in memory and byte-compare: any renderer change shows up as an explicit
//! golden diff to review. Re-bless deliberately after review with:
//!
//! ```sh
//! UPDATE_GOLDENS=1 cargo test --test e2e
//! ```
//!
//! Fixtures are re-recorded with `scripts/record-fixtures.sh` (needs the
//! asciinema dev tool). Re-recording changes timing, so goldens must be
//! re-blessed afterwards — never bless blindly; inspect the diff first.
//!
//! Alongside the byte comparison, every fixture asserts semantic markers
//! (`must_contain`) so a stale-but-identical golden still means something.

use std::path::PathBuf;

use nemasvg::{Options, generate};

struct Fixture {
    name: &'static str,
    options: Options,
    must_contain: &'static [&'static str],
}

fn windowed() -> Options {
    Options {
        window: true,
        ..Options::default()
    }
}

fn fixtures() -> Vec<Fixture> {
    vec![
        Fixture {
            name: "colors",
            options: Options::default(),
            must_contain: &["Bold Red", "truecolor", "done"],
        },
        Fixture {
            name: "colors-window",
            options: windowed(),
            must_contain: &["Bold Red", "Terminal"],
        },
        Fixture {
            name: "altscreen",
            options: Options::default(),
            must_contain: &["before", "back"],
        },
        Fixture {
            name: "vim",
            options: Options::default(),
            must_contain: &["vim-work.txt"],
        },
        Fixture {
            name: "unicode",
            options: Options::default(),
            must_contain: &["CJK:"],
        },
        Fixture {
            name: "cursor",
            options: Options::default(),
            must_contain: &["TOP", "HELLO", "done"],
        },
        Fixture {
            name: "scroll",
            options: Options::default(),
            must_contain: &["35"],
        },
        Fixture {
            name: "progress",
            options: Options::default(),
            must_contain: &["100%", "finished"],
        },
        Fixture {
            name: "idle",
            options: Options::default(),
            must_contain: &["start", "end"],
        },
    ]
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// `colors-window` reuses the `colors` recording with different options.
fn cast_name(name: &str) -> &str {
    name.strip_suffix("-window").unwrap_or(name)
}

#[test]
fn real_recordings_match_goldens() {
    let dir = fixture_dir();
    let bless = std::env::var("UPDATE_GOLDENS").is_ok();
    for fx in fixtures() {
        let cast = std::fs::read_to_string(dir.join(format!("{}.cast", cast_name(fx.name))))
            .unwrap_or_else(|e| panic!("fixture {}.cast missing: {e}", fx.name));
        let svg = generate(&cast, &fx.options)
            .unwrap_or_else(|e| panic!("fixture {} failed to convert: {e}", fx.name));
        assert!(
            svg.starts_with("<svg xmlns"),
            "{}: output is not an SVG document",
            fx.name
        );
        for marker in fx.must_contain {
            assert!(
                svg.contains(marker),
                "{}: expected rendered output to contain {marker:?}",
                fx.name
            );
        }
        let golden_path = dir.join(format!("{}.svg", fx.name));
        if bless {
            std::fs::write(&golden_path, &svg).unwrap();
            println!("blessed {}", golden_path.display());
            continue;
        }
        let golden = std::fs::read_to_string(&golden_path).unwrap_or_else(|e| {
            panic!(
                "golden {}.svg missing ({e}); bless with UPDATE_GOLDENS=1",
                fx.name
            )
        });
        assert!(
            svg == golden,
            "{}: output differs from golden ({} vs {} bytes). \
             Inspect the diff; if the change is intended, re-bless with \
             UPDATE_GOLDENS=1 cargo test --test e2e",
            fx.name,
            svg.len(),
            golden.len()
        );
    }
}
