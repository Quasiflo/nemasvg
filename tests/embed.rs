//! Font embedding contracts: default-on subsets, opt-out, emoji opt-in.
//!
//! resvg and librsvg ignore `@font-face` data URIs, so these tests decode
//! the embedded subsets and re-parse them instead of rasterizing.

mod common;

use base64::Engine as _;
use common::{Cast, opts};
use nemasvg::generate;

fn svg(cast: &str, options: &nemasvg::Options) -> String {
    generate(cast, options).expect("fixture must convert")
}

fn faces(svg: &str) -> Vec<(String, String, Vec<u8>)> {
    let mut out = Vec::new();
    let mut rest = svg;
    while let Some(i) = rest.find("@font-face{font-family:'") {
        let fam_start = i + "@font-face{font-family:'".len();
        let fam_end = rest[fam_start..].find('\'').unwrap() + fam_start;
        let family = rest[fam_start..fam_end].to_owned();
        let b64_start = rest.find("base64,").unwrap() + "base64,".len();
        let b64_end = rest[b64_start..].find(')').unwrap() + b64_start;
        let raw = base64::engine::general_purpose::STANDARD
            .decode(&rest[b64_start..b64_end])
            .expect("embedded font must be valid base64");
        let desc_start = rest[b64_end..].find(';').unwrap() + b64_end + 1;
        let desc_end = rest[desc_start..].find('}').unwrap() + desc_start;
        out.push((family, rest[desc_start..desc_end].to_owned(), raw));
        rest = &rest[desc_end..];
    }
    out
}

#[test]
fn default_output_embeds_mono_subset() {
    let cast = Cast::new(40, 8).event(0.1, "o", "hello").build();
    let out = svg(&cast, &opts());
    let faces = faces(&out);
    assert_eq!(faces.len(), 1, "regular only when no bold used");
    assert_eq!(faces[0].0, "Nemasvg Mono");
    assert!(faces[0].1.contains("font-weight:400"));
    // Subset: far smaller than the ~270KB source face.
    assert!(faces[0].2.len() < 50_000, "got {}", faces[0].2.len());
    assert!(faces[0].2.len() > 1_000);
    assert!(out.contains("font-family:'Nemasvg Mono',"));
}

#[test]
fn bold_text_embeds_bold_face() {
    let cast = Cast::new(40, 8)
        .event(0.1, "o", "plain\n")
        .event(0.1, "o", "\u{1b}[1mbold\u{1b}[0m")
        .build();
    let families: Vec<String> = faces(&svg(&cast, &opts()))
        .into_iter()
        .map(|(f, _, _)| f)
        .collect();
    assert_eq!(families, vec!["Nemasvg Mono", "Nemasvg Mono"]);
}

#[test]
fn opt_out_leaves_system_stack_alone() {
    let cast = Cast::new(40, 8).event(0.1, "o", "hello").build();
    let mut o = opts();
    o.embed_fonts = false;
    let out = svg(&cast, &o);
    assert!(!out.contains("@font-face"));
    assert!(!out.contains("Nemasvg Mono"));
    assert!(out.contains("font-family:JetBrains Mono"));
}

#[test]
fn blank_recording_embeds_nothing() {
    let cast = Cast::new(40, 8).build();
    let out = svg(&cast, &opts());
    assert!(!out.contains("@font-face"));
}

#[test]
fn emoji_opt_in_embeds_monochrome_subset() {
    let cast = Cast::new(40, 8).event(0.1, "o", "party \u{1f389}!").build();
    let mut o = opts();
    o.embed_emoji = true;
    let out = svg(&cast, &o);
    let faces = faces(&out);
    let families: Vec<&str> = faces.iter().map(|(f, _, _)| f.as_str()).collect();
    assert!(families.contains(&"Nemasvg Emoji"), "{families:?}");
    assert!(out.contains("font-family:'Nemasvg Mono','Nemasvg Emoji',"));
}

#[test]
fn emoji_stays_system_by_default() {
    let cast = Cast::new(40, 8).event(0.1, "o", "party \u{1f389}!").build();
    let out = svg(&cast, &opts());
    assert!(!out.contains("Nemasvg Emoji"));
    assert!(out.contains("party"));
}
