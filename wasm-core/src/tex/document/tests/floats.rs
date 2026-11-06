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
    let right: Vec<i32> =
        d.all_items().iter().filter(|i| i.kind == IT_GLYPH).map(|i| i.x + crate::data::GLYPHS[i.a as usize].w as i32).collect();
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
fn table_column_spec_accepts_the_common_forms() {
    body("\\begin{tabular}{p{2cm}|m{1cm}@{}X*{2}{c}}a&b&c&d&e\\\\\\end{tabular}");
    body("\\begin{tabular}{q}a\\end{tabular}");
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
    let s = format!(
        "\\documentclass[twocolumn]{{article}}\\begin{{document}}a\\begin{{figure*}}[t]\\centering x\\caption{{C}}\\end{{figure*}}\n{}\\end{{document}}",
        words(1800)
    );
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
    let s = format!(
        "\\documentclass[twocolumn]{{article}}\\begin{{document}}\\begin{{figure*}}\\caption{{{}}}\\end{{figure*}}a\\newpage b\\end{{document}}",
        words(120)
    );
    let d = good(&s);
    let last = d.pages - 1;
    assert!(d.right_edge(last) - d.left_edge(last) > 30000);
}

#[test]
fn text_flows_below_a_wide_float() {
    let s = format!(
        "\\documentclass[twocolumn]{{article}}\\begin{{document}}a\\begin{{figure*}}x\\end{{figure*}}\n{}\\end{{document}}",
        words(1800)
    );
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
    let s = format!(
        "{}\n\n\\begin{{figure}}\\begin{{tikzpicture}}\\draw (0,0) -- (0,20);{tall}\\end{{tikzpicture}}\\end{{figure}}",
        words(700)
    );
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

fn numbered_table(name: &str, count: usize) -> String {
    let rows: String = (0..count).map(|i| format!("Row{i:03}\\\\\n")).collect();
    format!("\\begin{{{name}}}{{|l|}}\\hline {rows}\\hline\\end{{{name}}}")
}

fn assert_numbered_rows(d: &mut Done, count: usize) {
    let mut actual = Vec::new();
    for page in 0..d.pages {
        for baseline in d.baselines(page) {
            let mut glyphs: Vec<_> = d.items(page).iter().filter(|i| i.kind == IT_GLYPH && i.y == baseline).collect();
            glyphs.sort_by_key(|i| i.x);
            actual.push(glyphs.iter().map(|i| i.a).collect::<Vec<_>>());
        }
        // Exercise serialized page output as well as the layout item ranges.
        let svg = d.page(page);
        well_formed(&svg).unwrap();
        assert!(svg.contains("<path"));
        assert!(d.items(page).iter().filter(|i| i.kind == IT_GLYPH).all(|i| i.y > 0 && i.y < d.laid().h));
    }
    let expected: Vec<Vec<_>> =
        (0..count).map(|i| body(&format!("Row{i:03}")).all_items().iter().filter(|i| i.kind == IT_GLYPH).map(|i| i.a).collect()).collect();
    assert_eq!(actual, expected, "every row must appear once, in order, inside the page");
}

#[test]
fn longtable_keeps_all_ninety_rows_across_pages() {
    let mut d = body(&numbered_table("longtable", 90));
    assert!(d.pages >= 2);
    assert!(d.e.warned.is_empty());
    assert_numbered_rows(&mut d, 90);
    for page in 0..d.pages {
        assert_eq!(d.items(page).iter().filter(|i| i.kind == IT_RULE && i.b > 1000).count(), 2, "vertical borders on page {page}");
    }
}

#[test]
fn oversized_tabular_preserves_rows_and_warns() {
    let mut d = body(&numbered_table("tabular", 90));
    assert!(d.pages >= 2);
    assert!(!d.e.warned.is_empty());
    assert_numbered_rows(&mut d, 90);
}

#[test]
fn longtable_continues_after_text_on_a_partial_page() {
    let table = numbered_table("longtable", 90);
    let mut d = body(&format!("{}\n\n{table}\nAfter", words(100)));
    assert!(d.pages >= 2);
    let expected = body("After").all_items().iter().filter(|i| i.kind == IT_GLYPH).map(|i| i.a).collect::<Vec<_>>();
    let last = d.baselines(d.pages - 1).last().copied().unwrap();
    let actual: Vec<_> = d.items(d.pages - 1).iter().filter(|i| i.kind == IT_GLYPH && i.y == last).map(|i| i.a).collect();
    assert_eq!(actual, expected);
    let mut rows = Vec::new();
    for page in 0..d.pages {
        for y in d.baselines(page) {
            let glyphs = d.items(page).iter().filter(|i| i.kind == IT_GLYPH && i.y == y).map(|i| i.a).collect::<Vec<_>>();
            if glyphs.len() == 6 {
                rows.push(glyphs);
            }
        }
        well_formed(&d.page(page)).unwrap();
    }
    let expected: Vec<Vec<_>> =
        (0..90).map(|i| body(&format!("Row{i:03}")).all_items().iter().filter(|i| i.kind == IT_GLYPH).map(|i| i.a).collect()).collect();
    assert_eq!(rows, expected);
}

#[test]
fn unsupported_longtable_repeated_heads_warn_without_losing_rows() {
    let d = body(r"\begin{longtable}{l}Header\\\endhead First\\Second\\\end{longtable}");
    assert!(!d.e.warned.is_empty());
    assert_eq!(d.glyphs(), "HeaderFirstSecond".len());
    assert!(body(r"\begin{longtable}{l}\toprule First\\\midrule Second\\\bottomrule\end{longtable}").e.warned.is_empty());
}

#[test]
fn an_unbreakable_table_row_taller_than_a_page_warns() {
    let text = words(500);
    let d = body(&format!("\\begin{{longtable}}{{p{{1cm}}}}{text}\\\\Last\\\\\\end{{longtable}}"));
    assert!(!d.e.warned.is_empty());
    assert!(d.pages >= 2);
    assert_eq!(d.glyphs(), text.chars().filter(|ch| !ch.is_whitespace()).count() + "Last".len());
}

fn has_letter(d: &Done, page: usize, ch: u8) -> bool {
    d.items(page).iter().any(|it| it.kind == IT_GLYPH && it.b == ch as i32)
}

#[test]
fn float_count_limits_defer_content_without_losing_it() {
    for (counter, placement) in [("topnumber", "t"), ("bottomnumber", "b"), ("totalnumber", "h")] {
        let source = format!(r"\setcounter{{{counter}}}{{0}}\begin{{figure}}[{placement}]Z\end{{figure}}Body");
        let d = body(&source);
        assert!(!has_letter(&d, 0, b'Z'), "{counter}");
        assert!(has_letter(&d, 0, b'B'));
        assert!((1..d.pages).any(|p| has_letter(&d, p, b'Z')));
        assert_eq!(d.glyphs(), 5);
        let override_source = source.replace(&format!("[{placement}]"), &format!("[!{placement}]"));
        assert!(has_letter(&body(&override_source), 0, b'Z'), "{counter} override");
    }
}

#[test]
fn top_float_limit_resets_on_new_text_pages() {
    let d = body(r"\setcounter{topnumber}{1}Body\begin{figure}[t]X\end{figure}\begin{figure}[t]Y\end{figure}\newpage Second\newpage Third");
    assert!(has_letter(&d, 1, b'X'));
    assert!(!has_letter(&d, 1, b'Y'));
    assert!(has_letter(&d, 2, b'Y'));
}

#[test]
fn wide_float_count_limits_and_zero_limit_flush_terminate() {
    let d = good(r"\documentclass[twocolumn]{article}\begin{document}\setcounter{dbltopnumber}{0}A\begin{figure*}[t]Z\end{figure*}\newpage B\newpage C\end{document}");
    assert!(!has_letter(&d, 1, b'Z'));
    assert!(has_letter(&d, d.pages - 1, b'Z'));
}

#[test]
fn table_cell_expansions_restore_the_outer_source_reader() {
    let d = body(r"\setcounter{table}{7}\begin{longtable}{lll}A&B&C\\\multicolumn{3}{c}{{\bfseries Table \thetable{} continued}}\\D&E&F\\\end{longtable}After");
    assert!(has_letter(&d, 0, b'D'));
    assert!(has_letter(&d, 0, b'F'));
    assert!(has_letter(&d, 0, b'7'));
    assert_eq!(d.glyphs(), "ABCTable7continuedDEFAfter".len());
}

#[test]
fn forced_here_floats_do_not_consume_float_count_limits() {
    let d = body(r"\setcounter{totalnumber}{0}\begin{figure}[H]Z\end{figure}Body");
    assert_eq!(d.pages, 1);
    assert!(has_letter(&d, 0, b'Z'));
}
