use super::*;

fn figure(text: &str) -> String {
    format!("\\begin{{figure}}{text}\\caption{{C}}\\end{{figure}}")
}

#[test]
fn tables_place_every_cell() {
    assert_eq!(body("\\begin{tabular}{cc}a&b\\\\c&d\\end{tabular}").glyphs(), 4);
}

#[test]
fn table_columns_line_up() {
    let d = body("\\begin{tabular}{ll}a&b\\\\wide&c\\end{tabular}");
    let col: Vec<i32> = d.all_items().iter().filter(|i| i.kind == IT_GLYPH).map(|i| i.x).collect();
    assert_eq!(col[0], col[2]);
    assert_eq!(col[1], col[6]);
}

#[test]
fn right_columns_align_right_edges() {
    let d = body("\\begin{tabular}{r}a\\\\bbb\\end{tabular}");
    let right: Vec<i32> = d.all_items().iter().filter(|i| i.kind == IT_GLYPH).map(|i| i.x + crate::data::GLYPHS[i.a as usize].w as i32).collect();
    assert!((right[0] - right[3]).abs() <= 2, "{right:?}");
}

#[test]
fn center_columns_center() {
    let d = body("\\begin{tabular}{c}a\\\\aaa\\end{tabular}");
    let xs: Vec<i32> = d.all_items().iter().filter(|i| i.kind == IT_GLYPH).map(|i| i.x).collect();
    assert_eq!(xs[0], xs[2]);
}

#[test]
fn every_rule_draws() {
    let rules = |s: &str| body(s).all_items().iter().filter(|i| i.kind == IT_RULE).count();
    assert_eq!(rules("\\begin{tabular}{c}a\\end{tabular}"), 0);
    assert_eq!(rules("\\begin{tabular}{c}\\hline a\\end{tabular}"), 1);
    assert_eq!(rules("\\begin{tabular}{c}\\toprule a\\\\\\midrule b\\\\\\bottomrule\\end{tabular}"), 3);
    assert_eq!(rules("\\begin{tabular}{|c|}a\\end{tabular}"), 2);
}

#[test]
fn booktabs_rules_differ_in_weight() {
    let d = body("\\begin{tabular}{c}\\toprule a\\\\\\midrule b\\\\\\bottomrule\\end{tabular}");
    let t: Vec<i32> = d.all_items().iter().filter(|i| i.kind == IT_RULE).map(|i| i.b.abs()).collect();
    assert!(t[0] > t[1], "{t:?}");
    assert_eq!(t[0], t[2]);
}

#[test]
fn tabcolsep_changes_the_width() {
    let w = |sep: &str| {
        let d = body(&format!("\\setlength{{\\tabcolsep}}{{{sep}}}\\begin{{tabular}}{{cc}}a&b\\end{{tabular}}"));
        let xs: Vec<i32> = d.all_items().iter().map(|i| i.x).collect();
        xs[1] - xs[0]
    };
    assert!(w("20pt") > w("1pt"));
}

#[test]
fn table_cells_hold_math_and_font_switches() {
    assert_eq!(body("\\begin{tabular}{cc}$x^2$&\\textbf{b}\\end{tabular}").glyphs(), 3);
}

#[test]
fn table_size_switch_scales_it() {
    let d = body("\\begin{table}\\footnotesize\\begin{tabular}{c}a\\end{tabular}\\end{table}");
    assert!(d.laid().groups.iter().any(|g| g.scale < 1000));
}

#[test]
fn table_column_spec_is_validated() {
    assert!(compile(&article("\\begin{tabular}{q}a\\end{tabular}")).is_err());
}

#[test]
fn table_must_be_closed() {
    assert!(compile(&article("\\begin{tabular}{c}a")).is_err());
}

#[test]
fn captions_are_numbered() {
    let one = body(&figure("x")).glyphs();
    let bare = body("\\begin{figure}x\\end{figure}").glyphs();
    assert_eq!(one - bare, "Figure1:C".len());
}

#[test]
fn figures_and_tables_count_separately() {
    let a = body(&format!("{}{}", figure("x"), "\\begin{table}x\\caption{C}\\end{table}")).glyphs();
    assert_eq!(a, 1 + "Figure1:C".len() + 1 + "Table1:C".len());
}

#[test]
fn ten_figures_need_two_digits() {
    let s: String = (0..10).map(|_| figure("x")).collect();
    let n = "Figure1:C".len() + 1;
    assert_eq!(body(&s).glyphs(), 10 * n + 1);
}

#[test]
fn short_captions_are_centered() {
    let d = body("\\begin{figure}\\caption{C}\\end{figure}");
    let middle = (d.left_edge(0) + d.right_edge(0)) / 2;
    assert!((middle - d.laid().w / 2).abs() < 500);
}

#[test]
fn long_captions_wrap() {
    let d = body(&format!("\\begin{{figure}}\\caption{{{}}}\\end{{figure}}", words(80)));
    assert!(d.baselines(0).len() > 2);
}

#[test]
fn captions_can_hold_math() {
    assert_eq!(body("\\begin{figure}\\caption{$x$}\\end{figure}").glyphs(), "Figure1:".len() + 1);
}

#[test]
fn caption_skip_changes_the_gap() {
    let gap = |pre: &str| {
        let d = good(&with(pre, "\\begin{figure}x\\caption{C}\\end{figure}"));
        let y = d.baselines(0);
        y[1] - y[0]
    };
    assert!(gap("\\captionsetup{skip=30pt}") > gap("\\captionsetup{skip=1pt}"));
}

#[test]
fn floats_stay_where_they_are_written() {
    let d = body(&format!("first\n\n{}\n\nlast", figure("x")));
    let ys = d.baselines(0);
    assert_eq!(ys.len(), 4);
}

#[test]
fn every_placement_option_is_accepted() {
    for p in ["h", "H", "t", "b", "p", "!", "htbp", "!ht"] {
        good(&article(&format!("\\begin{{figure}}[{p}]x\\end{{figure}}")));
    }
}

#[test]
fn wide_floats_move_to_the_next_page_top() {
    let s = format!("\\documentclass[twocolumn]{{article}}\\begin{{document}}a\\begin{{figure*}}[t]\\centering x\\caption{{C}}\\end{{figure*}}\n{}\\end{{document}}", words(1800));
    let d = good(&s);
    assert!(d.pages >= 2);
    let top = d.top_edge(1);
    let first_glyph = d.items(1).iter().find(|i| i.y == top).unwrap();
    assert!(first_glyph.x > d.laid().w / 4, "{}", first_glyph.x);
}

#[test]
fn wide_float_on_the_last_page_gets_its_own_page() {
    let s = "\\documentclass[twocolumn]{article}\\begin{document}a\\begin{figure*}x\\end{figure*}\\end{document}";
    assert_eq!(good(s).pages, 2);
}

#[test]
fn wide_floats_span_both_columns() {
    let s = format!("\\documentclass[twocolumn]{{article}}\\begin{{document}}\\begin{{figure*}}\\caption{{{}}}\\end{{figure*}}a\\newpage b\\end{{document}}", words(120));
    let d = good(&s);
    let last = d.pages - 1;
    assert!(d.right_edge(last) - d.left_edge(last) > 30000);
}

#[test]
fn text_flows_below_a_wide_float() {
    let s = format!("\\documentclass[twocolumn]{{article}}\\begin{{document}}a\\begin{{figure*}}x\\end{{figure*}}\n{}\\end{{document}}", words(1800));
    let d = good(&s);
    let ys = d.baselines(1);
    assert!(ys.len() > 10);
    let mut seen = 0;
    for y in &ys {
        seen += d.items(1).iter().filter(|i| i.y == *y).count();
    }
    assert!(seen > 100);
}

#[test]
fn a_float_too_tall_for_the_column_moves_on() {
    let tall: String = (0..30).map(|_| "\\draw (0,0) -- (0,1);").collect();
    let s = format!("{}\n\n\\begin{{figure}}\\begin{{tikzpicture}}\\draw (0,0) -- (0,20);{tall}\\end{{tikzpicture}}\\end{{figure}}", words(700));
    let d = good(&with("\\usepackage{tikz}", &s));
    assert!(d.pages >= 2);
    let art_pages: Vec<usize> = (0..d.pages).map(|p| d.art(p).len()).collect();
    assert_eq!(art_pages[0], 0, "{art_pages:?}");
}

#[test]
fn unclosed_floats_are_errors() {
    assert!(compile(&article("\\begin{figure}x")).is_err());
    assert!(compile(&article("\\begin{table}x")).is_err());
    assert!(compile(&article("\\begin{figure}x\\end{table}")).is_err());
}
