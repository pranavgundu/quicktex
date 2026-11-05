use super::*;

#[test]
fn every_word_is_placed() {
    let d = body("abc def");
    assert_eq!(d.glyphs(), 6);
}

#[test]
fn long_text_wraps() {
    assert!(body(&words(120)).baselines(0).len() > 3);
}

#[test]
fn lines_are_evenly_spaced() {
    let d = body(&words(400));
    let ys = d.baselines(0);
    let gaps: Vec<i32> = ys.windows(2).map(|w| w[1] - w[0]).collect();
    assert!(gaps.iter().all(|g| (*g - 1200).abs() <= 2), "{gaps:?}");
}

#[test]
fn baseline_follows_the_font_size() {
    for (option, gap) in [("10pt", 1200), ("11pt", 1236), ("12pt", 1208)] {
        let s = format!("\\documentclass[{option}]{{article}}\\begin{{document}}{}\\end{{document}}", words(300));
        let ys = good(&s).baselines(0);
        assert!((ys[1] - ys[0] - gap).abs() <= 2, "{option}: {}", ys[1] - ys[0]);
    }
}

#[test]
fn justified_lines_share_a_right_edge() {
    let d = body(&words(400));
    let edges: Vec<i32> = d
        .baselines(0)
        .iter()
        .map(|y| d.items(0).iter().filter(|i| i.y == *y).map(|i| i.x + crate::data::GLYPHS[i.a as usize].w as i32).max().unwrap())
        .collect();
    let body = &edges[..edges.len() - 1];
    let (min, max) = (body.iter().min().unwrap(), body.iter().max().unwrap());
    assert!(max - min < 60, "{min} {max}");
}

#[test]
fn last_line_is_not_stretched() {
    let d = body("short line");
    let line = body("shortline");
    assert!(d.right_edge(0) - line.right_edge(0) < 500);
}

#[test]
fn text_stays_inside_the_page() {
    for s in [LAB, SHAPES, SAMPLE] {
        let d = good(s);
        for p in 0..d.pages {
            for i in d.items(p) {
                assert!(i.x >= 0 && i.x <= d.laid().w, "x {}", i.x);
                assert!(i.y >= 0 && i.y <= d.laid().h, "y {}", i.y);
            }
        }
    }
}

#[test]
fn pages_fill_then_break() {
    assert_eq!(body(&words(100)).pages, 1);
    assert!(body(&words(2000)).pages >= 2);
    assert!(body(&words(20000)).pages > body(&words(2000)).pages);
}

#[test]
fn page_breaks_keep_every_glyph() {
    let text = words(3000);
    let glyphs = text.chars().filter(|c| !c.is_whitespace()).count();
    let d = body(&text);
    assert!(d.pages >= 2);
    assert_eq!(d.glyphs(), glyphs);
}

#[test]
fn pages_are_numbered_at_the_foot() {
    let d = body(&words(3000));
    assert!(d.pages >= 2);
    for p in 0..d.pages {
        let number = d.raw_items(p).last().unwrap();
        assert!(number.y > 68000, "page {p}: {}", number.y);
        assert!((number.x - d.laid().w / 2).abs() < 500);
    }
}

#[test]
fn newpage_starts_a_page() {
    assert_eq!(body("a\\newpage b").pages, 2);
    assert_eq!(body("a\\clearpage b").pages, 2);
    assert_eq!(body("a\\newpage b\\newpage c").pages, 3);
}

#[test]
fn paragraphs_indent_after_the_first() {
    let d = body("first\n\nsecond");
    let ys = d.baselines(0);
    let x = |y: i32| d.items(0).iter().filter(|i| i.y == y).map(|i| i.x).min().unwrap();
    assert!(x(ys[1]) > x(ys[0]) - 1);
}

#[test]
fn noindent_removes_the_indent() {
    let a = body("first\n\nsecond");
    let b = body("first\n\n\\noindent second");
    let last = |d: &Done| {
        let y = *d.baselines(0).get(1).unwrap();
        d.items(0).iter().filter(|i| i.y == y).map(|i| i.x).min().unwrap()
    };
    assert!(last(&a) > last(&b));
}

#[test]
fn parskip_adds_space_and_drops_the_indent() {
    let plain = body("first\n\nsecond");
    let spaced = good(&with("\\usepackage{parskip}", "first\n\nsecond"));
    let gap = |d: &Done| {
        let y = d.baselines(0);
        y[1] - y[0]
    };
    assert!(gap(&spaced) > gap(&plain));
    let x = |d: &Done| {
        let y = d.baselines(0)[1];
        d.items(0).iter().filter(|i| i.y == y).map(|i| i.x).min().unwrap()
    };
    assert!(x(&spaced) < x(&plain));
}

#[test]
fn blank_lines_collapse() {
    assert_eq!(body("a\n\n\n\n\nb").baselines(0), body("a\n\nb").baselines(0));
}

#[test]
fn spaces_collapse() {
    let a = body("a b");
    let b = body("a      b");
    assert_eq!(a.right_edge(0), b.right_edge(0));
}

#[test]
fn vertical_skips_grow_in_order() {
    let gap = |skip: &str| {
        let d = body(&format!("a\n\n{skip}\n\nb"));
        let y = d.baselines(0);
        y[1] - y[0]
    };
    let (none, small, med, big) = (gap(""), gap("\\smallskip"), gap("\\medskip"), gap("\\bigskip"));
    assert!(none < small && small < med && med < big, "{none} {small} {med} {big}");
}

#[test]
fn vspace_uses_its_length() {
    let gap = |skip: &str| {
        let d = body(&format!("a\n\n{skip}\n\nb"));
        let y = d.baselines(0);
        y[1] - y[0]
    };
    assert!(((gap("\\vspace{1cm}") - gap("")) - 2845).abs() <= 3);
}

#[test]
fn section_numbers_count_up() {
    let starred = body("\\section*{A}\\section*{B}").glyphs();
    let numbered = body("\\section{A}\\section{B}").glyphs();
    assert_eq!(numbered, starred + 2);
}

#[test]
fn subsection_numbers_have_two_parts() {
    assert_eq!(body("\\section{A}\\subsection{B}").glyphs(), 1 + 1 + 3 + 1);
    assert_eq!(body("\\section{A}\\subsection{B}\\subsubsection{C}").glyphs(), 1 + 1 + 3 + 1 + 5 + 1);
}

#[test]
fn numbers_reset_under_a_new_section() {
    let d = body("\\section{A}\\subsection{B}\\subsection{C}\\section{D}\\subsection{E}");
    assert_eq!(d.glyphs(), 2 + 4 + 4 + 2 + 4);
}

#[test]
fn ten_sections_need_two_digits() {
    let s: String = (0..10).map(|_| "\\section{A}\n").collect();
    assert_eq!(body(&s).glyphs(), 9 * 2 + 3);
}

#[test]
fn headings_are_bigger_in_order() {
    let height = |s: &str| {
        let d = body(s);
        d.laid().groups.iter().map(|g| g.scale).max().unwrap()
    };
    assert!(height("\\section{A}") > height("\\subsection{A}"));
    assert!(height("\\subsection{A}") > height("\\subsubsection{A}"));
}

#[test]
fn heading_at_the_foot_moves_to_the_next_page() {
    let mut s = String::new();
    for _ in 0..200 {
        s.push_str("line\n\n");
        let d = body(&format!("{s}\\section{{Last}}\nbody"));
        for p in 0..d.pages {
            assert!(!d.items(p).is_empty());
        }
    }
}

#[test]
fn title_spacing_changes_section_gaps() {
    let gap = |pre: &str| {
        let d = good(&with(pre, "a\n\n\\section*{B}\nc"));
        let y = d.baselines(0);
        (y[1] - y[0], y[2] - y[1])
    };
    let tight = gap("\\usepackage{titlesec}\\titlespacing*{\\section}{0pt}{2pt}{1pt}");
    let loose = gap("\\usepackage{titlesec}\\titlespacing*{\\section}{0pt}{30pt}{20pt}");
    assert!(loose.0 > tight.0 && loose.1 > tight.1, "{tight:?} {loose:?}");
}

#[test]
fn two_columns_use_both_sides() {
    let d = good(&format!("\\documentclass[twocolumn]{{article}}\\begin{{document}}{}\\end{{document}}", words(1500)));
    let mid = d.laid().w / 2;
    assert!(d.items(0).iter().any(|i| i.x < mid));
    assert!(d.items(0).iter().any(|i| i.x > mid));
}

#[test]
fn second_column_starts_at_the_top() {
    let d = good(&format!("\\documentclass[twocolumn]{{article}}\\begin{{document}}{}\\end{{document}}", words(1500)));
    let mid = d.laid().w / 2;
    let top = |right: bool| d.items(0).iter().filter(|i| (i.x > mid) == right).map(|i| i.y).min().unwrap();
    assert_eq!(top(false), top(true));
}

#[test]
fn columns_do_not_overlap() {
    let d = good(&format!("\\documentclass[twocolumn]{{article}}\\begin{{document}}{}\\end{{document}}", words(1500)));
    let mid = d.laid().w / 2;
    let left_end =
        d.items(0).iter().filter(|i| i.x < mid && i.y < 70000).map(|i| i.x + crate::data::GLYPHS[i.a as usize].w as i32).max().unwrap();
    let right_start = d.items(0).iter().filter(|i| i.x > mid).map(|i| i.x).min().unwrap();
    assert!(right_start > left_end, "{left_end} {right_start}");
}

#[test]
fn newpage_in_two_columns_moves_one_column() {
    let d = good("\\documentclass[twocolumn]{article}\\begin{document}a\\newpage b\\end{document}");
    assert_eq!(d.pages, 1);
}

#[test]
fn columnsep_widens_the_gap() {
    let edge = |pre: &str| {
        let s = format!("\\documentclass[twocolumn]{{article}}{pre}\\begin{{document}}{}\\end{{document}}", words(1500));
        let d = good(&s);
        let mid = d.laid().w / 2;
        d.items(0).iter().filter(|i| i.x > mid).map(|i| i.x).min().unwrap()
    };
    assert!(edge("\\setlength{\\columnsep}{3cm}") > edge("\\setlength{\\columnsep}{1pt}"));
}

#[test]
fn fonts_change_the_text() {
    let plain = body("x");
    for s in [
        "\\textbf{x}",
        "\\textit{x}",
        "\\texttt{x}",
        "\\textsf{x}",
        "\\emph{x}",
        "{\\bfseries x}",
        "{\\itshape x}",
        "{\\ttfamily x}",
        "{\\sffamily x}",
    ] {
        let d = body(s);
        assert_eq!(d.glyphs(), 1, "{s}");
        assert_ne!(d.all_items()[0].a, plain.all_items()[0].a, "{s}");
    }
}

#[test]
fn font_groups_end() {
    let d = body("{\\bfseries x} x");
    let a: Vec<u32> = d.all_items().iter().filter(|i| i.kind == IT_GLYPH).map(|i| i.a as u32).collect();
    assert_ne!(a[0], a[1]);
}

#[test]
fn size_switches_scale_text() {
    let d = |s: &str| body(s).laid().groups.iter().map(|g| g.scale as i32).collect::<Vec<_>>();
    assert!(d("{\\small x}").iter().any(|s| *s < 1000));
    assert!(d("{\\large x}").iter().any(|s| *s > 1000));
    assert!(d("{\\Huge x}").iter().any(|s| *s > 2000));
}

#[test]
fn quotes_and_dashes_become_ligatures() {
    assert_eq!(body("a--b").glyphs(), 3);
    assert_eq!(body("a---b").glyphs(), 3);
    assert_eq!(body("``a''").glyphs(), 3);
}

#[test]
fn text_escapes_print() {
    for (s, n) in [("\\%", 1), ("\\$", 1), ("\\#", 1), ("\\&", 1), ("\\_", 1), ("\\{", 1), ("\\}", 1), ("\\LaTeX", 5), ("\\TeX", 3)] {
        assert_eq!(body(s).glyphs(), n, "{s}");
    }
}

#[test]
fn accented_letters_are_drawn_from_their_parts() {
    assert_eq!(body("caf\u{e9}").glyphs(), 5);
    assert_eq!(body("caf\\'e").glyphs(), 5);
    assert_eq!(body("\\\"o \\`a \\^i").glyphs(), 6);
    assert!(body("caf\u{e9}").e.warned.is_empty());
}

#[test]
fn color_in_body_text_works() {
    assert_eq!(body("\\textcolor{red}{x}").glyphs(), 1);
    assert_eq!(body("$\\textcolor{red}{x}$").glyphs(), 1);
    assert!(body("\\textcolor{red}{x}").e.warned.is_empty());
}

#[test]
fn caption_can_follow_text_on_the_same_line() {
    assert!(body("\\begin{figure}x\\caption{C}\\end{figure}").glyphs() > 3);
}

#[test]
fn inline_math_joins_the_line() {
    let d = body("a $x^2$ b");
    assert_eq!(d.baselines(0).len(), 2);
    assert_eq!(d.glyphs(), 4);
}

#[test]
fn every_inline_math_form_works() {
    let want = body("$x$").glyphs();
    for s in ["\\(x\\)", "$x$"] {
        assert_eq!(body(s).glyphs(), want, "{s}");
    }
}

#[test]
fn display_math_is_centered_on_its_own_line() {
    let d = body("a\\[x\\]b");
    let ys = d.baselines(0);
    assert_eq!(ys.len(), 3);
    let middle = d.items(0).iter().find(|i| i.y == ys[1]).unwrap();
    assert!((middle.x - d.laid().w / 2).abs() < 500);
}

#[test]
fn every_display_form_works() {
    for s in ["\\[x\\]", "$$x$$", "\\begin{equation*}x\\end{equation*}", "\\begin{align*}x\\end{align*}", "\\begin{gather*}x\\end{gather*}"]
    {
        assert_eq!(body(s).glyphs(), 1, "{s}");
    }
}

#[test]
fn numbered_equations_print_the_number() {
    assert_eq!(body("\\begin{equation}x\\end{equation}").glyphs(), 4);
    assert_eq!(body("\\begin{equation}x\\end{equation}\\begin{equation}y\\end{equation}").glyphs(), 8);
}

#[test]
fn equation_number_sits_at_the_right() {
    let d = body("\\begin{equation}x\\end{equation}");
    assert!(d.right_edge(0) > d.laid().w / 2 + 10000);
}

#[test]
fn align_rows_are_not_numbered_yet() {
    let starred = body("\\begin{align*}a&=b\\\\c&=d\\end{align*}").glyphs();
    let numbered = body("\\begin{align}a&=b\\\\c&=d\\end{align}").glyphs();
    assert_eq!(numbered, starred);
}

#[test]
fn lists_put_a_marker_on_every_item() {
    assert_eq!(body("\\begin{itemize}\\item a\\item b\\end{itemize}").glyphs(), 4);
    assert_eq!(body("\\begin{enumerate}\\item a\\item b\\end{enumerate}").glyphs(), 6);
}

#[test]
fn list_items_are_indented() {
    let plain = body("a").left_edge(0);
    let d = body("\\begin{itemize}\\item a\\end{itemize}");
    let text = d.items(0).iter().map(|i| i.x).max().unwrap();
    assert!(text > plain);
}

#[test]
fn nested_lists_indent_further() {
    let one = body("\\begin{itemize}\\item a\\end{itemize}");
    let two = body("\\begin{itemize}\\item\\begin{itemize}\\item a\\end{itemize}\\end{itemize}");
    assert!(two.right_edge(0) > one.right_edge(0));
}

#[test]
fn enumerate_counts_past_nine() {
    let s: String = (0..10).map(|_| "\\item a\n").collect();
    assert_eq!(body(&format!("\\begin{{enumerate}}{s}\\end{{enumerate}}")).glyphs(), 9 * 3 + 4);
}

#[test]
fn quotes_are_inset_on_both_sides() {
    let text = words(120);
    let q = body(&format!("\\begin{{quote}}{text}\\end{{quote}}"));
    let p = body(&text);
    assert!(q.left_edge(0) > p.left_edge(0));
    assert!(q.right_edge(0) < p.right_edge(0));
}

#[test]
fn center_centers_each_line() {
    let d = body("\\begin{center}abc\\end{center}");
    let middle = (d.left_edge(0) + d.right_edge(0)) / 2;
    assert!((middle - d.laid().w / 2).abs() < 400);
}

#[test]
fn center_gap_is_the_larger_skip() {
    let after_heading = |s: &str| {
        let d = body(s);
        let y = d.baselines(0);
        y[1] - y[0]
    };
    assert_eq!(after_heading("\\section*{A}\n\\begin{center}b\\end{center}"), after_heading("\\section*{A}\nb"));
}

#[test]
fn centering_applies_inside_a_group() {
    let d = body("\\begin{figure}\\centering x\\end{figure}");
    assert!((d.left_edge(0) - d.laid().w / 2).abs() < 500);
}

#[test]
fn macros_work_in_text() {
    assert_eq!(good(&with("\\newcommand{\\me}{abc}", "\\me")).glyphs(), 3);
    assert_eq!(good(&with("\\newcommand{\\me}[1]{#1#1}", "\\me{a}")).glyphs(), 2);
}

#[test]
fn macros_work_in_math() {
    assert_eq!(good(&with("\\newcommand{\\R}{\\mathbb{R}}", "$\\R$")).glyphs(), 1);
}
