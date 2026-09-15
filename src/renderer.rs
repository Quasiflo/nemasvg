use std::collections::HashMap;
use std::fmt::Write as _;

use avt::{Color, Line, terminal::Cursor};

use crate::Options;
use crate::asciicast::Header;
use crate::theme::{Theme, rgb_hex};
use crate::timeline::Timeline;

/// Extra hold on the final frame before looping, in seconds.
const TAIL_HOLD: f64 = 1.0;

/// Resolved per-cell style used for run grouping and CSS classes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Style {
    fg: String,
    bold: bool,
    faint: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
}

impl Style {
    fn is_plain(&self, theme: &Theme) -> bool {
        self.fg == theme.fg
            && !self.bold
            && !self.faint
            && !self.italic
            && !self.underline
            && !self.strikethrough
    }

    fn css(&self) -> String {
        let mut out = format!("fill:{}", self.fg);
        if self.bold {
            out.push_str(";font-weight:bold");
        }
        if self.faint {
            // fill-opacity (not opacity): identical for fill-only text, and
            // opacity on <text> crashes some renderers (e.g. resvg).
            out.push_str(";fill-opacity:.55");
        }
        if self.italic {
            out.push_str(";font-style:italic");
        }
        if self.underline && self.strikethrough {
            out.push_str(";text-decoration:underline line-through");
        } else if self.underline {
            out.push_str(";text-decoration:underline");
        } else if self.strikethrough {
            out.push_str(";text-decoration:line-through");
        }
        out
    }
}

struct Ctx<'a> {
    theme: &'a Theme,
    options: &'a Options,
    col_w: u32,
    row_h: u32,
    content_w: u32,
    content_h: u32,
    ox: u32,
    oy: u32,
    total_w: u32,
    total_h: u32,
    bar_h: u32,
    baseline_dy: u32,
}

/// Render a played timeline into a self-contained animated SVG document.
///
/// Encoding: distinct frames sit side-by-side on a horizontal reel; a single
/// discrete CSS animation translates the reel by whole-frame steps, and
/// identical rows are stored once in `<defs>` and shared via `<use>`.
/// No JavaScript, no external references, so the file animates inside
/// `<img>`/README sandboxes.
pub fn render(timeline: &Timeline, theme: &Theme, header: &Header, options: &Options) -> String {
    let font_size = options.font_size;
    let col_w = (font_size as f32 * 0.6).round().max(1.0) as u32;
    let row_h = (font_size as f32 * options.line_height).round() as u32;
    let row_h = row_h.max(font_size);
    let content_w = timeline.cols as u32 * col_w;
    let content_h = timeline.rows as u32 * row_h;
    let bar_h = if options.window { font_size * 2 } else { 0 };
    let ox = options.padding;
    let oy = options.padding + bar_h;

    // Baseline leaves a small descent gap so glyphs don't touch the row below.
    let baseline_dy = font_size + (row_h - font_size) / 2 - 2.min(font_size / 2);

    let ctx = Ctx {
        theme,
        options,
        col_w,
        row_h,
        content_w,
        content_h,
        ox,
        oy,
        total_w: content_w + options.padding * 2,
        total_h: content_h + options.padding * 2 + bar_h,
        bar_h,
        baseline_dy,
    };

    let animated = timeline.frames.len() > 1 && options.at.is_none();
    let play_len = if animated {
        timeline.duration.max(last_time(timeline)) + TAIL_HOLD
    } else {
        0.0
    };

    // First pass: collect every non-plain style so classes are stable.
    let mut styles: HashMap<Style, usize> = HashMap::new();
    for frame in &timeline.frames {
        collect_styles(&frame.snap.lines, &ctx, &mut styles);
    }
    let mut classes: Vec<(usize, Style)> = styles.into_iter().map(|(s, i)| (i, s)).collect();
    classes.sort_by_key(|(i, _)| *i);

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\" role=\"img\">",
        w = ctx.total_w,
        h = ctx.total_h,
    ));
    out.push_str("<style>");
    write!(
        out,
        "text{{font-family:{};font-size:{}px;font-variant-ligatures:none;font-kerning:none}}",
        ctx.options.font_family, font_size,
    )
    .unwrap();
    for (i, style) in &classes {
        write!(out, ".s{i}{{{}}}", style.css()).unwrap();
    }
    if animated {
        write!(
            out,
            ".r{{animation:reel {}s steps(1,end) {}}}@keyframes reel{{{}}}@media (prefers-reduced-motion:reduce){{.r{{animation:none}}}}",
            fmt_num(play_len),
            if options.no_loop { "1 forwards" } else { "infinite" },
            keyframes(timeline, &ctx, play_len),
        )
        .unwrap();
    }
    out.push_str("</style>");
    write!(
        out,
        "<rect width=\"{w}\" height=\"{h}\" fill=\"{bg}\"/>",
        w = ctx.total_w,
        h = ctx.total_h,
        bg = theme.bg,
    )
    .unwrap();

    if options.window {
        render_chrome(&mut out, &ctx, header);
    }

    // Row interning: identical rows are emitted once into <defs> and shared
    // via <use>. Rows are position-independent (relative y); each <use>
    // places one at its frame offset and row. Empty rows emit nothing.
    let lookup = styles_lookup(&classes);
    let mut registry: HashMap<String, usize> = HashMap::new();
    let mut defs: Vec<String> = Vec::new();
    let mut frame_rows: Vec<Vec<Option<usize>>> = Vec::with_capacity(timeline.frames.len());
    for frame in &timeline.frames {
        let lines = &frame.snap.lines;
        let mut ids: Vec<Option<usize>> = Vec::new();
        for line in lines.iter().take(ctx_rows(&ctx)) {
            let markup = render_row(line, &ctx, &lookup);
            if markup.is_empty() {
                ids.push(None);
                continue;
            }
            let id = *registry.entry(markup.clone()).or_insert_with(|| {
                defs.push(markup);
                defs.len() - 1
            });
            ids.push(Some(id));
        }
        frame_rows.push(ids);
    }

    write!(
        out,
        "<svg x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" overflow=\"hidden\">",
        x = ctx.ox,
        y = ctx.oy,
        w = ctx.content_w,
        h = ctx.content_h,
    )
    .unwrap();
    if !defs.is_empty() {
        out.push_str("<defs>");
        for (id, markup) in defs.iter().enumerate() {
            write!(out, "<g id=\"r{id}\">{markup}</g>").unwrap();
        }
        out.push_str("</defs>");
    }
    if animated {
        write!(
            out,
            "<g class=\"r\" xml:space=\"preserve\" fill=\"{fg}\">",
            fg = theme.fg
        )
        .unwrap();
    } else {
        write!(
            out,
            "<g xml:space=\"preserve\" fill=\"{fg}\">",
            fg = theme.fg
        )
        .unwrap();
    }
    if timeline.frames.is_empty() {
        out.push_str("<g></g>");
    }
    for (i, frame) in timeline.frames.iter().enumerate() {
        write!(
            out,
            "<g transform=\"translate({})\">",
            i as u32 * ctx.content_w
        )
        .unwrap();
        for (row, id) in frame_rows[i].iter().enumerate() {
            if let Some(id) = id {
                write!(
                    out,
                    "<use href=\"#r{id}\" y=\"{}\"/>",
                    row as u32 * ctx.row_h
                )
                .unwrap();
            }
        }
        render_cursor(&mut out, &frame.snap.cursor, &ctx);
        out.push_str("</g>");
    }
    out.push_str("</g></svg></svg>");
    out
}

fn last_time(timeline: &Timeline) -> f64 {
    timeline.frames.last().map_or(0.0, |f| f.time)
}

fn styles_lookup(classes: &[(usize, Style)]) -> HashMap<&Style, usize> {
    classes.iter().map(|(i, s)| (s, *i)).collect()
}

/// Resolve an `avt` color against the theme. `bold` promotes the first 8
/// indexed colors to their bright variants, matching terminal convention.
fn resolve(color: Option<Color>, bold: bool, fallback: &str, theme: &Theme) -> String {
    match color {
        None => fallback.to_owned(),
        Some(Color::Indexed(i)) => {
            let i = if bold && i < 8 { i + 8 } else { i };
            theme.indexed(i)
        }
        Some(Color::RGB(rgb)) => rgb_hex(rgb.r, rgb.g, rgb.b),
    }
}

fn cell_style(cell: &avt::Cell, theme: &Theme) -> (Style, Option<String>) {
    let pen = cell.pen();
    let bold = pen.is_bold();
    let mut fg = resolve(pen.foreground(), bold, &theme.fg, theme);
    let mut bg = resolve(pen.background(), false, &theme.bg, theme);
    if pen.is_inverse() {
        std::mem::swap(&mut fg, &mut bg);
    }
    let bg = (bg != theme.bg).then_some(bg);
    (
        Style {
            fg,
            bold,
            faint: pen.is_faint(),
            italic: pen.is_italic(),
            underline: pen.is_underline(),
            strikethrough: pen.is_strikethrough(),
        },
        bg,
    )
}

fn collect_styles(lines: &[Line], ctx: &Ctx<'_>, styles: &mut HashMap<Style, usize>) {
    for line in lines.iter().take(ctx_rows(ctx)) {
        for cell in line.cells().iter().take(ctx_cols(ctx)) {
            if cell.width() == 0 {
                continue;
            }
            let (style, _) = cell_style(cell, ctx.theme);
            if !style.is_plain(ctx.theme) && !styles.contains_key(&style) {
                let id = styles.len();
                styles.insert(style, id);
            }
        }
    }
}

fn ctx_cols(ctx: &Ctx<'_>) -> usize {
    (ctx.content_w / ctx.col_w.max(1)) as usize
}

fn ctx_rows(ctx: &Ctx<'_>) -> usize {
    (ctx.content_h / ctx.row_h.max(1)) as usize
}

/// Render one terminal row with row-relative coordinates (bg at y=0, text
/// baseline at `baseline_dy`) so identical rows share one `<defs>` entry
/// regardless of position. Returns "" for fully blank rows.
fn render_row(line: &Line, ctx: &Ctx<'_>, lookup: &HashMap<&Style, usize>) -> String {
    let max_cols = ctx_cols(ctx);
    let mut out = String::new();

    // Background pass: merge adjacent cells sharing a background into one rect.
    {
        let mut run_start: Option<(usize, String)> = None;
        let mut col = 0usize;
        let flush = |out: &mut String, run: &mut Option<(usize, String)>, end: usize| {
            if let Some((start, color)) = run.take() {
                write!(
                    out,
                    "<rect x=\"{}\" y=\"0\" width=\"{}\" height=\"{h}\" fill=\"{color}\"/>",
                    start as u32 * ctx.col_w,
                    (end - start) as u32 * ctx.col_w,
                    h = ctx.row_h,
                )
                .unwrap();
            }
        };
        for cell in line.cells().iter().take(max_cols) {
            let w = cell.width();
            if w == 0 {
                continue;
            }
            let (_, bg) = cell_style(cell, ctx.theme);
            match (bg, &mut run_start) {
                (Some(color), Some((_, cur))) if *cur == color => {}
                (Some(color), run) => {
                    flush(&mut out, run, col);
                    *run = Some((col, color));
                }
                (None, run) => flush(&mut out, run, col),
            }
            col += w as usize;
        }
        flush(&mut out, &mut run_start, col);
    }

    // Text pass: group consecutive same-style cells into one <text>.
    // Trailing blanks are invisible, so text runs stop at the last non-space
    // cell (backgrounds were already painted in the pass above).
    {
        let cells = line.cells();
        let mut text_end = 0usize;
        let mut scan_col = 0usize;
        for cell in cells.iter().take(max_cols) {
            let w = cell.width();
            if w == 0 {
                continue;
            }
            if cell.char() != ' ' {
                text_end = scan_col + w as usize;
            }
            scan_col += w as usize;
        }
        let mut run = String::new();
        let mut run_style: Option<Style> = None;
        let mut run_x = 0u32;
        let mut col = 0usize;
        let flush = |out: &mut String, run: &mut String, style: &mut Option<Style>, x: u32| {
            if run.is_empty() || run.trim().is_empty() {
                run.clear();
                *style = None;
                return;
            }
            let style = style.take().expect("run always has a style");
            if style.is_plain(ctx.theme) {
                write!(
                    out,
                    "<text x=\"{x}\" y=\"{dy}\">{}</text>",
                    esc(run),
                    dy = ctx.baseline_dy
                )
                .unwrap();
            } else if let Some(id) = lookup.get(&style) {
                write!(
                    out,
                    "<text x=\"{x}\" y=\"{dy}\" class=\"s{id}\">{}</text>",
                    esc(run),
                    dy = ctx.baseline_dy
                )
                .unwrap();
            }
            run.clear();
        };
        for cell in line.cells().iter().take(max_cols) {
            let w = cell.width();
            if w == 0 {
                continue;
            }
            if col >= text_end {
                break;
            }
            let (style, _) = cell_style(cell, ctx.theme);
            match &run_style {
                Some(cur) if *cur == style => run.push(cell.char()),
                _ => {
                    flush(&mut out, &mut run, &mut run_style, run_x);
                    run_x = col as u32 * ctx.col_w;
                    run.push(cell.char());
                    run_style = Some(style);
                }
            }
            col += w as usize;
        }
        flush(&mut out, &mut run, &mut run_style, run_x);
    }

    out
}

fn render_cursor(out: &mut String, cursor: &Cursor, ctx: &Ctx<'_>) {
    if ctx.options.no_cursor || !cursor.visible {
        return;
    }
    let cols = ctx_cols(ctx).max(1);
    let rows = ctx_rows(ctx).max(1);
    let c = (cursor.col.min(cols - 1) as u32) * ctx.col_w;
    let r = (cursor.row.min(rows - 1) as u32) * ctx.row_h;
    write!(
        out,
        "<rect x=\"{c}\" y=\"{r}\" width=\"{w}\" height=\"{h}\" fill=\"{fg}\"/>",
        w = ctx.col_w,
        h = ctx.row_h,
        fg = ctx.theme.fg,
    )
    .unwrap();
}

fn render_chrome(out: &mut String, ctx: &Ctx<'_>, header: &Header) {
    let bar = ctx.bar_h;
    write!(
        out,
        "<rect width=\"{w}\" height=\"{bar}\" fill=\"#2b2e3b\"/>",
        w = ctx.total_w,
    )
    .unwrap();
    let cy = bar / 2;
    for (i, color) in ["#ff5f57", "#febc2e", "#28c840"].iter().enumerate() {
        let cx = ctx.options.padding + 10 + i as u32 * 18;
        write!(
            out,
            "<circle cx=\"{cx}\" cy=\"{cy}\" r=\"6\" fill=\"{color}\"/>"
        )
        .unwrap();
    }
    let title = ctx
        .options
        .title
        .clone()
        .or_else(|| header.title.clone())
        .unwrap_or_else(|| "Terminal".to_owned());
    write!(
        out,
        "<text x=\"{w}\" y=\"{y}\" text-anchor=\"middle\" fill=\"#e6e6e6\" font-size=\"13\">{}</text>",
        esc(&title),
        w = ctx.total_w / 2,
        y = cy + 5,
    )
    .unwrap();
}

/// Discrete reel keyframes: each frame's offset holds from its start percent
/// until the next frame, enforced by `steps(1,end)` on the animation.
fn keyframes(timeline: &Timeline, ctx: &Ctx<'_>, play_len: f64) -> String {
    let n = timeline.frames.len();
    let mut out = String::new();
    for (i, frame) in timeline.frames.iter().enumerate() {
        let pct = if play_len <= 0.0 {
            0.0
        } else {
            (frame.time / play_len * 100.0).clamp(0.0, 100.0)
        };
        write!(
            out,
            "{}%{{transform:translateX({}px)}}",
            fmt_num(pct),
            -(i as i64) * i64::from(ctx.content_w),
        )
        .unwrap();
    }
    write!(
        out,
        "100%{{transform:translateX({}px)}}",
        -((n.saturating_sub(1)) as i64) * i64::from(ctx.content_w),
    )
    .unwrap();
    out
}

fn fmt_num(v: f64) -> String {
    if v == 0.0 {
        return "0".to_owned();
    }
    let s = format!("{v:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}
