use std::collections::HashMap;
use std::fmt::Write as _;

use avt::{Color, Line, terminal::Cursor};

use crate::Options;
use crate::asciicast::Header;
use crate::fonts::FontPlan;
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
pub fn render(
    timeline: &Timeline,
    theme: &Theme,
    header: &Header,
    options: &Options,
    fonts: &FontPlan,
) -> String {
    let font_size = options.font_size;
    let ratio = fonts.advance_ratio as f32;
    let col_w = (font_size as f32 * ratio).round().max(1.0) as u32;
    let row_h = (font_size as f32 * options.line_height).round() as u32;
    let row_h = row_h.max(font_size);
    let content_w = timeline.cols as u32 * col_w;
    let content_h = timeline.rows as u32 * row_h;
    let bar_h = if options.window { font_size * 2 } else { 0 };
    let ox = options.padding;
    let oy = options.padding + bar_h;

    // Baseline leaves a small descent gap so glyphs don't touch the row below.
    let baseline_dy = font_size + (row_h - font_size) / 2 - 2.min(font_size / 2);

    // Column grid vs font metrics: the remainder between the integer column
    // width and the true advance accumulates along a row, so compensate with
    // letter-spacing. Residual drift on unmeasured fonts is bounded per run
    // (runs restart at absolute x).
    // Deliberately not textLength: viewer support varies, and per-run
    // scaling makes identical glyphs pulse between frames.
    let spacing = col_w as f32 - font_size as f32 * ratio;
    let spacing_css = if spacing == 0.0 {
        String::new()
    } else {
        format!(";letter-spacing:{}px", fmt_num(spacing as f64))
    };

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
    let classes = collect_classes(timeline, &ctx);
    let lookup = styles_lookup(&classes);

    let mut out = String::new();
    // xml:space lives on the root: whitespace is fixed at XML parse time from
    // the in-scope value, and our text is defined in <defs> (cloned via
    // <use> only afterwards). Scoping it to the reel <g> left <defs> with the
    // collapsing default, silently stripping leading/interior spaces.
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\" role=\"img\" xml:space=\"preserve\">",
        w = ctx.total_w,
        h = ctx.total_h,
    ));
    out.push_str("<style>");
    out.push_str(&fonts.face_css);
    // white-space:pre is load-bearing: Chromium strips leading/trailing
    // spaces in SVG text cloned through <use> shadow DOM based on CSS
    // white-space, regardless of xml:space. (xml:space on the root covers
    // non-CSS renderers such as resvg.)
    write!(
        out,
        "text{{font-family:{}{};font-size:{}px{};font-variant-ligatures:none;font-kerning:none;white-space:pre}}",
        fonts.family_prefix, ctx.options.font_family, font_size, spacing_css,
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

    // Two-level interning: identical rows share one `<defs>` entry, and
    // repeated runs inside rows share a deeper entry referenced by
    // position-free `<use>` (x rides on the reference, y is constant).
    // Rows stay position-independent (relative y); each row `<use>` places
    // one at its frame offset and row. Empty rows emit nothing.
    let (run_defs, row_defs, frame_rows) = intern_rows(timeline, &ctx, &lookup);

    write!(
        out,
        "<svg x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" overflow=\"hidden\">",
        x = ctx.ox,
        y = ctx.oy,
        w = ctx.content_w,
        h = ctx.content_h,
    )
    .unwrap();
    if !run_defs.is_empty() || !row_defs.is_empty() {
        out.push_str("<defs>");
        for markup in &run_defs {
            out.push_str(markup);
        }
        for (id, markup) in row_defs.iter().enumerate() {
            write!(out, "<g id=\"r{id}\">{markup}</g>").unwrap();
        }
        out.push_str("</defs>");
    }
    let reel_class = if animated { " class=\"r\"" } else { "" };
    write!(out, "<g{reel_class} fill=\"{fg}\">", fg = theme.fg).unwrap();
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

fn collect_classes(timeline: &Timeline, ctx: &Ctx<'_>) -> Vec<(usize, Style)> {
    let mut styles: HashMap<Style, usize> = HashMap::new();
    for frame in &timeline.frames {
        for line in frame.snap.lines.iter().take(ctx_rows(ctx)) {
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
    let mut classes: Vec<(usize, Style)> = styles.into_iter().map(|(s, i)| (i, s)).collect();
    classes.sort_by_key(|(i, _)| *i);
    classes
}

/// A drawable run with absolute column position.
#[derive(Clone)]
struct Run {
    x: u32,
    kind: RunKind,
}

#[derive(Hash, PartialEq, Eq, Clone)]
enum RunKind {
    Bg { w: u32, color: String },
    Text { style: Style, text: String },
}

impl RunKind {
    /// Position-free identity: same content at any column shares one def.
    /// Tuple of plain data (HashMap keys cannot borrow the enum cleanly).
    fn key(&self) -> (bool, u32, String, Option<Style>, String) {
        match self {
            RunKind::Bg { w, color } => (true, *w, color.clone(), None, String::new()),
            RunKind::Text { style, text } => {
                (false, 0, String::new(), Some(style.clone()), text.clone())
            }
        }
    }

    fn prefix(&self) -> char {
        match self {
            RunKind::Bg { .. } => 'b',
            RunKind::Text { .. } => 'c',
        }
    }
}

fn intern_rows(
    timeline: &Timeline,
    ctx: &Ctx<'_>,
    lookup: &HashMap<&Style, usize>,
) -> (Vec<String>, Vec<String>, Vec<Vec<Option<usize>>>) {
    // Pass 1: structured rows, deduplicated structurally up front. Run
    // frequencies are counted over UNIQUE rows only: a run inside a
    // repeated row emits a single shared reference, so raw occurrences
    // would overcount the savings and intern losers. Exact byte economics
    // per distinct run: interned only when sharing truly saves bytes
    // (sum(inline) > def + sum(refs)), measured with real strings — sparse
    // content keeps today's output byte-for-byte.
    struct RunEntry {
        kind: RunKind,
        inline_total: usize,
        xs: Vec<u32>,
    }
    type RowKey = Vec<(u32, (bool, u32, String, Option<Style>, String))>;
    let mut uniq_rows: Vec<Vec<Run>> = Vec::new();
    let mut uniq_of: HashMap<RowKey, usize> = HashMap::new();
    let mut frame_row_idx: Vec<Vec<Option<usize>>> = Vec::with_capacity(timeline.frames.len());
    for frame in &timeline.frames {
        let mut idxs = Vec::new();
        for line in frame.snap.lines.iter().take(ctx_rows(ctx)) {
            let row = render_row_runs(line, ctx);
            if row.is_empty() {
                idxs.push(None);
                continue;
            }
            let key: RowKey = row.iter().map(|run| (run.x, run.kind.key())).collect();
            let idx = *uniq_of.entry(key).or_insert_with(|| {
                uniq_rows.push(row.clone());
                uniq_rows.len() - 1
            });
            idxs.push(Some(idx));
        }
        frame_row_idx.push(idxs);
    }
    let mut order_of: HashMap<(bool, u32, String, Option<Style>, String), usize> = HashMap::new();
    let mut entries: Vec<RunEntry> = Vec::new();
    for row in &uniq_rows {
        for run in row {
            let key = run.kind.key();
            let inline = run_inline_len(run, ctx, lookup);
            match order_of.get(&key) {
                Some(&i) => {
                    let e = &mut entries[i];
                    e.inline_total += inline;
                    e.xs.push(run.x);
                }
                None => {
                    order_of.insert(key, entries.len());
                    entries.push(RunEntry {
                        kind: run.kind.clone(),
                        inline_total: inline,
                        xs: vec![run.x],
                    });
                }
            }
        }
    }

    // Pass 2: ids for winners only, in first-seen order (no gaps).
    // Single-use runs always stay inline: def + ref can only cost more.
    let mut winners: Vec<usize> = Vec::new();
    for (i, e) in entries.iter().enumerate() {
        if e.xs.len() < 2 {
            continue;
        }
        // Provisional id width for the economics below; final ids are
        // compacted afterwards, and a digit either way only matters on
        // razor-thin margins where either choice is fine.
        let digits = winners.len().to_string().len().max(1);
        let def_len = run_def_len(&e.kind, digits, ctx, lookup);
        let ref_len: usize =
            e.xs.iter()
                .map(|x| run_ref_len(e.kind.prefix(), digits, *x))
                .sum();
        if e.inline_total > def_len + ref_len {
            winners.push(i);
        }
    }
    let mut id_of: HashMap<usize, (char, usize)> = HashMap::new();
    let mut run_defs: Vec<String> = Vec::new();
    let mut counters: HashMap<char, usize> = HashMap::new();
    for i in winners {
        let e = &entries[i];
        let prefix = e.kind.prefix();
        let id = counters.get(&prefix).copied().unwrap_or(0);
        counters.insert(prefix, id + 1);
        id_of.insert(i, (prefix, id));
        run_defs.push(run_def_markup(&e.kind, prefix, id, ctx, lookup));
    }

    // Pass 3: one markup per unique row (shared runs become references,
    // singles stay inline), then frames point at rows exactly as before.
    // The markup registry stays as a safety net: distinct structures can
    // still collide textually only if truly identical.
    let mut row_registry: HashMap<String, usize> = HashMap::new();
    let mut row_defs: Vec<String> = Vec::new();
    let mut row_id_of: Vec<usize> = Vec::with_capacity(uniq_rows.len());
    for row in &uniq_rows {
        let mut markup = String::new();
        for run in row {
            match order_of.get(&run.kind.key()).and_then(|i| id_of.get(i)) {
                Some(&(prefix, id)) => {
                    write!(markup, "<use href=\"#{prefix}{id}\" x=\"{}\"/>", run.x).unwrap();
                }
                None => run_inline_markup(&mut markup, run, ctx, lookup),
            }
        }
        let id = *row_registry.entry(markup.clone()).or_insert_with(|| {
            row_defs.push(markup);
            row_defs.len() - 1
        });
        row_id_of.push(id);
    }
    let frame_rows: Vec<Vec<Option<usize>>> = frame_row_idx
        .into_iter()
        .map(|idxs| {
            idxs.into_iter()
                .map(|idx| idx.map(|idx| row_id_of[idx]))
                .collect()
        })
        .collect();
    (run_defs, row_defs, frame_rows)
}

fn ctx_cols(ctx: &Ctx<'_>) -> usize {
    (ctx.content_w / ctx.col_w.max(1)) as usize
}

fn ctx_rows(ctx: &Ctx<'_>) -> usize {
    (ctx.content_h / ctx.row_h.max(1)) as usize
}

/// Collect one terminal row as structured runs (background pass, then text
/// pass, same grouping as ever). Coordinates stay absolute here; the caller
/// strips positions when interning.
fn render_row_runs(line: &Line, ctx: &Ctx<'_>) -> Vec<Run> {
    let max_cols = ctx_cols(ctx);
    let mut runs = Vec::new();

    // Background pass: merge adjacent cells sharing a background into one rect.
    {
        let mut run_start: Option<(usize, String)> = None;
        let mut col = 0usize;
        let mut flush = |run: &mut Option<(usize, String)>, end: usize| {
            if let Some((start, color)) = run.take() {
                runs.push(Run {
                    x: start as u32 * ctx.col_w,
                    kind: RunKind::Bg {
                        w: (end - start) as u32 * ctx.col_w,
                        color,
                    },
                });
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
                    flush(run, col);
                    *run = Some((col, color));
                }
                (None, run) => flush(run, col),
            }
            col += w as usize;
        }
        flush(&mut run_start, col);
    }

    // Text pass: group consecutive same-style cells into one run.
    // Trailing blanks are invisible, so runs stop at the last non-space
    // cell (backgrounds were already collected in the pass above).
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
        let mut flush = |run: &mut String, style: &mut Option<Style>, x: u32| {
            if run.is_empty() || run.trim().is_empty() {
                run.clear();
                *style = None;
                return;
            }
            let style = style.take().expect("run always has a style");
            runs.push(Run {
                x,
                kind: RunKind::Text {
                    style,
                    text: std::mem::take(run),
                },
            });
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
                    flush(&mut run, &mut run_style, run_x);
                    run_x = col as u32 * ctx.col_w;
                    run.push(cell.char());
                    run_style = Some(style);
                }
            }
            col += w as usize;
        }
        flush(&mut run, &mut run_style, run_x);
    }

    runs
}

/// Inline form of one run (today's exact markup, kept for single-use runs).
fn run_inline_markup(out: &mut String, run: &Run, ctx: &Ctx<'_>, lookup: &HashMap<&Style, usize>) {
    match &run.kind {
        RunKind::Bg { w, color } => {
            write!(
                out,
                "<rect x=\"{}\" y=\"0\" width=\"{w}\" height=\"{h}\" fill=\"{color}\"/>",
                run.x,
                h = ctx.row_h,
            )
            .unwrap();
        }
        RunKind::Text { style, text } => {
            if style.is_plain(ctx.theme) {
                write!(
                    out,
                    "<text x=\"{}\" y=\"{dy}\">{}</text>",
                    run.x,
                    esc(text),
                    dy = ctx.baseline_dy
                )
                .unwrap();
            } else if let Some(id) = lookup.get(style) {
                write!(
                    out,
                    "<text x=\"{}\" y=\"{dy}\" class=\"s{id}\">{}</text>",
                    run.x,
                    esc(text),
                    dy = ctx.baseline_dy
                )
                .unwrap();
            }
        }
    }
}

/// Byte length of the inline form, without building it.
fn run_inline_len(run: &Run, ctx: &Ctx<'_>, lookup: &HashMap<&Style, usize>) -> usize {
    let mut out = String::new();
    run_inline_markup(&mut out, run, ctx, lookup);
    out.len()
}

/// Shared-def form: position-free element addressed by `<use>` (x rides on
/// the reference; y is constant across rows so it stays in the def).
fn run_def_markup(
    kind: &RunKind,
    prefix: char,
    id: usize,
    ctx: &Ctx<'_>,
    lookup: &HashMap<&Style, usize>,
) -> String {
    let mut out = String::new();
    match kind {
        RunKind::Bg { w, color } => {
            write!(
                out,
                "<rect id=\"{prefix}{id}\" y=\"0\" width=\"{w}\" height=\"{h}\" fill=\"{color}\"/>",
                h = ctx.row_h,
            )
            .unwrap();
        }
        RunKind::Text { style, text } => {
            if style.is_plain(ctx.theme) {
                write!(
                    out,
                    "<text id=\"{prefix}{id}\" y=\"{dy}\">{}</text>",
                    esc(text),
                    dy = ctx.baseline_dy
                )
                .unwrap();
            } else if let Some(class) = lookup.get(style) {
                write!(
                    out,
                    "<text id=\"{prefix}{id}\" y=\"{dy}\" class=\"s{class}\">{}</text>",
                    esc(text),
                    dy = ctx.baseline_dy
                )
                .unwrap();
            }
        }
    }
    out
}

fn run_def_len(
    kind: &RunKind,
    id_digits: usize,
    ctx: &Ctx<'_>,
    lookup: &HashMap<&Style, usize>,
) -> usize {
    // Measure with a same-width stand-in id; prefixes are one char each.
    let fake_id: usize = "9".repeat(id_digits).parse().unwrap_or(0);
    run_def_markup(kind, 'q', fake_id, ctx, lookup).len()
}

/// Byte length of `<use href="#p12" x="345"/>` for the given widths.
fn run_ref_len(prefix: char, id_digits: usize, x: u32) -> usize {
    let _ = prefix;
    // "<use href=\"#\" x=\"\"/>" is 19 chars plus id, prefix and x digits.
    19 + 1 + id_digits + x.to_string().len()
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
