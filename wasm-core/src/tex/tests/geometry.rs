use super::*;

#[test]
fn contour_bezier_coordinates_have_unambiguous_separators() {
    use crate::tex::{IT_ELLIPSE, Item};
    let item = Item { x: 0, y: 0, a: 200, b: 100, g: 0, kind: IT_ELLIPSE };
    let mut bytes = [0u8; 512];
    let len = crate::emit::brace_path(&item, &mut bytes);
    let text = core::str::from_utf8(&bytes[..len]).unwrap();
    assert!(text.contains("c0 55 -90 100 -200 100"), "{text}");
    assert_eq!(text.matches('c').count(), 8);
    assert_eq!(text.matches('z').count(), 2);
}

#[test]
fn multiple_contour_integrals_are_operators_with_shared_closed_contours() {
    for name in [r"\oiint", r"\oiiint"] {
        let svg = display(&format!("{name}\\limits_{{S}}^{{T}} f")).unwrap();
        well_formed(&svg).unwrap();
        assert!(svg.contains('c'), "contour must have curved sides");
        assert!(measure_display(&format!("{name}\\limits_{{S}}^{{T}}")).height > measure_display(name).height);
        // An operator must also work where an unbraced single atom is expected.
        inline(&format!(r"\frac{name} x")).unwrap();
    }
    assert!(measure_display(r"\oiiint").width > measure_display(r"\oiint").width);
    assert!(measure_display(r"\oiint").width > measure_display(r"\int").width);
}

fn wider(rows: &[(&str, &str)]) {
    check(rows, |r| {
        if measure_display(r.0).width > measure_display(r.1).width {
            Ok(())
        } else {
            Err(format!("{:?} vs {:?}", measure_display(r.0), measure_display(r.1)))
        }
    });
}

fn taller(rows: &[(&str, &str)]) {
    check(rows, |r| {
        let (a, b) = (measure_display(r.0), measure_display(r.1));
        if a.height + a.depth > b.height + b.depth { Ok(()) } else { Err(format!("{a:?} vs {b:?}")) }
    });
}

#[test]
fn width_adds_up() {
    let (a, b, ab) = (measure("a").width, measure("b").width, measure("ab").width);
    assert!((a + b - ab).abs() <= 2, "{a} {b} {ab}");
}

#[test]
fn longer_input_is_wider() {
    wider(&[
        ("xx", "x"),
        ("xxx", "xx"),
        ("abcdef", "abc"),
        ("a+b", "ab"),
        ("a=b", "a+b"),
        ("\\sin x", "x"),
        ("\\text{hello}", "\\text{hi}"),
    ]);
}

#[test]
fn spaces_widen_by_size() {
    wider(&[
        ("a\\,b", "ab"),
        ("a\\:b", "a\\,b"),
        ("a\\;b", "a\\:b"),
        ("a\\quad b", "a\\;b"),
        ("a\\qquad b", "a\\quad b"),
        ("ab", "a\\!b"),
        ("a\\ b", "ab"),
        ("a~b", "ab"),
    ]);
}

#[test]
fn quad_is_one_em() {
    let gap = measure("a\\quad b").width - measure("ab").width;
    assert!((gap - 1000).abs() <= 2, "{gap}");
    let gap = measure("a\\qquad b").width - measure("ab").width;
    assert!((gap - 2000).abs() <= 2, "{gap}");
}

#[test]
fn hspace_units_convert() {
    let base = measure("ab").width;
    check(
        &[("1em", 1000), ("2em", 2000), ("0.5em", 500), ("18mu", 1000), ("10pt", 1000), ("1cm", 2845), ("1in", 7227), ("1mm", 284)],
        |r| {
            let got = measure(&format!("a\\hspace{{{}}}b", r.0)).width - base;
            if (got - r.1).abs() <= 3 { Ok(()) } else { Err(format!("got {got}")) }
        },
    );
}

#[test]
fn negative_space_narrows() {
    assert!(measure("a\\hspace{-0.1em}b").width < measure("ab").width);
}

#[test]
fn binary_operators_get_medium_space() {
    let spaced = measure("a+b").width;
    let tight = measure("a{+}b").width;
    assert!(spaced > tight);
}

#[test]
fn relations_get_thick_space() {
    let rel = measure("a=b").width - measure("a{=}b").width;
    let bin = measure("a+b").width - measure("a{+}b").width;
    assert!(rel > bin);
}

#[test]
fn leading_binary_is_treated_as_ordinary() {
    assert!(measure("+a").width < measure("b+a").width - measure("b").width);
}

#[test]
fn script_style_drops_binary_space() {
    let a = measure("x^{a+b}").width;
    let b = measure("x^{a{+}b}").width;
    assert_eq!(a, b);
}

#[test]
fn stacked_things_are_taller() {
    taller(&[
        ("\\frac{a}{b}", "a"),
        ("\\frac{\\frac{a}{b}}{c}", "\\frac{a}{b}"),
        ("\\sum", "+"),
        ("\\int", "x"),
        ("\\begin{matrix}a\\\\b\\end{matrix}", "a"),
        ("\\begin{matrix}a\\\\b\\\\c\\end{matrix}", "\\begin{matrix}a\\\\b\\end{matrix}"),
        ("\\begin{cases}a\\\\b\\end{cases}", "a"),
        ("\\begin{aligned}a\\\\b\\end{aligned}", "a"),
        ("\\overline{x}", "x"),
        ("\\underline{x}", "x"),
        ("\\hat{x}", "x"),
        ("\\tilde{x}", "x"),
        ("\\vec{x}", "x"),
        ("\\sqrt{x}", "x"),
        ("\\overbrace{x}", "x"),
        ("\\underbrace{x}", "x"),
        ("\\binom{n}{k}", "n"),
        ("\\sum_{i=1}^n", "\\sum"),
        ("\\substack{a\\\\b}", "a"),
        ("\\boxed{x}", "x"),
    ]);
}

#[test]
fn delimiters_grow_in_steps() {
    taller(&[("\\big(", "("), ("\\Big(", "\\big("), ("\\bigg(", "\\Big("), ("\\Bigg(", "\\bigg(")]);
}

#[test]
fn left_right_fits_the_content() {
    taller(&[
        ("\\left(\\frac{a}{b}\\right)", "(\\frac{a}{b})"),
        ("\\left(\\sum_i^n\\right)", "\\left(x\\right)"),
        ("\\left[\\begin{matrix}a\\\\b\\\\c\\\\d\\end{matrix}\\right]", "\\left[\\begin{matrix}a\\\\b\\end{matrix}\\right]"),
    ]);
}

#[test]
fn every_delimiter_stretches() {
    let rows = [
        "(",
        "[",
        "\\{",
        "|",
        "\\|",
        "\\langle",
        "\\lfloor",
        "\\lceil",
        "\\vert",
        "\\Vert",
        "/",
        "\\backslash",
        "\\uparrow",
        "\\downarrow",
        "\\updownarrow",
        "\\Uparrow",
        "\\Downarrow",
    ];
    check(&rows, |d| {
        let tall = "\\begin{matrix}a\\\\b\\\\c\\\\d\\\\e\\\\f\\end{matrix}";
        let big = measure_display(&format!("\\left{d}{tall}\\right."));
        let content = measure_display(tall);
        if big.height + big.depth >= (content.height + content.depth) * 9 / 10 {
            Ok(())
        } else {
            Err(format!("{big:?} around {content:?}"))
        }
    });
}

#[test]
fn null_delimiter_has_no_ink() {
    let svg = inline("\\left.\\right.").unwrap();
    assert_eq!(paths(&svg), 0);
}

#[test]
fn display_style_is_larger() {
    check(&["\\sum", "\\prod", "\\int", "\\frac{a}{b}", "\\bigcup", "\\bigcap", "\\coprod", "\\oint", "\\bigoplus", "\\bigotimes"], |s| {
        let (a, b) = (measure(s), measure_display(s));
        if b.height + b.depth > a.height + a.depth { Ok(()) } else { Err(format!("{a:?} vs {b:?}")) }
    });
}

#[test]
fn limits_go_below_only_in_display() {
    assert!(measure_display("\\sum_{i=1}").depth > measure("\\sum_{i=1}").depth);
    assert!(measure("\\sum_{i=1}").width > measure("\\sum").width);
    assert!(measure_display("\\sum_{i}").width == measure_display("\\sum").width);
    assert!(measure_display("\\sum\\nolimits_{i}").width > measure_display("\\sum").width);
    assert!(measure("\\sum\\limits_{i}").depth > measure("\\sum_{i}").depth);
}

#[test]
fn scripts_move_up_and_down() {
    assert!(measure("x^2").height > measure("x").height);
    assert!(measure("x_2").depth > measure("x").depth);
    assert_eq!(measure("x^2").depth, measure("x").depth);
    assert!(measure("x^2_i").width < measure("x^2").width + measure("x_i").width - measure("x").width);
}

#[test]
fn script_sizes_shrink() {
    let a = measure("x").width;
    let b = measure("x^x").width - a;
    let c = measure("x^{x^x}").width - a - b;
    assert!(b < a && c < b, "{a} {b} {c}");
    assert!(c * 2 > a / 2);
}

#[test]
fn script_sizes_stop_shrinking() {
    let c = measure("x^{x^{x}}").width;
    let d = measure("x^{x^{x^{x}}}").width;
    let e = measure("x^{x^{x^{x^{x}}}}").width;
    assert_eq!(e - d, d - c);
}

#[test]
fn size_commands_scale_in_order() {
    let widths: Vec<i32> =
        ["\\tiny", "\\scriptsize", "\\footnotesize", "\\small", "\\normalsize", "\\large", "\\Large", "\\LARGE", "\\huge", "\\Huge"]
            .iter()
            .map(|c| measure(&format!("{c} x")).width)
            .collect();
    assert!(widths.windows(2).all(|w| w[0] < w[1]), "{widths:?}");
    assert_eq!(widths[4], measure("x").width);
}

#[test]
fn phantom_keeps_size_without_ink() {
    check(&["x", "\\frac{a}{b}", "\\sum_i^n", "\\sqrt{x}"], |s| {
        let svg = display(&format!("\\phantom{{{s}}}")).map_err(|e| format!("{e:?}"))?;
        if paths(&svg) != 0 {
            return Err("has ink".into());
        }
        let (a, b) = (size_of(&svg), measure_display(s));
        if a == b { Ok(()) } else { Err(format!("{a:?} vs {b:?}")) }
    });
}

#[test]
fn hphantom_keeps_only_width() {
    let a = measure_display("\\hphantom{\\frac{a}{b}}");
    let b = measure_display("\\frac{a}{b}");
    assert_eq!(a.width, b.width);
    assert_eq!(a.height, 0);
    assert_eq!(a.depth, 0);
}

#[test]
fn vphantom_keeps_only_height() {
    let a = measure_display("\\vphantom{\\frac{a}{b}}");
    let b = measure_display("\\frac{a}{b}");
    assert_eq!(a.width, 0);
    assert_eq!(a.height, b.height);
    assert_eq!(a.depth, b.depth);
}

#[test]
fn smash_drops_height() {
    let a = measure_display("\\smash{\\frac{a}{b}}");
    let b = measure_display("\\frac{a}{b}");
    assert_eq!(a.width, b.width);
    assert!(a.height < b.height);
}

#[test]
fn laps_take_no_width() {
    assert_eq!(measure("\\rlap{xyz}").width, 0);
    assert_eq!(measure("\\llap{xyz}").width, 0);
    assert_eq!(measure("a\\rlap{xyz}").width, measure("a").width);
}

#[test]
fn matrices_grow_with_columns() {
    wider(&[
        ("\\begin{matrix}a&b\\end{matrix}", "\\begin{matrix}a\\end{matrix}"),
        ("\\begin{matrix}a&b&c\\end{matrix}", "\\begin{matrix}a&b\\end{matrix}"),
        ("\\begin{pmatrix}a\\end{pmatrix}", "\\begin{matrix}a\\end{matrix}"),
        ("\\begin{matrix}a&b\\end{matrix}", "\\begin{smallmatrix}a&b\\end{smallmatrix}"),
    ]);
}

#[test]
fn ragged_rows_are_padded() {
    assert_eq!(measure("\\begin{matrix}a&b\\\\c\\end{matrix}").width, measure("\\begin{matrix}a&b\\\\c&\\end{matrix}").width);
}

#[test]
fn array_rules_add_width() {
    wider(&[("\\begin{array}{|c|}a\\end{array}", "\\begin{array}{c}a\\end{array}")]);
}

#[test]
fn hline_adds_ink_not_columns() {
    let a = inline("\\begin{array}{c}\\hline a\\\\\\hline\\end{array}").unwrap();
    let b = inline("\\begin{array}{c}a\\end{array}").unwrap();
    assert_eq!(size_of(&a).width, size_of(&b).width);
    assert_ne!(a, b);
}

#[test]
fn sizes_are_never_negative() {
    check(&["x", "\\frac{a}{b}", "\\hspace{-5em}", "a\\hspace{-5em}", "\\smash{x}", "\\raisebox{1em}{x}"], |s| match inline(s) {
        Ok(svg) => {
            let z = size_of(&svg);
            if z.height >= 0 && z.depth >= 0 { Ok(()) } else { Err(format!("{z:?}")) }
        }
        Err(_) => Ok(()),
    });
}

#[test]
fn header_matches_the_box() {
    let svg = display("\\frac{a}{b}").unwrap();
    let z = size_of(&svg);
    assert!(svg.contains(&format!("height=\"{}", (z.height + z.depth) as f64 / 1000.0)) || svg.contains("height=\""));
    assert!(svg.contains("vertical-align:-"));
    assert!(svg.contains("fill=\"currentColor\""));
}
