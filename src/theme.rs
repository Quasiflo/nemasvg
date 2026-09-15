use std::collections::HashMap;
use std::sync::LazyLock;

/// Named fallback palette used when neither `--theme` nor the recording header
/// provides one.
pub const DEFAULT_THEME: &str = "dracula";

/// Monospace stack used for SVG text. Mirrors agg's ordering philosophy:
/// preferred coding fonts first, symbol coverage before color emoji.
pub const DEFAULT_FONT_FAMILY: &str = "JetBrains Mono,Fira Code,SF Mono,Menlo,Consolas,DejaVu Sans Mono,Liberation Mono,Symbols Nerd Font,Apple Color Emoji,Segoe UI Emoji,Noto Color Emoji";

/// Terminal theme: default colors plus the 16 ANSI palette entries.
#[derive(Debug, Clone)]
pub struct Theme {
    pub fg: String,
    pub bg: String,
    pub palette: [String; 16],
}

impl Theme {
    /// Look up a builtin by name (case-insensitive, `-`/`_`/` ` equivalent).
    pub fn builtin(name: &str) -> Option<Self> {
        BUILTINS.get(&normalize(name)).cloned()
    }

    /// Parse an embedded v3 header theme. An 8-color palette is expanded to 16
    /// by repeating it, matching asciinema player behavior.
    pub fn from_header(fg: &str, bg: &str, palette: &str) -> anyhow::Result<Self> {
        let fg = validate_hex(fg)?;
        let bg = validate_hex(bg)?;
        let parts: Vec<&str> = palette.split(':').collect();
        let expanded: Vec<String> = match parts.len() {
            8 => {
                let base: Vec<String> = parts
                    .iter()
                    .map(|c| validate_hex(c))
                    .collect::<Result<_, _>>()?;
                base.iter().chain(base.iter()).cloned().collect()
            }
            16 => parts
                .iter()
                .map(|c| validate_hex(c))
                .collect::<Result<_, _>>()?,
            n => anyhow::bail!("expected 8 or 16 palette colors, got {n}"),
        };
        Ok(Self {
            fg,
            bg,
            palette: expanded
                .try_into()
                .map_err(|_| anyhow::anyhow!("palette expansion failed"))?,
        })
    }

    /// Resolve an `avt` indexed color (0-255) to a CSS hex string.
    pub fn indexed(&self, index: u8) -> String {
        let i = index as usize;
        if i < 16 {
            return self.palette[i].clone();
        }
        if i < 232 {
            // 6x6x6 color cube.
            const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
            let n = i - 16;
            return rgb_hex(LEVELS[n / 36], LEVELS[(n % 36) / 6], LEVELS[n % 6]);
        }
        // Grayscale ramp.
        let level = 8 + 10 * (i - 232) as u16;
        let level = level.min(255) as u8;
        rgb_hex(level, level, level)
    }
}

fn validate_hex(color: &str) -> anyhow::Result<String> {
    let c = color.trim();
    let hex = c.strip_prefix('#').unwrap_or(c);
    if hex.len() == 6 && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(format!("#{}", hex.to_ascii_lowercase()))
    } else {
        Err(anyhow::anyhow!("invalid #rrggbb color: {color}"))
    }
}

pub fn rgb_hex(r: u8, g: u8, b: u8) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn normalize(name: &str) -> String {
    name.to_ascii_lowercase()
        .chars()
        .filter(|c| !matches!(c, '-' | '_' | ' '))
        .collect()
}

fn theme(fg: &str, bg: &str, palette: [&str; 16]) -> Theme {
    Theme {
        fg: fg.to_owned(),
        bg: bg.to_owned(),
        palette: palette.map(str::to_owned),
    }
}

static BUILTINS: LazyLock<HashMap<String, Theme>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert(
        "dracula".to_owned(),
        theme(
            "#f8f8f2",
            "#282a36",
            [
                "#21222c", "#ff5555", "#50fa7b", "#f1fa8c", "#bd93f9", "#ff79c6", "#8be9fd",
                "#f8f8f2", "#6272a4", "#ff6e6e", "#69ff94", "#ffffa5", "#d6acff", "#ff92df",
                "#a4ffff", "#ffffff",
            ],
        ),
    );
    m.insert(
        "asciinema".to_owned(),
        theme(
            "#cccccc",
            "#121212",
            [
                "#000000", "#cc0000", "#4e9a06", "#c4a000", "#3465a4", "#75507b", "#06989a",
                "#d3d7cf", "#555753", "#ef2929", "#8ae234", "#fce94f", "#729fcf", "#ad7fa8",
                "#34e2e2", "#eeeeec",
            ],
        ),
    );
    m.insert(
        "monokai".to_owned(),
        theme(
            "#f8f8f2",
            "#272822",
            [
                "#272822", "#f92672", "#a6e22e", "#f4bf75", "#66d9ef", "#ae81ff", "#a1efe4",
                "#f8f8f2", "#75715e", "#f92672", "#a6e22e", "#f4bf75", "#66d9ef", "#ae81ff",
                "#a1efe4", "#f9f8f5",
            ],
        ),
    );
    m.insert(
        "solarizeddark".to_owned(),
        theme(
            "#839496",
            "#002b36",
            [
                "#073642", "#dc322f", "#859900", "#b58900", "#268bd2", "#d33682", "#2aa198",
                "#eee8d5", "#002b36", "#cb4b16", "#586e75", "#657b83", "#839496", "#6c71c4",
                "#93a1a1", "#fdf6e3",
            ],
        ),
    );
    m.insert(
        "solarizedlight".to_owned(),
        theme(
            "#657b83",
            "#fdf6e3",
            [
                "#073642", "#dc322f", "#859900", "#b58900", "#268bd2", "#d33682", "#2aa198",
                "#eee8d5", "#002b36", "#cb4b16", "#586e75", "#657b83", "#839496", "#6c71c4",
                "#93a1a1", "#fdf6e3",
            ],
        ),
    );
    m.insert(
        "githubdark".to_owned(),
        theme(
            "#e6edf3",
            "#0d1117",
            [
                "#484f58", "#ff7b72", "#3fb950", "#d29922", "#58a6ff", "#bc8cff", "#39c5cf",
                "#b1bac4", "#6e7681", "#ffa198", "#56d364", "#e3b341", "#79c0ff", "#d2a8ff",
                "#56d4dd", "#ffffff",
            ],
        ),
    );
    m
});

/// Names of all builtin themes, for `--help` and error messages.
pub fn builtin_names() -> Vec<&'static str> {
    vec![
        "asciinema",
        "dracula",
        "github-dark",
        "monokai",
        "solarized-dark",
        "solarized-light",
    ]
}
