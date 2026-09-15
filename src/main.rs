use std::io::Write as _;

use clap::Parser;
use nemasvg::{DEFAULT_FONT_FAMILY, Options, builtin_names, generate, read_cast_text};

/// Convert asciicast v3 recordings into animated SVG files.
#[derive(Debug, Parser)]
#[command(name = "nemasvg", version, about)]
struct Cli {
    /// Input .cast path, or - for stdin.
    #[arg(default_value = "-")]
    input: String,

    /// Output .svg path, or - for stdout.
    #[arg(default_value = "-")]
    output: String,

    /// Playback speed multiplier.
    #[arg(long, default_value_t = 1.0)]
    speed: f64,

    /// Maximum visual frames per second.
    #[arg(long, default_value_t = 30)]
    fps: u32,

    /// Cap idle gaps (seconds); defaults to the recording header's limit.
    #[arg(long)]
    idle_time_limit: Option<f64>,

    /// Pin terminal width in columns.
    #[arg(long, visible_alias = "width")]
    cols: Option<usize>,

    /// Pin terminal height in rows.
    #[arg(long, visible_alias = "height")]
    rows: Option<usize>,

    /// CSS font stack for terminal text.
    #[arg(long, default_value = DEFAULT_FONT_FAMILY)]
    font_family: String,

    /// Font size in pixels.
    #[arg(long, default_value_t = 16)]
    font_size: u32,

    /// Line height multiplier.
    #[arg(long, default_value_t = 1.4)]
    line_height: f32,

    /// Padding in pixels around the terminal area.
    #[arg(long, default_value_t = 0)]
    padding: u32,

    /// Color theme (see --list-themes). Defaults to the recording's embedded
    /// theme, then dracula.
    #[arg(long)]
    theme: Option<String>,

    /// List builtin themes and exit.
    #[arg(long, default_value_t = false)]
    list_themes: bool,

    /// Render a single static frame at this time (seconds).
    #[arg(long, conflicts_with_all = ["from", "to"])]
    at: Option<f64>,

    /// Start of animated excerpt (seconds).
    #[arg(long)]
    from: Option<f64>,

    /// End of animated excerpt (seconds).
    #[arg(long)]
    to: Option<f64>,

    /// Hide the cursor.
    #[arg(long, default_value_t = false)]
    no_cursor: bool,

    /// Play once instead of looping.
    #[arg(long, default_value_t = false)]
    no_loop: bool,

    /// Draw window chrome around the terminal.
    #[arg(long, default_value_t = false)]
    window: bool,

    /// Window title (defaults to the recording title).
    #[arg(long)]
    title: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.list_themes {
        for name in builtin_names() {
            println!("{name}");
        }
        return Ok(());
    }

    let options = Options {
        speed: cli.speed,
        fps: cli.fps,
        idle_time_limit: cli.idle_time_limit,
        cols: cli.cols,
        rows: cli.rows,
        font_family: cli.font_family,
        font_size: cli.font_size,
        line_height: cli.line_height,
        padding: cli.padding,
        theme: cli.theme,
        at: cli.at,
        from: cli.from,
        to: cli.to,
        no_cursor: cli.no_cursor,
        no_loop: cli.no_loop,
        window: cli.window,
        title: cli.title,
    };

    let cast = read_cast_text(&cli.input)?;
    let svg = generate(&cast, &options)?;

    if cli.output == "-" {
        let mut stdout = std::io::stdout().lock();
        stdout.write_all(svg.as_bytes())?;
    } else {
        std::fs::write(&cli.output, svg)
            .map_err(|e| anyhow::anyhow!("cannot write {}: {e}", cli.output))?;
    }
    Ok(())
}
