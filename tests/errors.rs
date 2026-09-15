//! Rejection paths: bad input fails loudly with actionable messages,
//! never silently and never with a panic.

mod common;

use common::{Cast, opts};
use nemasvg::generate;

fn err(cast: &str, options: &nemasvg::Options) -> String {
    generate(cast, options).expect_err("must fail").to_string()
}

#[test]
fn empty_input_reports_missing_header() {
    let e = err("", &opts());
    assert!(e.contains("missing v3 header"), "{e}");
}

#[test]
fn v2_recording_points_at_converter() {
    let cast = Cast::header_raw(r#"{"version":2,"width":80,"height":24}"#)
        .event(0.5, "o", "hi")
        .build();
    let e = err(&cast, &opts());
    assert!(e.contains("asciinema convert"), "{e}");
}

#[test]
fn v1_recording_points_at_converter() {
    let cast = Cast::header_raw(r#"{"version":1,"width":80,"height":24}"#).build();
    let e = err(&cast, &opts());
    assert!(e.contains("asciinema convert"), "{e}");
}

#[test]
fn corrupt_event_reports_line_number() {
    let cast = Cast::new(80, 24).raw_event("not-json").build();
    let e = err(&cast, &opts());
    assert!(e.contains("line 2"), "{e}");
}

#[test]
fn negative_interval_is_rejected() {
    let cast = Cast::new(80, 24).raw_event(r#"[-1.0,"o","hi"]"#).build();
    let e = err(&cast, &opts());
    assert!(e.contains("interval"), "{e}");
}

#[test]
fn zero_terminal_size_is_rejected() {
    let cast = Cast::header_raw(r#"{"version":3,"term":{"cols":0,"rows":24}}"#).build();
    let e = err(&cast, &opts());
    assert!(e.contains("terminal size"), "{e}");
}

#[test]
fn oversized_header_is_rejected() {
    let cast = Cast::header_raw(r#"{"version":3,"term":{"cols":5000,"rows":24}}"#).build();
    let e = err(&cast, &opts());
    assert!(e.contains("4096"), "{e}");
}

#[test]
fn bad_resize_event_is_rejected() {
    let cast = Cast::new(80, 24).event(0.1, "r", "wide").build();
    let e = err(&cast, &opts());
    assert!(e.contains("resize"), "{e}");
}

#[test]
fn zero_resize_is_rejected() {
    let cast = Cast::new(80, 24).event(0.1, "r", "0x24").build();
    let e = err(&cast, &opts());
    assert!(e.contains("resize"), "{e}");
}

#[test]
fn oversized_resize_is_rejected() {
    let cast = Cast::new(80, 24).event(0.1, "r", "5000x24").build();
    let e = err(&cast, &opts());
    assert!(e.contains("4096"), "{e}");
}

#[test]
fn unknown_theme_is_rejected() {
    let cast = Cast::new(80, 24).event(0.1, "o", "hi").build();
    let mut o = opts();
    o.theme = Some("bogus".to_owned());
    let e = err(&cast, &o);
    assert!(e.contains("unknown theme"), "{e}");
}

#[test]
fn bad_embedded_theme_reports_header_line() {
    let cast = Cast::header_raw(
        r##"{"version":3,"term":{"cols":80,"rows":24,"theme":{"fg":"red","bg":"#000000","palette":"#000000"}}}"##,
    )
    .build();
    let e = err(&cast, &opts());
    assert!(e.contains("line 1") && e.contains("theme"), "{e}");
}

#[test]
fn zero_speed_is_rejected() {
    let cast = Cast::new(80, 24).event(0.1, "o", "hi").build();
    let mut o = opts();
    o.speed = 0.0;
    assert!(err(&cast, &o).contains("--speed"));
}

#[test]
fn nan_speed_is_rejected() {
    let cast = Cast::new(80, 24).event(0.1, "o", "hi").build();
    let mut o = opts();
    o.speed = f64::NAN;
    assert!(err(&cast, &o).contains("--speed"));
}

#[test]
fn zero_cols_override_is_rejected() {
    let cast = Cast::new(80, 24).event(0.1, "o", "hi").build();
    let mut o = opts();
    o.cols = Some(0);
    assert!(err(&cast, &o).contains("--cols"));
}

#[test]
fn oversized_cols_override_is_rejected() {
    let cast = Cast::new(80, 24).event(0.1, "o", "hi").build();
    let mut o = opts();
    o.rows = Some(99999);
    assert!(err(&cast, &o).contains("--rows"));
}

#[test]
fn negative_idle_limit_is_rejected() {
    let cast = Cast::new(80, 24).event(0.1, "o", "hi").build();
    let mut o = opts();
    o.idle_time_limit = Some(-1.0);
    assert!(err(&cast, &o).contains("--idle-time-limit"));
}

#[test]
fn negative_seek_is_rejected() {
    let cast = Cast::new(80, 24).event(0.1, "o", "hi").build();
    let mut o = opts();
    o.at = Some(-2.0);
    assert!(err(&cast, &o).contains("--at"));
}

#[test]
fn inverted_range_is_rejected() {
    let cast = Cast::new(80, 24).event(0.1, "o", "hi").build();
    let mut o = opts();
    o.from = Some(2.0);
    o.to = Some(1.0);
    let e = err(&cast, &o);
    assert!(e.contains("--from") && e.contains("--to"), "{e}");
}

#[test]
fn css_breakout_font_stack_is_rejected() {
    let cast = Cast::new(80, 24).event(0.1, "o", "hi").build();
    let mut o = opts();
    o.font_family = "</style><script>alert(1)</script>".to_owned();
    let e = err(&cast, &o);
    assert!(e.contains("--font-family"), "{e}");
}

#[test]
fn brace_font_stack_is_rejected() {
    let cast = Cast::new(80, 24).event(0.1, "o", "hi").build();
    let mut o = opts();
    o.font_family = "x{color:red}".to_owned();
    assert!(err(&cast, &o).contains("--font-family"));
}

#[test]
fn zero_font_size_is_rejected() {
    let cast = Cast::new(80, 24).event(0.1, "o", "hi").build();
    let mut o = opts();
    o.font_size = 0;
    assert!(err(&cast, &o).contains("--font-size"));
}

#[test]
fn bad_line_height_is_rejected() {
    let cast = Cast::new(80, 24).event(0.1, "o", "hi").build();
    let mut o = opts();
    o.line_height = 0.0;
    assert!(err(&cast, &o).contains("--line-height"));
}

#[test]
fn missing_input_file_reports_path() {
    let e = nemasvg::read_cast_text("/nonexistent/nemasvg-test.cast")
        .expect_err("must fail")
        .to_string();
    assert!(e.contains("/nonexistent/nemasvg-test.cast"), "{e}");
}
