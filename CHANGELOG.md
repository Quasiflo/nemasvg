# Changelog

## [0.1.0] - 2026-09-16

Initial release: NemaSVG converts asciicast v3 recordings into self-contained animated SVGs with no player, no JavaScript, and no external requests.

- Strict asciicast v3 parsing (v1/v2 are rejected with a pointer to `asciinema convert`), with delta-to-absolute clock accumulation and hardened validation of terminal sizes, resize events, embedded themes, and CLI options.
- Playback timeline with idle-gap capping, speed scaling, duplicate-state dedup, zero-delta burst collapsing, anchored FPS windows, and `--at`/`--from`/`--to` stills and excerpts.
- Reel SVG renderer: distinct frames on a horizontal reel driven by one discrete CSS animation (`steps(1,end)`), rows interned in `<defs>` and shared via `<use>`, repeated runs interned a level deeper on exact byte economics; 16/256/true colors, text styles, cursor, alternate screen, resize, Unicode, and emoji.
- Six builtin themes (dracula, asciinema, monokai, solarized-dark, solarized-light, github-dark); a recording's embedded theme wins unless `--theme` overrides it.
- Fonts embedded by default: bundled JetBrains Mono Regular/Bold (plus opt-in monochrome Noto Emoji via `--embed-emoji`) subset to used glyphs and inlined as WOFF2 data URIs; `--no-embed-fonts` opts out to viewer system fonts.
- Presentation options: macOS-style window chrome with title, loop/reduced-motion handling, terminal geometry tuning (`--cols`/`--rows`, `--font-size`, `--line-height`, `--padding`), and local `.cast`, zstd-compressed, or stdin inputs with file/stdout outputs.
