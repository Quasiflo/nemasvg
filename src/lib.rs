//! nemasvg: asciicast v3 → animated SVG.
//!
//! Library entry point is [`generate`]: feed it full cast text plus [`Options`]
//! and get a self-contained SVG document back. The CLI in `main.rs` is a thin
//! wrapper around it.

mod asciicast;
mod input;
mod renderer;
mod terminal;
mod theme;
mod timeline;

pub use input::read_cast_text;
pub use theme::{DEFAULT_FONT_FAMILY, DEFAULT_THEME, Theme, builtin_names};

/// Conversion options. Mirrors the CLI flags one-to-one.
#[derive(Debug, Clone)]
pub struct Options {
    /// Playback speed multiplier.
    pub speed: f64,
    /// Maximum visual frames per second.
    pub fps: u32,
    /// Cap idle gaps; defaults to the recording header's limit.
    pub idle_time_limit: Option<f64>,
    /// Pin terminal width, disabling auto-grow on that axis.
    pub cols: Option<usize>,
    /// Pin terminal height, disabling auto-grow on that axis.
    pub rows: Option<usize>,
    /// CSS font stack for terminal text.
    pub font_family: String,
    /// Font size in pixels.
    pub font_size: u32,
    /// Line height multiplier.
    pub line_height: f32,
    /// Padding in pixels around the terminal area.
    pub padding: u32,
    /// Theme name; falls back to the recording's embedded theme, then dracula.
    pub theme: Option<String>,
    /// Render a single static frame at this time (seconds).
    pub at: Option<f64>,
    /// Start of animated excerpt (seconds).
    pub from: Option<f64>,
    /// End of animated excerpt (seconds).
    pub to: Option<f64>,
    /// Hide the cursor.
    pub no_cursor: bool,
    /// Play once instead of looping.
    pub no_loop: bool,
    /// Draw macOS-style window chrome.
    pub window: bool,
    /// Window title (defaults to the recording title, if any).
    pub title: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            speed: 1.0,
            fps: 30,
            idle_time_limit: None,
            cols: None,
            rows: None,
            font_family: DEFAULT_FONT_FAMILY.to_owned(),
            font_size: 16,
            line_height: 1.4,
            padding: 0,
            theme: None,
            at: None,
            from: None,
            to: None,
            no_cursor: false,
            no_loop: false,
            window: false,
            title: None,
        }
    }
}

/// Convert full asciicast v3 text into a self-contained animated SVG document.
pub fn generate(cast: &str, options: &Options) -> anyhow::Result<String> {
    if options.font_size == 0 {
        anyhow::bail!("invalid --font-size 0");
    }
    if !(options.line_height > 0.0 && options.line_height.is_finite()) {
        anyhow::bail!("invalid --line-height {}", options.line_height);
    }

    let (header, events) = asciicast::parse(cast)?;

    let theme = match options.theme.as_deref() {
        Some(name) => Theme::builtin(name)
            .ok_or_else(|| anyhow::anyhow!("unknown theme {name:?} (see --list-themes)"))?,
        None => header
            .theme
            .clone()
            .or_else(|| Theme::builtin(DEFAULT_THEME))
            .expect("default theme must exist"),
    };

    let timeline = timeline::build(&header, &events, options)?;
    Ok(renderer::render(&timeline, &theme, &header, options))
}
