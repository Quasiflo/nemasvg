//! Timeline behavior: event clock, dedup, FPS merging, seeks.
//!
//! Each test pins an operational contract: given these event times,
//! this many frames play for this long.

mod common;

use common::{Cast, animation_secs, frame_count, keyframes, opts};
use nemasvg::generate;

fn svg(cast: &str, options: &nemasvg::Options) -> String {
    generate(cast, options).expect("fixture must convert")
}

#[test]
fn single_state_renders_static_svg() {
    let cast = Cast::new(40, 8).event(0.5, "o", "hello").build();
    let out = svg(&cast, &opts());
    assert_eq!(frame_count(&out), 1);
    assert!(!out.contains("@keyframes"), "single state must not animate");
    assert!(!out.contains("class=\"r\""), "single state needs no reel");
}

#[test]
fn repeated_identical_output_dedups_to_one_frame() {
    let cast = Cast::new(40, 8)
        .event(0.5, "o", "same")
        .event(0.5, "o", "")
        .event(0.5, "m", "marker")
        .event(0.5, "x", "0")
        .build();
    let out = svg(&cast, &opts());
    assert_eq!(
        frame_count(&out),
        1,
        "input/marker/exit add time, not frames"
    );
}

#[test]
fn animation_duration_covers_events_plus_tail_hold() {
    // 1.0s of events + 1.0s final hold.
    let cast = Cast::new(40, 8)
        .event(0.5, "o", "one")
        .event(0.5, "o", "two")
        .build();
    let out = svg(&cast, &opts());
    assert_eq!(frame_count(&out), 2);
    assert_eq!(animation_secs(&out), Some(2.0));
}

#[test]
fn speed_scales_playback_time() {
    let cast = Cast::new(40, 8)
        .event(1.0, "o", "one")
        .event(1.0, "o", "two")
        .build();
    let mut o = opts();
    o.speed = 2.0;
    let out = svg(&cast, &o);
    assert_eq!(
        animation_secs(&out),
        Some(2.0),
        "2s of events at 2x + 1s hold"
    );
}

#[test]
fn idle_gaps_are_capped_by_cli_limit() {
    let cast = Cast::new(40, 8)
        .event(0.1, "o", "start")
        .event(10.0, "o", "after idle")
        .build();
    let mut o = opts();
    o.idle_time_limit = Some(2.0);
    let out = svg(&cast, &o);
    assert_eq!(
        animation_secs(&out),
        Some(3.1),
        "0.1 + capped 2.0 + 1s hold"
    );
}

#[test]
fn header_idle_limit_applies_without_cli_override() {
    let cast =
        Cast::header_raw(r#"{"version":3,"term":{"cols":40,"rows":8},"idle_time_limit":1.5}"#)
            .event(0.1, "o", "start")
            .event(10.0, "o", "after idle")
            .build();
    let out = svg(&cast, &opts());
    assert_eq!(animation_secs(&out), Some(2.6));
}

#[test]
fn bursts_merge_to_fps_windows_keeping_latest() {
    // 5 events across 0.15s at 10fps: windows [0,.1) and [.1,.2).
    // (Deltas avoid exact window boundaries, where float fuzz decides.)
    let mut cast = Cast::new(40, 8);
    for i in 0..5 {
        cast = cast.event(0.03, "o", &format!("n{i}\n"));
    }
    let mut o = opts();
    o.fps = 10;
    let out = svg(&cast.build(), &o);
    assert_eq!(frame_count(&out), 2);
    assert!(out.contains("n4"), "latest state per window wins");
}

#[test]
fn fps_zero_disables_capping() {
    let mut cast = Cast::new(40, 8);
    for i in 0..5 {
        cast = cast.event(0.01, "o", &format!("n{i}\n"));
    }
    let mut o = opts();
    o.fps = 0;
    assert_eq!(frame_count(&svg(&cast.build(), &o)), 5);
}

#[test]
fn sustained_burst_does_not_collapse_to_one_frame() {
    // Regression: sliding windows once merged a whole progress bar away.
    let mut cast = Cast::new(40, 8);
    for i in 0..20 {
        cast = cast.event(0.02, "o", &format!("\rprogress {i}\n"));
    }
    let out = svg(&cast.build(), &opts());
    assert!(frame_count(&out) > 5, "got {}", frame_count(&out));
}

#[test]
fn zero_delta_events_keep_only_latest_state() {
    // Same output clock: earlier state was never viewable.
    let cast = Cast::new(40, 8)
        .event(0.5, "o", "first")
        .event(0.0, "o", "\rsecond")
        .build();
    let out = svg(&cast, &opts());
    assert_eq!(frame_count(&out), 1);
    assert!(out.contains("second"));
}

#[test]
fn at_selects_state_and_renders_static() {
    let cast = Cast::new(40, 8)
        .event(1.0, "o", "one\n")
        .event(1.0, "o", "two\n")
        .build();
    let mut o = opts();
    o.at = Some(1.5);
    let out = svg(&cast, &o);
    assert_eq!(frame_count(&out), 1);
    assert!(!out.contains("@keyframes"));
    assert!(out.contains("one") && !out.contains("two"));
}

#[test]
fn at_before_first_frame_falls_back_to_first_state() {
    let cast = Cast::new(40, 8).event(1.0, "o", "one").build();
    let mut o = opts();
    o.at = Some(0.1);
    let out = svg(&cast, &o);
    assert!(out.contains("one"));
}

#[test]
fn at_past_end_clamps_to_last_state() {
    let cast = Cast::new(40, 8)
        .event(1.0, "o", "one\n")
        .event(1.0, "o", "two\n")
        .build();
    let mut o = opts();
    o.at = Some(99.0);
    let out = svg(&cast, &o);
    assert!(out.contains("two"));
}

#[test]
fn excerpt_rebases_to_zero_and_carries_prior_state() {
    let cast = Cast::new(40, 8)
        .event(1.0, "o", "AAA\n")
        .event(1.0, "o", "BBB\n")
        .event(1.0, "o", "CCC\n")
        .build();
    let mut o = opts();
    o.from = Some(1.5);
    o.to = Some(2.5);
    let out = svg(&cast, &o);
    // Opens on the BBB state carried from before `from`, rebased to t=0.
    assert!(out.contains("BBB"), "excerpt must open on carried state");
    let kf = keyframes(&out);
    assert_eq!(kf.first().map(|(p, _)| *p), Some(0.0));
    assert_eq!(animation_secs(&out), Some(2.0), "1s range + 1s hold");
}

#[test]
fn keyframes_hold_offsets_until_next_frame() {
    let cast = Cast::new(10, 4)
        .event(1.0, "o", "a\n")
        .event(2.0, "o", "b\n")
        .build();
    // 10 cols * 10px = 100px per frame; play length 3 + 1 hold = 4s.
    let out = svg(&cast, &opts());
    let kf = keyframes(&out);
    assert_eq!(kf, vec![(25.0, 0), (75.0, -100), (100.0, -100)]);
}

#[test]
fn resize_grows_canvas() {
    let cast = Cast::new(20, 4)
        .event(0.1, "o", "small")
        .event(0.1, "r", "40x6")
        .event(0.1, "o", "wide")
        .build();
    let out = svg(&cast, &opts());
    assert!(out.contains("width=\"400\""), "canvas follows max cols");
    assert!(out.contains("height=\"132\""), "canvas follows max rows");
}

#[test]
fn cols_override_pins_canvas_width() {
    let cast = Cast::new(80, 24).event(0.1, "o", "hi").build();
    let mut o = opts();
    o.cols = Some(40);
    let out = svg(&cast, &o);
    assert!(out.contains("width=\"400\""));
}

#[test]
fn recordings_without_visual_events_render_blank_static() {
    let cast = Cast::new(80, 24)
        .event(0.5, "i", "x")
        .event(0.5, "m", "mark")
        .event(0.5, "x", "0")
        .build();
    let out = svg(&cast, &opts());
    assert_eq!(frame_count(&out), 0);
    assert!(!out.contains("@keyframes"));
    assert!(out.starts_with("<svg ") && out.ends_with("</svg>"));
}
