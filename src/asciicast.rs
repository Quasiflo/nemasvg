use serde::Deserialize;

use crate::theme::Theme;

/// Raw `term` object from a v3 header.
#[derive(Debug, Deserialize)]
struct TermRaw {
    cols: usize,
    rows: usize,
    #[serde(default)]
    theme: Option<ThemeRaw>,
}

/// Raw `term.theme` object from a v3 header.
#[derive(Debug, Deserialize)]
struct ThemeRaw {
    fg: String,
    bg: String,
    palette: String,
}

/// Raw v3 header. Unknown fields are ignored so newer recorders keep working.
#[derive(Debug, Deserialize)]
struct HeaderRaw {
    version: u8,
    term: TermRaw,
    #[serde(default)]
    idle_time_limit: Option<f64>,
    #[serde(default)]
    title: Option<String>,
}

/// Validated recording metadata.
#[derive(Debug, Clone)]
pub struct Header {
    pub cols: usize,
    pub rows: usize,
    pub idle_time_limit: Option<f64>,
    pub title: Option<String>,
    pub theme: Option<Theme>,
}

/// Event type. Only `Output` and `Resize` affect the screen; every event
/// advances the clock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventCode {
    Output,
    Input,
    Marker,
    Resize,
    Exit,
    Other(String),
}

impl EventCode {
    fn from_str(s: &str) -> Self {
        match s {
            "o" => Self::Output,
            "i" => Self::Input,
            "m" => Self::Marker,
            "r" => Self::Resize,
            "x" => Self::Exit,
            other => Self::Other(other.to_owned()),
        }
    }
}

/// Single event with its absolute source time in seconds.
#[derive(Debug, Clone)]
pub struct Event {
    pub time: f64,
    pub code: EventCode,
    pub data: String,
}

/// Parse full cast text into header plus events with accumulated absolute time.
///
/// v3 event times are deltas from the previous event; they are summed here so
/// downstream code only deals with absolute source times.
pub fn parse(text: &str) -> anyhow::Result<(Header, Vec<Event>)> {
    let mut lines = text.lines().enumerate().peekable();

    // First non-blank, non-comment line is the header object.
    let mut header_line: Option<(usize, &str)> = None;
    for (idx, line) in lines.by_ref() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        header_line = Some((idx, line));
        break;
    }
    let (header_no, header_text) =
        header_line.ok_or_else(|| anyhow::anyhow!("empty recording: missing v3 header"))?;

    let header = parse_header(header_no, header_text)?;

    let mut events = Vec::new();
    let mut time = 0.0f64;
    for (idx, line) in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let (delta, code, data): (f64, String, serde_json::Value) = serde_json::from_str(trimmed)
            .map_err(|e| {
            anyhow::anyhow!(
                "line {}: invalid event ({}): {}",
                idx + 1,
                e,
                snippet(trimmed)
            )
        })?;
        if !delta.is_finite() || delta < 0.0 {
            anyhow::bail!("line {}: invalid event interval {delta}", idx + 1);
        }
        time += delta;
        events.push(Event {
            time,
            code: EventCode::from_str(&code),
            data: json_string(&data),
        });
    }

    Ok((header, events))
}

fn parse_header(line_no: usize, text: &str) -> anyhow::Result<Header> {
    let raw: HeaderRaw = serde_json::from_str(text).map_err(|e| {
        // v1/v2 recordings have a different header shape; point at the fix.
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(text)
            && v.get("version").and_then(|v| v.as_u64()) != Some(3)
        {
            return anyhow::anyhow!(
                "line {}: not an asciicast v3 recording (convert with `asciinema convert old.cast v3.cast`)",
                line_no + 1
            );
        }
        anyhow::anyhow!("line {}: invalid v3 header: {e}", line_no + 1)
    })?;

    if raw.version != 3 {
        anyhow::bail!(
            "line {}: unsupported asciicast version {} (only v3 is supported; convert with `asciinema convert`)",
            line_no + 1,
            raw.version
        );
    }
    if raw.term.cols == 0 || raw.term.rows == 0 {
        anyhow::bail!(
            "line {}: invalid terminal size {}x{}",
            line_no + 1,
            raw.term.cols,
            raw.term.rows
        );
    }
    if let Some(limit) = raw.idle_time_limit
        && !(limit.is_finite() && limit >= 0.0)
    {
        anyhow::bail!("line {}: invalid idle_time_limit {limit}", line_no + 1);
    }

    let theme = raw
        .term
        .theme
        .map(|t| Theme::from_header(&t.fg, &t.bg, &t.palette))
        .transpose()
        .map_err(|e| anyhow::anyhow!("line {}: invalid embedded theme: {e}", line_no + 1))?;

    Ok(Header {
        cols: raw.term.cols,
        rows: raw.term.rows,
        idle_time_limit: raw.idle_time_limit,
        title: raw.title,
        theme,
    })
}

fn json_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn snippet(line: &str) -> String {
    const MAX: usize = 80;
    if line.len() <= MAX {
        line.to_owned()
    } else {
        format!("{}…", &line[..MAX])
    }
}
