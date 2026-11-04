use super::*;
use crate::tex::tests::check;

fn format(v: f64) -> String {
    let mut out = [0u8; 32];
    let n = format_number(v, &mut out);
    String::from_utf8_lossy(&out[..n]).into_owned()
}

fn map() -> AxisMap {
    AxisMap { x0: 0.0, y0: 0.0, w: 10.0, h: 10.0, xmin: 0.0, xmax: 10.0, ymin: 0.0, ymax: 10.0 }
}

#[test]
fn ticks_land_on_nice_numbers() {
    check(&[(0.0, 3.6, 200.0, 0.5), (-1.0, 2.0, 200.0, 0.5), (-0.5, 1.0, 100.0, 0.5), (0.0, 10.0, 200.0, 2.0), (0.0, 100.0, 200.0, 20.0), (0.0, 1.0, 200.0, 0.2), (0.0, 1000.0, 200.0, 200.0)], |r| {
        let got = tick_step(r.0, r.1, r.2);
        if (got - r.3).abs() < 1e-9 { Ok(()) } else { Err(format!("{got}")) }
    });
}

#[test]
fn short_axes_still_get_at_least_four_steps() {
    check(&[(0.0, 10.0, 50.0, 2.5), (0.0, 10.0, 35.0, 2.5), (0.0, 1.0, 50.0, 0.25), (0.0, 100.0, 70.0, 25.0)], |r| {
        let got = tick_step(r.0, r.1, r.2);
        if (got - r.3).abs() < 1e-9 { Ok(()) } else { Err(format!("{got}")) }
    });
}

#[test]
fn tick_steps_are_one_two_or_five_times_a_power() {
    for i in 1..400 {
        let range = i as f64 * 0.37;
        let s = tick_step(0.0, range, 200.0);
        let mut m = s;
        while m >= 10.0 {
            m /= 10.0;
        }
        while m < 1.0 {
            m *= 10.0;
        }
        assert!([1.0, 2.0, 2.5, 5.0, 10.0].iter().any(|n| (m - n).abs() < 1e-6), "{range}: {s}");
    }
}

#[test]
fn tick_counts_stay_readable() {
    for i in 1..400 {
        let range = i as f64 * 0.37;
        let n = range / tick_step(0.0, range, 200.0);
        assert!(n >= 2.0 && n <= 12.0, "{range}: {n}");
    }
}

#[test]
fn longer_axes_get_finer_ticks() {
    assert!(tick_step(0.0, 10.0, 800.0) < tick_step(0.0, 10.0, 100.0));
}

#[test]
fn empty_ranges_get_a_unit_step() {
    assert_eq!(tick_step(1.0, 1.0, 200.0), 1.0);
    assert_eq!(tick_step(2.0, 1.0, 200.0), 1.0);
    assert_eq!(tick_step(f64::NAN, 1.0, 200.0), 1.0);
}

#[test]
fn numbers_print_without_trailing_zeros() {
    check(&[(0.0, "0"), (1.0, "1"), (10.0, "10"), (1000.0, "1000"), (0.5, "0.5"), (0.25, "0.25"), (1.5, "1.5"), (-1.0, "-1"), (-0.5, "-0.5"), (2.5, "2.5"), (0.125, "0.125"), (0.0625, "0.0625")], |r| if format(r.0) == r.1 { Ok(()) } else { Err(format(r.0)) });
}

#[test]
fn tiny_negative_numbers_print_as_zero() {
    assert_eq!(format(-0.000001), "0");
    assert_eq!(format(-0.0), "0");
}

#[test]
fn float_noise_is_rounded_away() {
    assert_eq!(format(0.1 + 0.2), "0.3");
    assert_eq!(format(1.0000000001), "1");
}

#[test]
fn huge_numbers_fit_the_buffer() {
    format(1e15);
    format(-1e15);
}

#[test]
fn inside_segments_are_untouched() {
    let got = clip((1.0, 1.0), (9.0, 9.0), &map()).unwrap();
    assert_eq!(got, ((1.0, 1.0), (9.0, 9.0)));
}

#[test]
fn outside_segments_are_dropped() {
    assert!(clip((11.0, 11.0), (12.0, 12.0), &map()).is_none());
    assert!(clip((-1.0, 5.0), (-1.0, 6.0), &map()).is_none());
    assert!(clip((11.0, 0.0), (20.0, 5.0), &map()).is_none());
}

#[test]
fn crossing_segments_are_cut_at_the_edge() {
    assert_eq!(clip((5.0, 5.0), (15.0, 5.0), &map()).unwrap(), ((5.0, 5.0), (10.0, 5.0)));
    assert_eq!(clip((-5.0, 5.0), (5.0, 5.0), &map()).unwrap(), ((0.0, 5.0), (5.0, 5.0)));
    assert_eq!(clip((5.0, -5.0), (5.0, 15.0), &map()).unwrap(), ((5.0, 0.0), (5.0, 10.0)));
}

#[test]
fn diagonal_segments_keep_their_slope() {
    let (a, b) = clip((-10.0, -10.0), (20.0, 20.0), &map()).unwrap();
    assert_eq!(a, (0.0, 0.0));
    assert_eq!(b, (10.0, 10.0));
}

#[test]
fn every_mark_name_resolves() {
    check(&[("*", MARK_DOT), ("o", MARK_RING), ("square*", MARK_SQUARE), ("square", MARK_BOX), ("triangle*", MARK_TRIANGLE), ("triangle", MARK_DELTA), ("x", MARK_CROSS), ("+", MARK_PLUS), ("none", MARK_NONE)], |r| if mark_value(r.0.as_bytes()) == Some(r.1) { Ok(()) } else { Err("wrong".into()) });
    assert_eq!(mark_value(b"nope"), None);
}
