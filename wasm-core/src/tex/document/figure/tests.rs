use super::*;
use crate::tex::tests::check;

fn near(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance * (1.0 + b.abs())
}

fn eval(s: &str) -> f64 {
    evaluate(s.as_bytes(), 0.0, 0.0).unwrap_or_else(|| panic!("{s}")).0
}

fn stat(s: &str) -> &'static [u8] {
    Box::leak(s.as_bytes().to_vec().into_boxed_slice())
}

#[test]
fn sine_matches_the_library_everywhere() {
    for i in -7200..7200 {
        let d = i as f64 * 0.5;
        assert!(near(sin_deg(d), d.to_radians().sin(), 1e-9), "{d}");
    }
}

#[test]
fn cosine_matches_the_library_everywhere() {
    for i in -7200..7200 {
        let d = i as f64 * 0.5;
        assert!(near(cos_deg(d), d.to_radians().cos(), 1e-9), "{d}");
    }
}

#[test]
fn sine_is_exact_at_the_quadrants() {
    assert!(sin_deg(0.0).abs() < 1e-12);
    assert!((sin_deg(90.0) - 1.0).abs() < 1e-9);
    assert!(sin_deg(180.0).abs() < 1e-9);
    assert!((sin_deg(270.0) + 1.0).abs() < 1e-9);
}

#[test]
fn exp_matches_the_library() {
    for i in -300..300 {
        let x = i as f64 * 0.1;
        assert!(near(exp(x), x.exp(), 1e-9), "{x}");
    }
}

#[test]
fn ln_matches_the_library() {
    for i in 1..4000 {
        let x = i as f64 * 0.05;
        assert!(near(ln(x), x.ln(), 1e-9), "{x}");
    }
    for x in [1e-9, 1e-3, 1e3, 1e9, 1e100] {
        assert!(near(ln(x), f64::ln(x), 1e-9), "{x}");
    }
}

#[test]
fn power_matches_the_library() {
    for (a, b) in [(2.0, 10.0), (2.0, 0.5), (10.0, -3.0), (0.5, 3.0), (3.0, 0.0), (1.5, 2.5), (7.0, 1.0)] {
        assert!(near(power(a, b), f64::powf(a, b), 1e-9), "{a} {b}");
    }
}

#[test]
fn negative_bases_work_with_whole_powers() {
    assert!(near(power(-2.0, 2.0), 4.0, 1e-9));
    assert!(near(power(-2.0, 3.0), -8.0, 1e-9));
}

#[test]
fn floor_matches_the_library() {
    for v in [0.0, 0.5, -0.5, 1.0, -1.0, 2.9, -2.1, 1e6 + 0.5] {
        assert_eq!(floor(v), f64::floor(v), "{v}");
    }
}

#[test]
fn arithmetic_follows_precedence() {
    check(&[("1+2*3", 7.0), ("(1+2)*3", 9.0), ("2^3", 8.0), ("2*3^2", 18.0), ("10/4", 2.5), ("10-3-2", 5.0), ("2+-3", -1.0), ("-2+3", 1.0), ("--2", 2.0), ("+2", 2.0), ("1.5*2", 3.0), (".5*4", 2.0), ("1e3", 1000.0), ("2.5e-1", 0.25), (" 1 + 2 ", 3.0), ("((((1))))", 1.0)], |r| if near(eval(r.0), r.1, 1e-12) { Ok(()) } else { Err(format!("{}", eval(r.0))) });
}

#[test]
fn every_function_evaluates() {
    check(
        &[
            ("sin(30)", 0.5),
            ("cos(60)", 0.5),
            ("tan(45)", 1.0),
            ("exp(0)", 1.0),
            ("exp(1)", core::f64::consts::E),
            ("ln(e)", 1.0),
            ("log(100)", 2.0),
            ("sqrt(16)", 4.0),
            ("abs(-3)", 3.0),
            ("deg(pi)", 180.0),
            ("rad(180)", PI),
            ("floor(2.7)", 2.0),
            ("floor(-2.1)", -3.0),
            ("ceil(2.1)", 3.0),
            ("ceil(-2.7)", -2.0),
            ("round(2.5)", 3.0),
            ("round(2.4)", 2.0),
            ("min(1,2)", 1.0),
            ("max(1,2)", 2.0),
            ("pow(2,5)", 32.0),
            ("pi", PI),
            ("e", core::f64::consts::E),
            ("sin(deg(pi/2))", 1.0),
        ],
        |r| if near(eval(r.0), r.1, 1e-9) { Ok(()) } else { Err(format!("{}", eval(r.0))) },
    );
}

#[test]
fn x_is_the_variable() {
    assert!(near(evaluate(b"x^2+1", 3.0, 0.0).unwrap().0, 10.0, 1e-12));
    assert_eq!(evaluate(b"2x", 3.0, 0.0).unwrap().0, 6.0);
}

#[test]
fn units_mark_a_length() {
    check(&[("1pt", 1.0), ("1cm", CM), ("1mm", CM / 10.0), ("1in", 72.27), ("1em", 10.0), ("1ex", 4.3), ("1bp", 1.00375), ("2.5cm", 2.5 * CM), ("1cm+1pt", CM + 1.0)], |r| {
        let (v, unit) = evaluate(r.0.as_bytes(), 0.0, 0.0).unwrap();
        if near(v, r.1, 1e-9) && unit { Ok(()) } else { Err(format!("{v} {unit}")) }
    });
    assert!(!evaluate(b"2", 0.0, 0.0).unwrap().1);
}

#[test]
fn line_width_is_available() {
    assert_eq!(evaluate(b"0.5\\linewidth", 0.0, 100.0).unwrap(), (50.0, true));
    assert_eq!(evaluate(b"\\textwidth", 0.0, 100.0).unwrap(), (100.0, true));
    assert_eq!(evaluate(b"\\columnwidth", 0.0, 100.0).unwrap(), (100.0, true));
}

#[test]
fn malformed_expressions_are_refused() {
    check(&["", "1+", "*2", "(1", "1)", "nope", "nope(1)", "1 2", "sin(", "sin(1", "\\nope", "1..2"], |s| if evaluate(s.as_bytes(), 0.0, 0.0).is_none() { Ok(()) } else { Err("evaluated".into()) });
}

#[test]
fn deep_expressions_are_refused() {
    let s = "(".repeat(5000) + "1" + &")".repeat(5000);
    assert!(evaluate(s.as_bytes(), 0.0, 0.0).is_none());
    let s = "-".repeat(5000) + "1";
    assert!(evaluate(s.as_bytes(), 0.0, 0.0).is_none());
}

#[test]
fn length_pt_needs_or_assumes_points() {
    assert!(near(length_pt(b"1cm").unwrap(), CM, 1e-9));
    assert_eq!(length_pt(b"10pt").unwrap(), 10.0);
    assert!(length_pt(b"nonsense").is_none());
}

#[test]
fn trim_strips_both_ends() {
    assert_eq!(trim(b"  a b \n"), b"a b");
    assert_eq!(trim(b""), b"");
    assert_eq!(trim(b"   "), b"");
}

#[test]
fn unbrace_removes_one_outer_pair() {
    assert_eq!(unbrace(stat("{a}")), b"a");
    assert_eq!(unbrace(stat(" {a} ")), b"a");
    assert_eq!(unbrace(stat("a")), b"a");
}

#[test]
fn options_split_at_top_level_commas() {
    let text = stat("a=1, b={2,3}, c, d=(1,2), e/.style={f,g=2}");
    let mut at = 0;
    let mut got = Vec::new();
    while let Some((k, v)) = next_option(text, &mut at) {
        got.push((String::from_utf8_lossy(k).into_owned(), String::from_utf8_lossy(v).into_owned()));
    }
    let want = [("a", "1"), ("b", "2,3"), ("c", ""), ("d", "(1,2)"), ("e/.style", "f,g=2")];
    assert_eq!(got.len(), want.len());
    for (g, w) in got.iter().zip(want) {
        assert_eq!((g.0.as_str(), g.1.as_str()), w);
    }
}

#[test]
fn empty_options_yield_nothing() {
    for s in ["", " ", ",", ",,"] {
        let mut at = 0;
        assert!(next_option(stat(s), &mut at).is_none(), "{s:?}");
    }
}

#[test]
fn only_the_first_equals_splits() {
    let mut at = 0;
    let (k, v) = next_option(stat("a=b=c"), &mut at).unwrap();
    assert_eq!((k, v), (&b"a"[..], &b"b=c"[..]));
}

#[test]
fn identity_matrix_changes_nothing() {
    assert_eq!(Mat::IDENTITY.at((3.0, 4.0)), (3.0, 4.0));
    assert_eq!(Mat::IDENTITY.vector((3.0, 4.0)), (3.0, 4.0));
}

#[test]
fn matrices_translate_points_not_vectors() {
    let m = Mat { e: 5.0, f: 6.0, ..Mat::IDENTITY };
    assert_eq!(m.at((1.0, 2.0)), (6.0, 8.0));
    assert_eq!(m.vector((1.0, 2.0)), (1.0, 2.0));
}

#[test]
fn undo_reverses_a_vector() {
    let m = Mat { a: 2.0, b: 1.0, c: -1.0, d: 3.0, e: 0.0, f: 0.0 };
    let v = m.vector((3.0, 4.0));
    let back = m.undo(v);
    assert!(near(back.0, 3.0, 1e-12) && near(back.1, 4.0, 1e-12));
}

#[test]
fn singular_matrices_undo_to_zero() {
    let m = Mat { a: 0.0, b: 0.0, c: 0.0, d: 0.0, e: 0.0, f: 0.0 };
    assert_eq!(m.undo((1.0, 2.0)), (0.0, 0.0));
}

#[test]
fn matrices_compose_in_order() {
    let scale = Mat { a: 2.0, d: 2.0, ..Mat::IDENTITY };
    let shift = Mat { e: 1.0, ..Mat::IDENTITY };
    let p = (1.0, 0.0);
    let both = scale.then(&shift);
    let a = both.at(p);
    let manual = scale.at(shift.at(p));
    assert_eq!(a, manual);
}

#[test]
fn color_names_resolve() {
    check(&[("red", 0xff0000u32), ("green", 0x00ff00), ("blue", 0x0000ff), ("black", 0), ("white", 0xffffff), ("gray", 0x808080), ("grey", 0x808080), ("cyan", 0x00ffff), ("magenta", 0xff00ff), ("yellow", 0xffff00)], |r| if color_by_name(r.0.as_bytes()) == Some(r.1) { Ok(()) } else { Err("wrong".into()) });
    assert_eq!(color_by_name(b"nope"), None);
}

#[test]
fn colors_mix_by_percent() {
    assert_eq!(mix(0xff0000, 0x0000ff, 100.0), 0xff0000);
    assert_eq!(mix(0xff0000, 0x0000ff, 0.0), 0x0000ff);
    assert_eq!(mix(0x000000, 0xffffff, 50.0), 0x808080);
}

#[test]
fn color_expressions_follow_xcolor() {
    check(&[("gray!20", 0xe6e6e6u32), ("red!50", 0xff8080), ("red!100", 0xff0000), ("red!0", 0xffffff), ("red!50!blue", 0x800080), ("green!50!black", 0x008000), ("red!150", 0xff0000), ("red!-10", 0xffffff), ("none", NO_COLOR)], |r| if color_value(r.0.as_bytes()) == Some(r.1) { Ok(()) } else { Err(format!("{:x?}", color_value(r.0.as_bytes()))) });
}

#[test]
fn bad_color_expressions_are_refused() {
    check(&["nope", "red!", "red!50!nope", ""], |s| if color_value(s.as_bytes()).is_none() { Ok(()) } else { Err("resolved".into()) });
}

#[test]
fn tip_names_resolve() {
    check(&[(">", TIP_DEFAULT), ("<", TIP_DEFAULT), ("to", TIP_OPEN), ("To", TIP_OPEN), ("Stealth", TIP_STEALTH), ("stealth", TIP_STEALTH), ("latex", TIP_STEALTH), ("Latex", TIP_STEALTH), ("LaTeX", TIP_STEALTH), ("", TIP_NONE)], |r| if tip_named(stat(r.0)).map(|t| t.kind) == Some(r.1) { Ok(()) } else { Err("wrong".into()) });
    assert!(tip_named(stat("nope")).is_none());
}

#[test]
fn tip_length_is_read() {
    assert_eq!(tip_named(stat("Stealth[length=5pt]")).unwrap().length, 5.0);
    assert_eq!(tip_named(stat("{Stealth[length=5pt]}")).unwrap().length, 5.0);
    assert!(tip_named(stat("Stealth[length=5pt")).is_none());
}

#[test]
fn tip_pairs_split_at_the_dash() {
    let (a, b) = tips(stat("<->")).unwrap();
    assert_eq!((a.kind, b.kind), (TIP_DEFAULT, TIP_DEFAULT));
    let (a, b) = tips(stat("->")).unwrap();
    assert_eq!((a.kind, b.kind), (TIP_NONE, TIP_DEFAULT));
    let (a, b) = tips(stat("-{Stealth[length=5pt]}")).unwrap();
    assert_eq!((a.kind, b.kind), (TIP_NONE, TIP_STEALTH));
    assert!(tips(stat("nodash")).is_none());
}

#[test]
fn placements_point_the_right_way() {
    assert_eq!(placement(b"above"), Some((0, -1)));
    assert_eq!(placement(b"below"), Some((0, 1)));
    assert_eq!(placement(b"left"), Some((1, 0)));
    assert_eq!(placement(b"right"), Some((-1, 0)));
    assert_eq!(placement(b"above left"), Some((1, -1)));
    assert_eq!(placement(b"nope"), None);
}

fn square() -> Extent {
    Extent { w: 2.0, h: 1.0, round: false }
}

#[test]
fn border_of_nothing_is_the_center() {
    assert_eq!(border((1.0, 1.0), Extent::default(), (5.0, 5.0)), (1.0, 1.0));
}

#[test]
fn border_toward_the_same_point_is_the_center() {
    assert_eq!(border((1.0, 1.0), square(), (1.0, 1.0)), (1.0, 1.0));
}

#[test]
fn box_borders_stop_at_each_side() {
    assert_eq!(border((0.0, 0.0), square(), (10.0, 0.0)), (2.0, 0.0));
    assert_eq!(border((0.0, 0.0), square(), (-10.0, 0.0)), (-2.0, 0.0));
    assert_eq!(border((0.0, 0.0), square(), (0.0, 10.0)), (0.0, 1.0));
    assert_eq!(border((0.0, 0.0), square(), (0.0, -10.0)), (0.0, -1.0));
}

#[test]
fn box_borders_hit_the_nearer_side_on_a_diagonal() {
    let p = border((0.0, 0.0), square(), (10.0, 10.0));
    assert!(near(p.0, 1.0, 1e-12) && near(p.1, 1.0, 1e-12), "{p:?}");
}

#[test]
fn circle_borders_are_at_the_radius() {
    let e = Extent { w: 5.0, h: 5.0, round: true };
    for a in [0.0f64, 30.0, 45.0, 123.0, 200.0, 359.0] {
        let target = (a.to_radians().cos() * 50.0, a.to_radians().sin() * 50.0);
        let p = border((0.0, 0.0), e, target);
        assert!(near((p.0 * p.0 + p.1 * p.1).sqrt(), 5.0, 1e-9), "{a}");
    }
}

#[test]
fn borders_work_away_from_the_origin() {
    assert_eq!(border((10.0, 10.0), square(), (20.0, 10.0)), (12.0, 10.0));
}
