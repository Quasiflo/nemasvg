use crate::Options;
use crate::asciicast::{Event, EventCode, Header};
use crate::terminal::{Snapshot, Term};

/// One distinct visual state with its output-clock time in seconds.
#[derive(Debug, Clone)]
pub struct Frame {
    pub time: f64,
    pub snap: Snapshot,
}

/// Playback timeline: distinct frames on the output clock plus canvas geometry.
///
/// The SVG viewport cannot resize mid-animation, so the canvas is the maximum
/// size ever observed (or the pinned `--cols`/`--rows` when given).
#[derive(Debug, Clone)]
pub struct Timeline {
    pub frames: Vec<Frame>,
    pub cols: usize,
    pub rows: usize,
    pub duration: f64,
}

/// Replay events through a virtual terminal and collect distinct frames.
///
/// Transform order is fixed: idle-cap the source interval, divide by speed,
/// accumulate onto the output clock, apply the event, then snapshot (only for
/// output/resize). Identical consecutive states are dropped; bursts are merged
/// to the FPS window keeping the latest state.
pub fn build(header: &Header, events: &[Event], options: &Options) -> anyhow::Result<Timeline> {
    let mut term = Term::new(header.cols, header.rows);
    let mut canvas_cols = options.cols.unwrap_or(header.cols);
    let mut canvas_rows = options.rows.unwrap_or(header.rows);
    if let (Some(c), Some(r)) = (options.cols, options.rows) {
        term = Term::new(c, r);
        canvas_cols = c;
        canvas_rows = r;
    } else {
        if let Some(c) = options.cols {
            let (_, rows) = term.size();
            term = Term::new(c, rows);
            canvas_cols = c;
        }
        if let Some(r) = options.rows {
            let (cols, _) = term.size();
            term = Term::new(cols, r);
            canvas_rows = r;
        }
    }

    let idle_limit = options
        .idle_time_limit
        .or(header.idle_time_limit)
        .unwrap_or(f64::INFINITY);
    if options.speed <= 0.0 || !options.speed.is_finite() {
        anyhow::bail!("invalid --speed {}", options.speed);
    }

    let mut frames: Vec<Frame> = Vec::new();
    let mut clock = 0.0f64;
    let mut prev_source = 0.0f64;

    for event in events {
        let interval = (event.time - prev_source).max(0.0);
        prev_source = event.time;
        clock += interval.min(idle_limit) / options.speed;

        match &event.code {
            EventCode::Output => term.feed(&event.data),
            EventCode::Resize => {
                let (cols, rows) = parse_resize(&event.data)?;
                term.resize(cols, rows);
                if options.cols.is_none() {
                    canvas_cols = canvas_cols.max(cols);
                }
                if options.rows.is_none() {
                    canvas_rows = canvas_rows.max(rows);
                }
            }
            EventCode::Input | EventCode::Marker | EventCode::Exit | EventCode::Other(_) => {}
        }

        if matches!(event.code, EventCode::Output | EventCode::Resize) {
            let snap = term.snapshot();
            // Drop states with no visual change.
            if frames.last().is_none_or(|f: &Frame| f.snap != snap) {
                frames.push(Frame { time: clock, snap });
            }
        }
    }

    let total = clock;
    let end = options.to.unwrap_or(total);

    // Range / single-frame selection.
    let selected: Vec<Frame> = if let Some(at) = options.at {
        match frames.iter().rfind(|f| f.time <= at + 1e-9) {
            Some(f) => vec![f.clone()],
            None => frames.first().cloned().into_iter().collect(),
        }
    } else {
        let from = options.from.unwrap_or(0.0);
        let mut in_range: Vec<Frame> = frames
            .iter()
            .filter(|f| f.time >= from - 1e-9 && f.time <= end + 1e-9)
            .cloned()
            .collect();
        // The state at `from` is the last frame before it; carry it forward
        // so excerpts open on the correct screen instead of a blank one.
        if let Some(before) = frames.iter().rfind(|f| f.time < from - 1e-9)
            && in_range.first().is_none_or(|f| f.time > from + 1e-9)
        {
            let mut carried = before.clone();
            carried.time = from;
            in_range.insert(0, carried);
        }
        for f in &mut in_range {
            f.time -= from;
        }
        in_range
    };

    let base = if options.at.is_some() {
        0.0
    } else {
        options.from.unwrap_or(0.0)
    };
    let duration = if options.at.is_some() {
        0.0
    } else {
        (end - base).max(0.0)
    };

    // FPS cap: fixed 1/fps windows anchored at t=0, keeping the latest state
    // in each window. Never invent filler frames for idle stretches, and
    // never let a sustained burst collapse to a single frame.
    let mut capped: Vec<Frame> = Vec::with_capacity(selected.len());
    if selected.len() > 1 && options.fps > 0 {
        let window = 1.0 / f64::from(options.fps);
        let mut kept_window: Option<i64> = None;
        for frame in selected {
            let window_idx = (frame.time / window).floor() as i64;
            match capped.last_mut() {
                // Latest state wins within the window.
                Some(last) if kept_window == Some(window_idx) => *last = frame,
                _ => {
                    kept_window = Some(window_idx);
                    capped.push(frame);
                }
            }
        }
    } else {
        capped = selected;
    }

    Ok(Timeline {
        frames: capped,
        cols: canvas_cols,
        rows: canvas_rows,
        duration,
    })
}

fn parse_resize(data: &str) -> anyhow::Result<(usize, usize)> {
    let (cols, rows) = data
        .split_once('x')
        .ok_or_else(|| anyhow::anyhow!("invalid resize event {data:?}"))?;
    let cols: usize = cols
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid resize event {data:?}"))?;
    let rows: usize = rows
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid resize event {data:?}"))?;
    if cols == 0 || rows == 0 {
        anyhow::bail!("invalid resize event {data:?}");
    }
    Ok((cols, rows))
}
