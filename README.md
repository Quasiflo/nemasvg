# Nemasvg

Asciinema v3 recordings become sharp, self-contained animated SVGs. No JavaScript, no video encodes, no external assets — a single `.svg` that plays anywhere an image renders, from GitHub READMEs to package pages.

![Terminal session converted to animated SVG](https://raw.githubusercontent.com/Quasiflo/nemasvg/main/docs/assets/hero.svg)

## Why This Exists

GIFs of terminals are blurry and heavy. The asciinema player needs JavaScript, which READMEs and registries strip out. Nemasvg sits in the remaining sweet spot: vector-sharp text, tiny files, and zero-runtime playback driven by one CSS animation.

![Text styles and terminal colors](https://raw.githubusercontent.com/Quasiflo/nemasvg/main/docs/assets/colors.svg)

## Install

```sh
cargo install --git https://github.com/Quasiflo/nemasvg
```

Or build from source with a pinned toolchain via `mise` (`mise install` picks up Rust from `.config/mise.toml`):

```sh
cargo build --release
```

## Usage

Record with the asciinema CLI, then convert:

```sh
asciinema rec demo.cast
nemasvg demo.cast demo.svg --window
```

Old v1/v2 recordings are rejected on purpose — convert them first:

```sh
asciinema convert old.cast demo.cast
```

Pipes work in both directions:

```sh
cat demo.cast | nemasvg - - > demo.svg
```

### Excerpts and Stills

```sh
nemasvg demo.cast still.svg --at 4.5
nemasvg demo.cast excerpt.svg --from 3 --to 12 --speed 1.5 --no-cursor
```

![Animated progress bar with right-anchored percentage](https://raw.githubusercontent.com/Quasiflo/nemasvg/main/docs/assets/progress.svg)

### Options

| Flag | Effect |
| ---- | ------ |
| `--speed <N>` | Playback speed multiplier (default 1) |
| `--fps <N>` | Max visual frames per second, 0 uncaps (default 30) |
| `--idle-time-limit <S>` | Cap idle gaps; defaults to the recording header's limit |
| `--cols/--rows <N>` | Pin terminal dimensions (also `--width/--height`) |
| `--theme <NAME>` | Builtin theme; defaults to the recording's embedded theme, then dracula (`--list-themes` lists them) |
| `--font-size/--line-height/--padding` | Terminal geometry tuning |
| `--at/--from/--to <S>` | Static frame or animated excerpt in seconds |
| `--no-cursor`, `--no-loop`, `--window`, `--title` | Presentation toggles |
| `--no-embed-fonts` | Skip font embedding; rely on viewer system fonts |
| `--embed-emoji` | Embed monochrome Noto Emoji for glyphs the mono faces lack |

Run `nemasvg --help` for the authoritative list.

## Themes and Fonts

Six builtin themes ship with the binary (dracula, asciinema, monokai, solarized-dark, solarized-light, github-dark). A recording's own embedded theme wins unless `--theme` overrides it.

Fonts are embedded by default: each conversion subsets bundled JetBrains Mono (plus monochrome Noto Emoji with `--embed-emoji`) down to the glyphs the recording actually uses and inlines them as WOFF2, so output looks identical on every machine. Color emoji always falls back to OS fonts. Pass `--no-embed-fonts` to skip embedding entirely.

![CJK, emoji, and box drawing](https://raw.githubusercontent.com/Quasiflo/nemasvg/main/docs/assets/unicode.svg)

## Development

```sh
cargo test       # behavior-first suite plus e2e goldens
cargo clippy --all-targets
cargo fmt --check
```

End-to-end fixtures in `tests/fixtures/` are real asciinema captures. Re-record them with `scripts/record-fixtures.sh` (needs the asciinema mise tool) and re-bless goldens with `UPDATE_GOLDENS=1 cargo test --test e2e` after reviewing each diff. README demos regenerate the same way via `scripts/generate-demos.sh`; intermediate `.cast` files are temp-only, `docs/assets/*.svg` is what ships.

## License

MIT. Bundled fonts live under their own licenses in `fonts/` (JetBrains Mono and Noto Emoji are SIL Open Font License 1.1).
