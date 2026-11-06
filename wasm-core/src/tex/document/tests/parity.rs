//! Lengths here are what pdflatex and the Computer Modern TFM files give, at
//! 100 units to the point in a 10pt document.
use super::*;

fn glyph_x(d: &Done) -> Vec<i32> {
    let mut xs: Vec<i32> = d.items(0).iter().filter(|it| it.kind == IT_GLYPH).map(|it| it.x).collect();
    xs.sort();
    xs
}

fn step(source: &str, from: usize) -> i32 {
    let xs = glyph_x(&body(source));
    xs[from + 1] - xs[from]
}

fn near(got: i32, want: i32) {
    assert!((got - want).abs() <= 3, "got {got}, want {want}");
}

#[test]
fn letters_kern_as_their_font_says() {
    near(step("\\noindent AV", 0), 750 - 111);
    near(step("\\noindent To", 0), 722 - 83);
    near(step("\\noindent \\textbf{AV}", 0), 869 - 128);
}

#[test]
fn a_program_runs_on_through_a_label_inside_it() {
    // A's kerns continue into the list L shares: V is only in the latter.
    near(step("\\noindent LV", 0), 625 - 111);
}

#[test]
fn ligatures_take_the_width_of_the_ligature() {
    near(step("\\noindent fi", 0), 556 - 278);
    let xs = glyph_x(&body("\\noindent ffi x"));
    near(xs[3] - xs[0], 833 + 333);
}

#[test]
fn kerns_stop_at_a_space_and_at_an_explicit_kern() {
    near(step("\\noindent A V", 0), 750 + 333);
    near(step("\\noindent A\\,V", 0), 750 + 170);
}

#[test]
fn each_size_uses_its_own_font_file() {
    near(step("\\noindent\\small ab", 0), 463);
    near(step("\\noindent\\footnotesize ab", 0), 425);
    near(step("\\noindent\\large ab", 0), 588);
}

#[test]
fn scripts_use_the_files_drawn_for_them() {
    let xs = glyph_x(&body("\\noindent $x_1y$"));
    near(xs[2] - xs[0], 572 + 399 + 50);
}

#[test]
fn angle_brackets_are_not_the_ot1_characters_at_their_codes() {
    let xs = glyph_x(&body("\\noindent $x_{<}y$"));
    near(xs[2] - xs[0], 1247);
}

#[test]
fn spaces_belong_to_the_font_they_are_set_in() {
    near(step("\\noindent \\textbf{a b}", 0), 559 + 383);
    near(step("\\noindent \\textit{a b}", 0), 511 + 358);
    near(step("\\noindent \\texttt{a b}", 0), 525 + 525);
    near(step("\\noindent \\small a b", 0), 463 + 308);
}

#[test]
fn a_control_word_swallows_the_space_after_it() {
    near(step("\\newcommand{\\g}{H}\\noindent\\g after", 0), 750);
    near(step("\\newcommand{\\g}{H}\\noindent\\g{} after", 0), 750 + 333);
    let blank = body("\\newcommand{\\g}{H}\\g\n\nafter");
    assert_eq!(blank.baselines(0).len(), 2);
}

#[test]
fn a_control_space_ignores_the_space_factor() {
    near(step("\\noindent Mr.~X", 2), 278 + 333);
    near(step("\\noindent Mr.\\ X", 2), 278 + 333);
    near(step("\\noindent Mr. X", 2), 278 + 333 + 111);
}

#[test]
fn italics_end_with_their_correction() {
    let plain = glyph_x(&body("\\noindent \\textit{f}x"));
    let stopped = glyph_x(&body("\\noindent \\textit{f}.x"));
    assert!(plain[1] - plain[0] > 306, "{plain:?}");
    near(stopped[1] - stopped[0], 306);
}

#[test]
fn text_dots_are_three_stops_and_kerns() {
    let xs = glyph_x(&body("\\noindent\\dots x"));
    near(xs[3] - xs[0], 3 * 278 + 3 * 167);
}

#[test]
fn amsmath_leaves_a_thin_space_after_closing_dots() {
    let dots = |preamble: &str| {
        let d = good(&with(preamble, "\\noindent $\\dots$x"));
        let xs = glyph_x(&d);
        xs[xs.len() - 1] - xs[0]
    };
    near(dots("\\usepackage{amsmath}"), 1333);
    near(dots(""), 1167);
}

#[test]
fn a_line_break_keeps_its_vertical_skip() {
    let d = body("\\begin{center}one\\\\[1.2em]two\\end{center}");
    let lines = d.baselines(0);
    near(lines[1] - lines[0], 1200 + 1200);
}

#[test]
fn a_tabular_sits_on_the_axis() {
    let d = body("\\noindent\\begin{tabular}{l}a\\\\b\\end{tabular}\n\nAfter");
    let lines = d.baselines(0);
    near(lines[lines.len() - 1] - lines[lines.len() - 2], 360 + 694 + 100);
}

#[test]
fn text_straight_after_a_center_is_not_indented() {
    let run = body("\\begin{center}x\\end{center}\nAfter");
    let gap = body("\\begin{center}x\\end{center}\n\nAfter");
    let left = |d: &Done| d.items(0).iter().map(|it| it.x).min().unwrap();
    assert_eq!(left(&gap) - left(&run), 1500);
}

#[test]
fn the_side_margin_is_a_whole_number_of_points() {
    let d = body("\\noindent H");
    near(d.items(0).iter().map(|it| it.x).min().unwrap(), 13427);
}

#[test]
fn page_costs_follow_the_page_builder() {
    assert_eq!(page_cost(0, 0, 0), 0);
    assert_eq!(page_cost(100, 0, 0), DEPLORABLE);
    assert_eq!(page_cost(100, 100, 150), 250);
    assert_eq!(page_cost(0, 0, NO_BREAK), i64::MAX);
}

#[test]
fn centered_node_text_balances_its_lines() {
    let text = "Applying Method M twice returns the original values, so Player C's completed line reproduces the main diagonal of Player A's cube.";
    let d = good(&with(
        "\\usepackage{tikz}",
        &format!("\\begin{{tikzpicture}}\\node[align=center, font=\\small, text width=12cm] {{{text}}};\\end{{tikzpicture}}"),
    ));
    let mut rows: Vec<(i32, usize)> = Vec::new();
    for it in d.items(0).iter().filter(|it| it.kind == IT_GLYPH) {
        match rows.iter_mut().find(|row| (row.0 - it.y).abs() < 300) {
            Some(row) => row.1 += 1,
            None => rows.push((it.y, 1)),
        }
    }
    rows.sort();
    let letters = |s: &str| s.chars().filter(|ch| !ch.is_whitespace()).count();
    assert_eq!(rows[0].1, letters("Applying Method M twice returns the original values, so Player C's"), "{rows:?}");
}
