//! Shared fixtures and SVG inspection helpers for behavior-first tests.
//!
//! Tests assert operational properties (frame counts, timing, resolved
//! colors, error messages) rather than exact output snapshots.

#![allow(dead_code)] // not every test target uses every helper

use nemasvg::Options;

/// Minimal v3 cast builder. Data is JSON-escaped properly.
pub struct Cast {
    header: String,
    events: Vec<String>,
}

impl Cast {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            header: format!(r#"{{"version":3,"term":{{"cols":{cols},"rows":{rows}}}}}"#),
            events: Vec::new(),
        }
    }

    pub fn header_raw(header: &str) -> Self {
        Self {
            header: header.to_owned(),
            events: Vec::new(),
        }
    }

    pub fn event(mut self, delta: f64, code: &str, data: &str) -> Self {
        let data = serde_json::to_string(data).expect("fixture data must be encodable");
        self.events.push(format!(
            r#"[{delta},{code},{data}]"#,
            code = serde_json::to_string(code).unwrap()
        ));
        self
    }

    pub fn raw_event(mut self, line: &str) -> Self {
        self.events.push(line.to_owned());
        self
    }

    pub fn build(self) -> String {
        let mut out = self.header;
        out.push('\n');
        for e in self.events {
            out.push_str(&e);
            out.push('\n');
        }
        out
    }
}

/// Count animation frames: one `<g transform="translate(` per frame.
pub fn frame_count(svg: &str) -> usize {
    svg.matches("<g transform=\"translate(").count()
}

/// Extract the `animation:reel Xs` duration, if animated.
pub fn animation_secs(svg: &str) -> Option<f64> {
    let rest = svg.split("animation:reel ").nth(1)?;
    rest.split('s').next()?.parse().ok()
}

/// Extract keyframe (percent, x-offset) pairs in order.
pub fn keyframes(svg: &str) -> Vec<(f64, i64)> {
    let mut out = Vec::new();
    let mut rest = svg;
    while let Some(i) = rest.find("%{transform:translateX(") {
        let pct_start = rest[..i].rfind(['{', ';', '}']).map_or(0, |p| p + 1);
        let pct: f64 = rest[pct_start..i].parse().unwrap_or(-1.0);
        let x_start = i + "%{transform:translateX(".len();
        let x_end = rest[x_start..].find("px)").unwrap() + x_start;
        let x: i64 = rest[x_start..x_end].parse().unwrap_or(0);
        out.push((pct, x));
        rest = &rest[x_end..];
    }
    out
}

pub fn opts() -> Options {
    Options::default()
}
