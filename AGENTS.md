# AGENTS.md

Asciinema v3 -> animated SVG converter (CLI + library). v3-only by design — v1/v2 are rejected, users convert via `asciinema convert`. No WASM/Node targets, native binaries only.

## Layout

- `src/lib.rs` — `Options` + `generate(cast: &str, &Options) -> String`, the library entry; `main.rs` is a thin clap wrapper.
- Pipeline: `asciicast.rs` (strict v3 parse, delta→absolute clock) -> `terminal.rs` (thin `avt` boundary, owns no terminal logic) -> `timeline.rs` (idle-cap ÷ speed → dedup → anchored FPS windows → `at`/`from`/`to`) -> `renderer.rs` (reel SVG) -> `theme.rs` / `input.rs` (zstd auto-detect).
- SVG encoding: distinct frames on a horizontal reel, one discrete CSS animation (`steps(1,end)`), rows interned in `<defs>` shared via `<use>`. No JS. Faint uses `fill-opacity` (not `opacity`: `opacity` on `<text>` crashes resvg). Plain text inherits `fill` from the reel `<g>` — never rely on the black default.
- Hardening invariants: `Options::validate()` is the single gate (rejects bad speed/dims/idle/times, `from>to`, non-allowlisted font stacks — font CSS is injection-sensitive); terminal dims capped at 4096 in header + resize parsing; zero-delta frames replace rather than duplicate keyframe stops.
- Grid alignment without embedded fonts: `letter-spacing` compensates the assumed 0.6 advance ratio. Never use per-run `textLength` — viewer support varies and identical glyphs pulse between frames. Both are interim until font embedding lands.
- Font embedding is a planned later post-pass; renderer currently assumes viewer system fonts.

## Toolchain

- `mise` manages tools from `.config/mise.toml` (auto-loaded): Rust 1.98.1, `hk`, `rumdl`, `zizmor`. Run `mise install` after clone.
- Crate edition 2024, single binary crate, entrypoint `src/main.rs`.

## Commands

- `cargo run` / `cargo build` — build/run
- `cargo test` — behavior-first suite: `tests/errors.rs` (rejections), `tests/timeline.rs` (clock/FPS/seeks), `tests/rendering.rs` (themes/styles/cursor/chrome), `tests/input.rs` (file/zstd), `tests/e2e.rs` (real recordings vs committed `.svg` goldens), plus unit tests in `theme.rs`. Fixture builder + SVG inspectors in `tests/common/`.
- E2E fixtures in `tests/fixtures/` are real `asciinema rec` captures (re-record via `scripts/record-fixtures.sh`, needs the asciinema mise tool). Tests only read them; timing nondeterminism never leaks in. Re-bless goldens with `UPDATE_GOLDENS=1 cargo test --test e2e` only after reviewing the diff.
- `cargo clippy --all-targets` — must pass clean
- `cargo fmt --check` / `cargo fmt` — formatter gate

## Checks (Git Hooks + CI Equivalent)

- Git hooks via `hk` (`.config/hk.pkl`): `rumdl`, `zizmor`, `cargo clippy`, `cargo fmt`, `cargo deny`. Run `hk check` before pushing; it no-ops when no files changed.
- No `deny.toml`/`rust-toolchain.toml` in repo yet — `cargo deny` uses defaults.
- Markdown lint via `rumdl` (`.config/rumdl.toml`, `MD013` disabled). VS Code runs it on save.
- GitHub Actions lint via `zizmor`; only workflow is `release-please.yml`.

## Conventions

- Use Conventional Commits — required by release-please (`release-type: rust`, config in `.config/rp-config.json` / `rp-manifest.json`). Do not hand-edit versions/tags; release-please owns them.
- Renovate manages deps (semantic commits, 4-day minimum release age, `lockFileMaintenance` off).
- Markdown files use continuous long lines (editor wraps visually); do not hard-wrap.
