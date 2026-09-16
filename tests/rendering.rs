//! Rendering contracts: themes, styles, cursor, chrome, compactness,
//! escaping. What the viewer sees must match terminal semantics.

mod common;

use common::{Cast, opts};
use nemasvg::generate;

fn svg(cast: &str, options: &nemasvg::Options) -> String {
    generate(cast, options).expect("fixture must convert")
}

fn dracula() -> String {
    Cast::new(40, 8).event(0.1, "o", "hi").build()
}

#[test]
fn default_theme_is_dracula() {
    let out = svg(&dracula(), &opts());
    assert!(out.contains("fill=\"#282a36\""), "dracula background");
}

#[test]
fn plain_text_inherits_theme_foreground() {
    // Regression: without an inherited fill, plain text renders black.
    let out = svg(&dracula(), &opts());
    assert!(out.contains("fill=\"#f8f8f2\""), "reel must carry theme fg");
}

#[test]
fn cli_theme_overrides_embedded_theme() {
    let cast = Cast::header_raw(
        r##"{"version":3,"term":{"cols":40,"rows":8,"theme":{"fg":"#ffffff","bg":"#000000","palette":"#000000:#ff0000:#00ff00:#ffff00:#0000ff:#ff00ff:#00ffff:#ffffff:#888888:#ff8888:#88ff88:#ffff88:#8888ff:#ff88ff:#88ffff:#ffffff"}}}"##,
    )
    .event(0.1, "o", "hi")
    .build();
    let plain = svg(&cast, &opts());
    assert!(
        plain.contains("fill=\"#000000\""),
        "embedded theme wins by default"
    );
    let mut o = opts();
    o.theme = Some("monokai".to_owned());
    let themed = svg(&cast, &o);
    assert!(
        themed.contains("fill=\"#272822\""),
        "CLI theme overrides embedded"
    );
}

#[test]
fn bold_red_resolves_to_bright_palette() {
    let cast = Cast::new(40, 8)
        .event(0.1, "o", "\u{1b}[1;31mR\u{1b}[0m")
        .build();
    let out = svg(&cast, &opts());
    assert!(out.contains("fill:#ff6e6e;font-weight:bold"), "{out}");
}

#[test]
fn faint_renders_without_opacity_property() {
    // opacity on <text> crashes resvg; faint must use fill-opacity.
    let cast = Cast::new(40, 8)
        .event(0.1, "o", "\u{1b}[2mfaint\u{1b}[0m")
        .build();
    let out = svg(&cast, &opts());
    assert!(out.contains("fill-opacity:.55"), "{out}");
    assert!(!out.contains(";opacity:"), "no opacity property anywhere");
}

#[test]
fn text_decorations_map_to_css() {
    let cast = Cast::new(60, 8)
        .event(
            0.1,
            "o",
            "\u{1b}[3mi\u{1b}[0m\u{1b}[4mu\u{1b}[0m\u{1b}[9ms\u{1b}[0m",
        )
        .build();
    let out = svg(&cast, &opts());
    assert!(out.contains("font-style:italic"));
    assert!(out.contains("text-decoration:underline"));
    assert!(out.contains("text-decoration:line-through"));
}

#[test]
fn inverse_swaps_foreground_and_background() {
    let cast = Cast::new(40, 8)
        .event(0.1, "o", "\u{1b}[7minv\u{1b}[0m")
        .build();
    let out = svg(&cast, &opts());
    // Background rect painted in theme fg, glyphs in theme bg.
    assert!(out.contains("fill=\"#f8f8f2\""));
    assert!(out.contains("fill:#282a36"));
}

#[test]
fn palette_256_and_truecolor_resolve() {
    let cast = Cast::new(60, 8)
        .event(
            0.1,
            "o",
            "\u{1b}[38;5;200mp\u{1b}[0m\u{1b}[38;2;10;20;30mq\u{1b}[0m",
        )
        .build();
    let out = svg(&cast, &opts());
    assert!(out.contains("fill:#ff00d7"), "256-color cube");
    assert!(out.contains("fill:#0a141e"), "truecolor passthrough");
}

#[test]
fn cursor_block_shows_by_default() {
    let out = svg(&dracula(), &opts());
    // Cursor after "hi" sits at column 2, row 0: one cell block in theme fg.
    assert!(
        out.contains("<rect x=\"20\" y=\"0\" width=\"10\" height=\"22\""),
        "{out}"
    );
}

#[test]
fn no_cursor_hides_the_block() {
    let mut o = opts();
    o.no_cursor = true;
    let out = svg(&dracula(), &o);
    assert!(!out.contains("width=\"10\" height=\"22\""), "{out}");
}

#[test]
fn terminal_hidden_cursor_is_honored() {
    let cast = Cast::new(40, 8)
        .event(0.1, "o", "\u{1b}[?25lhidden")
        .build();
    let out = svg(&cast, &opts());
    assert!(!out.contains("width=\"10\" height=\"22\""), "{out}");
}

#[test]
fn window_chrome_uses_recording_title() {
    let cast =
        Cast::header_raw(r#"{"version":3,"term":{"cols":40,"rows":8},"title":"Demo Title"}"#)
            .event(0.1, "o", "hi")
            .build();
    let mut o = opts();
    o.window = true;
    let out = svg(&cast, &o);
    assert!(out.contains("Demo Title"));
    assert!(out.contains("<circle"), "traffic lights");
}

#[test]
fn cli_title_overrides_recording_title() {
    let cast =
        Cast::header_raw(r#"{"version":3,"term":{"cols":40,"rows":8},"title":"Demo Title"}"#)
            .event(0.1, "o", "hi")
            .build();
    let mut o = opts();
    o.window = true;
    o.title = Some("Custom".to_owned());
    let out = svg(&cast, &o);
    assert!(out.contains("Custom") && !out.contains("Demo Title"));
}

#[test]
fn shared_rows_are_interned_once() {
    // Two frames sharing the "same" row: one def, two uses.
    // (\r\n, like real pty output: bare \n would stair-step mid-line.)
    let cast = Cast::new(40, 8)
        .event(0.5, "o", "same\r\n")
        .event(1.0, "o", "different\r\n")
        .build();
    let out = svg(&cast, &opts());
    assert!(out.contains("<defs>"), "shared rows live in defs");
    assert_eq!(out.matches("<use href=").count(), 3, "2 rows + carried row");
    assert_eq!(out.matches("<g id=\"r").count(), 2, "same + different");
}

#[test]
fn repeated_runs_share_one_def() {
    // The same styled run inside two differently-shaped rows: one run def,
    // one reference per unique row. (A run recurring only inside identical
    // rows is already covered by the shared row entry.)
    // (\r\n, like real pty output: bare \n would stair-step mid-line.)
    let cast = Cast::new(80, 8)
        .event(
            0.5,
            "o",
            "\u{1b}[7m worms worms worms worms worms\u{1b}[0m\r\n",
        )
        .event(
            0.5,
            "o",
            "\u{1b}[7m worms worms worms worms worms\u{1b}[0m bait\r\n",
        )
        .build();
    let out = svg(&cast, &opts());
    assert_eq!(out.matches("<text id=\"c").count(), 1, "the worms run");
    assert_eq!(out.matches("<use href=\"#c").count(), 2, "one ref per row");
}

#[test]
fn single_use_runs_stay_inline() {
    // No repetition: output keeps plain inline runs, no indirection.
    let cast = Cast::new(40, 8).event(0.1, "o", "one-off\n").build();
    let out = svg(&cast, &opts());
    assert!(!out.contains("<use href=\"#c"), "nothing to share");
    assert!(!out.contains("<use href=\"#b"), "nothing to share");
}

#[test]
fn markup_is_escaped() {
    let cast = Cast::new(40, 8).event(0.1, "o", "<b>&\"'").build();
    let out = svg(&cast, &opts());
    assert!(out.contains("&lt;b&gt;&amp;&quot;&apos;"), "{out}");
}

#[test]
fn wide_chars_advance_two_columns() {
    // Style change after the wide char forces a new run at the true column.
    let cast = Cast::new(40, 8)
        .event(0.1, "o", "ab\u{65e5}\u{1b}[31mcd\u{1b}[0m")
        .build();
    let out = svg(&cast, &opts());
    // a, b, wide = columns 0..4, so "cd" starts at x 40.
    assert!(out.contains("<text x=\"40\""), "{out}");
}

#[test]
fn letter_spacing_compensates_column_rounding() {
    // 16px font: 10px columns vs 9.6px advances -> 0.4px per character.
    // Constant glyph size everywhere (no per-run scaling, which pulsed).
    let cast = Cast::new(40, 8).event(0.1, "o", "hi").build();
    let out = svg(&cast, &opts());
    assert!(out.contains("letter-spacing:0.4px"), "{out}");
    assert!(!out.contains("textLength"), "no per-run scaling");
}

#[test]
fn whitespace_survives_into_defs() {
    // Text is defined in <defs> and cloned via <use>; xml:space must cover
    // the definitions, otherwise leading/interior spaces are stripped at
    // parse time (HELLO lost its indent, progress % drifted).
    let cast = Cast::new(40, 8).event(0.1, "o", "  pad  1").build();
    let out = svg(&cast, &opts());
    assert!(out.contains(">  pad  1<"), "spaces intact in markup");
    assert!(
        out.contains("<svg xmlns=\"http://www.w3.org/2000/svg\" width="),
        "{out}"
    );
    let root = out.split('>').next().unwrap_or_default();
    assert!(
        root.contains("xml:space=\"preserve\""),
        "root must preserve space: {root}"
    );
    // CSS-level pin: Chromium strips edge spaces in use-cloned SVG text
    // unless white-space:pre, whatever xml:space says.
    assert!(out.contains("white-space:pre"), "{out}");
}

#[test]
fn trailing_blanks_are_not_emitted_as_text() {
    let cast = Cast::new(40, 8).event(0.1, "o", "hi").build();
    let out = svg(&cast, &opts());
    assert!(!out.contains("hi "), "no trailing space runs in text");
}

#[test]
fn pinned_geometry_scales_dimensions() {
    let mut o = opts();
    o.font_size = 20;
    o.line_height = 1.5;
    o.padding = 5;
    o.cols = Some(10);
    o.rows = Some(4);
    let out = svg(&dracula(), &o);
    // col 12px, row 30px, +10 padding.
    assert!(out.contains("width=\"130\""), "{out}");
    assert!(out.contains("height=\"130\""), "{out}");
}
