//! Bundled fonts and `@font-face` embedding.
//!
//! The default output embeds subsets of the bundled monospace faces, so the
//! SVG renders identically anywhere without viewer system fonts. Subsets
//! contain only glyphs the recording uses, encoded as WOFF2 data URIs.
//!
//! Design notes:
//! - JetBrains Mono (OFL) Regular + Bold are bundled; oblique text falls back
//!   to browser-synthesized slant, bold+italic to slanted bold.
//! - Monochrome Noto Emoji (OFL) is bundled for the opt-in `--embed-emoji`
//!   path. Color emoji is deliberately out of scope: bitmap/COLR subsets are
//!   heavy and viewer support is uneven; without the flag, emoji falls back
//!   to OS fonts as before.
//! - resvg and librsvg ignore `@font-face` data URIs (upstream limitation),
//!   so embedded rendering cannot be raster-verified here; tests decode and
//!   re-parse the subsets instead. Browsers honor them.

use std::collections::{BTreeSet, HashSet};

use base64::Engine as _;

use crate::timeline::Timeline;

/// Family name used for the embedded monospace faces.
pub const MONO_FAMILY: &str = "Nemasvg Mono";
/// Family name used for the embedded emoji face (`--embed-emoji` only).
pub const EMOJI_FAMILY: &str = "Nemasvg Emoji";

const REGULAR: &[u8] = include_bytes!("../fonts/JetBrainsMono-Regular.ttf");
const BOLD: &[u8] = include_bytes!("../fonts/JetBrainsMono-Bold.ttf");
const EMOJI: &[u8] = include_bytes!("../fonts/NotoEmoji[wght].ttf");

/// Advance ratio assumed when no font is measured (system-font mode).
pub const FALLBACK_RATIO: f64 = 0.6;

/// Resolved font plan for one conversion.
pub struct FontPlan {
    /// `@font-face` CSS to prepend to the stylesheet (empty when disabled).
    pub face_css: String,
    /// Font stack prefix, e.g. `'Nemasvg Mono',` (empty when disabled).
    pub family_prefix: String,
    /// Space-advance / units-per-em of the primary face.
    pub advance_ratio: f64,
}

/// Collect every rendered character plus whether bold text occurs.
fn collect_used(timeline: &Timeline) -> (BTreeSet<char>, bool) {
    let mut chars = BTreeSet::new();
    let mut bold = false;
    for frame in &timeline.frames {
        for line in &frame.snap.lines {
            for cell in line.cells() {
                if cell.width() == 0 {
                    continue;
                }
                chars.insert(cell.char());
                bold |= cell.pen().is_bold();
            }
        }
    }
    // Space is always needed for advances even when trimmed from runs.
    chars.insert(' ');
    (chars, bold)
}

/// Measure the space-advance ratio of a font (advance / units-per-em).
fn advance_ratio(font_data: &[u8]) -> anyhow::Result<f64> {
    let face = ttf_parser::Face::parse(font_data, 0)
        .map_err(|e| anyhow::anyhow!("cannot parse bundled font: {e:?}"))?;
    let upem = f64::from(face.units_per_em());
    let gid = face
        .glyph_index(' ')
        .ok_or_else(|| anyhow::anyhow!("bundled font has no space glyph"))?;
    Ok(f64::from(
        face.glyph_hor_advance(gid)
            .ok_or_else(|| anyhow::anyhow!("bundled font has no advance for space"))?,
    ) / upem)
}

/// Subset a font to `chars` and encode the subset as WOFF2.
fn subset_woff2(font_data: &[u8], chars: &HashSet<char>, what: &str) -> anyhow::Result<Vec<u8>> {
    let subset = fontcull::subset_font_data(font_data, chars, &[])
        .map_err(|e| anyhow::anyhow!("cannot subset {what} font: {e:?}"))?;
    ttf2woff2::encode(&subset, ttf2woff2::BrotliQuality::default())
        .map_err(|e| anyhow::anyhow!("cannot encode {what} subset as WOFF2: {e:?}"))
}

fn face_block(family: &str, woff2: &[u8], descriptors: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    write!(
        out,
        "@font-face{{font-family:'{family}';src:url(data:font/woff2;base64,{}) format('woff2');{descriptors}}}",
        base64::engine::general_purpose::STANDARD.encode(woff2),
    )
    .expect("writing to String cannot fail");
    out
}

/// Plan fonts for a conversion: subset, encode, and describe faces.
///
/// `embed_emoji` additionally embeds monochrome glyphs for characters the
/// monospace faces lack (when the emoji face provides them); everything else
/// keeps falling back to viewer fonts.
pub fn plan(timeline: &Timeline, embed_emoji: bool) -> anyhow::Result<FontPlan> {
    let ratio = advance_ratio(REGULAR)?;
    let (used, bold) = collect_used(timeline);
    // No text at all: nothing to embed (and no dangling family reference),
    // but the measured ratio still applies to the grid.
    if used.len() <= 1 {
        return Ok(FontPlan {
            face_css: String::new(),
            family_prefix: String::new(),
            advance_ratio: ratio,
        });
    }

    let used_set: HashSet<char> = used.iter().copied().collect();
    let regular = subset_woff2(REGULAR, &used_set, "monospace")?;
    let mut css = face_block(MONO_FAMILY, &regular, "font-weight:400;");
    if bold {
        let bold_subset = subset_woff2(BOLD, &used_set, "monospace bold")?;
        css.push_str(&face_block(MONO_FAMILY, &bold_subset, "font-weight:700;"));
    }

    let regular_face = ttf_parser::Face::parse(REGULAR, 0)
        .map_err(|e| anyhow::anyhow!("cannot parse bundled font: {e:?}"))?;
    let mut prefix = format!("'{MONO_FAMILY}',");
    if embed_emoji {
        let emoji_face = ttf_parser::Face::parse(EMOJI, 0)
            .map_err(|e| anyhow::anyhow!("cannot parse bundled emoji font: {e:?}"))?;
        let missing: HashSet<char> = used
            .iter()
            .copied()
            .filter(|c| {
                regular_face.glyph_index(*c).is_none() && emoji_face.glyph_index(*c).is_some()
            })
            .collect();
        if !missing.is_empty() {
            let emoji_subset = subset_woff2(EMOJI, &missing, "emoji")?;
            css.push_str(&face_block(EMOJI_FAMILY, &emoji_subset, ""));
            prefix.push_str(&format!("'{EMOJI_FAMILY}',"));
        }
    }

    Ok(FontPlan {
        face_css: css,
        family_prefix: prefix,
        advance_ratio: ratio,
    })
}

/// System-font plan: no embedding, assumed advance ratio.
pub fn system_plan() -> FontPlan {
    FontPlan {
        face_css: String::new(),
        family_prefix: String::new(),
        advance_ratio: FALLBACK_RATIO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn bundled_mono_measures_at_narrow_ratio() {
        let ratio = advance_ratio(REGULAR).unwrap();
        assert!(
            (0.59..0.61).contains(&ratio),
            "unexpected advance ratio {ratio}"
        );
    }

    #[test]
    fn subset_covers_exactly_the_used_glyphs() {
        let chars: HashSet<char> = ['A', ' ', 'é'].into_iter().collect();
        let subset = fontcull::subset_font_data(REGULAR, &chars, &[]).unwrap();
        assert!(subset.len() < REGULAR.len() / 10);
        let face = ttf_parser::Face::parse(&subset, 0).unwrap();
        for c in ['A', ' ', 'é'] {
            assert!(face.glyph_index(c).is_some(), "missing {c}");
        }
        assert!(face.glyph_index('Z').is_none(), "unused glyph retained");
    }

    #[test]
    fn woff2_roundtrip_keeps_magic() {
        let chars: HashSet<char> = ['A'].into_iter().collect();
        let woff2 = subset_woff2(REGULAR, &chars, "test").unwrap();
        assert_eq!(&woff2[..4], b"wOF2");
    }
}
