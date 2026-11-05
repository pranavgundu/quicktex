use super::*;

#[test]
fn default_page_is_us_letter_at_ten_points() {
    let d = body("x");
    assert_eq!(d.laid().pt, 10);
    assert_eq!(d.laid().w, 61200);
    assert_eq!(d.laid().h, 79200);
}

#[test]
fn font_size_options_set_the_unit() {
    for (option, pt) in [("10pt", 10), ("11pt", 11), ("12pt", 12)] {
        let d = good(&format!("\\documentclass[{option}]{{article}}\\begin{{document}}x\\end{{document}}"));
        assert_eq!(d.laid().pt, pt, "{option}");
        assert_eq!(d.laid().w, 612 * 1000 / pt, "{option}");
    }
}

#[test]
fn a4_changes_the_page() {
    let d = good("\\documentclass[a4paper]{article}\\begin{document}x\\end{document}");
    assert_eq!(d.laid().w, 59500);
    assert_eq!(d.laid().h, 84200);
}

#[test]
fn options_combine() {
    let d = good("\\documentclass[12pt,a4paper,twocolumn]{article}\\begin{document}x\\end{document}");
    assert_eq!(d.laid().pt, 12);
    assert_eq!(d.laid().w, 595 * 1000 / 12);
}

#[test]
fn page_frames_match_the_article_class() {
    for (options, top, height) in [
        ("10pt", 125, 550),
        ("11pt", 130, 541),
        ("12pt", 126, 548),
        ("10pt,a4paper", 126, 598),
        ("11pt,a4paper", 127, 596),
        ("12pt,a4paper", 129, 592),
    ] {
        let mut e = Engine::new();
        let s = format!("\\documentclass[{options}]{{article}}\\begin{{document}}x\\end{{document}}");
        run(&mut e, &s).unwrap();
        let pt = e.laid.pt;
        let d = Done { e, pages: 1 };
        let first = d.top_edge(0);
        let want = top * 1000 / pt + 1000;
        assert!((first - want).abs() <= 150, "{options}: {first} vs {want}");
        let _ = height;
    }
}

#[test]
fn margins_move_the_text() {
    let cases = [
        ("margin=1in", 7227),
        ("margin=2cm", 5690),
        ("left=3cm", 8535),
        ("margin=72.27pt", 7227),
        ("hmargin=1in", 7227),
        ("lmargin=1in", 7227),
    ];
    for (option, left) in cases {
        let d = good(&with(&format!("\\usepackage[{option}]{{geometry}}"), "\\noindent x"));
        assert!((d.left_edge(0) - left).abs() <= 120, "{option}: {}", d.left_edge(0));
    }
}

#[test]
fn top_margin_moves_the_first_line() {
    let near = good(&with("\\usepackage[top=1cm]{geometry}", "x")).top_edge(0);
    let far = good(&with("\\usepackage[top=4cm]{geometry}", "x")).top_edge(0);
    assert!((far - near - 3 * 2845).abs() <= 20, "{near} {far}");
}

#[test]
fn geometry_command_works_like_the_option() {
    let a = good(&with("\\usepackage[margin=1in]{geometry}", "x")).left_edge(0);
    let b = good(&with("\\usepackage{geometry}\\geometry{margin=1in}", "x")).left_edge(0);
    assert_eq!(a, b);
}

#[test]
fn wide_margins_make_more_lines() {
    let text = words(200);
    let wide = good(&with("\\usepackage[margin=1cm]{geometry}", &text));
    let narrow = good(&with("\\usepackage[margin=6cm]{geometry}", &text));
    assert!(narrow.baselines(0).len() > wide.baselines(0).len());
}

#[test]
fn ignored_setup_commands_print_nothing() {
    let plain = body("x").glyphs();
    for line in [
        "\\usepackage{amsmath}",
        "\\usepackage{booktabs}",
        "\\usepackage{graphicx}",
        "\\usepackage{titlesec}",
        "\\usepackage{float}",
        "\\usepackage{caption}",
        "\\usepackage{hyperref}",
        "\\usepackage[utf8]{inputenc}",
        "\\usepackage[T1]{fontenc}",
        "\\pagestyle{empty}",
        "\\setlength{\\columnsep}{1cm}",
        "\\setlength{\\intextsep}{8pt}",
        "\\setlength{\\textfloatsep}{10pt}",
        "\\setlength{\\parindent}{0pt}",
        "\\setlength{\\parskip}{1em}",
        "\\captionsetup{skip=4pt}",
        "\\titlespacing*{\\section}{0pt}{10pt plus 2pt minus 2pt}{5pt}",
        "\\usetikzlibrary{arrows.meta,calc}",
        "\\pgfplotsset{compat=1.18}",
        "\\raggedbottom",
        "\\sloppy",
        "\\setcounter{page}{3}",
    ] {
        let d = good(&with(line, "x"));
        assert_eq!(d.glyphs(), plain, "{line}");
    }
}

#[test]
fn unknown_packages_load_quietly() {
    assert!(good(&with("\\usepackage{notapackage}", "x")).e.warned.is_empty());
}

#[test]
fn math_packages_work_in_documents() {
    good(&with("\\usepackage{physics}", "$\\ket{x}$"));
    good(&with("\\usepackage{siunitx}", "$\\si{m}$"));
    good(&with("\\usepackage{mhchem}", "$\\ce{H2O}$"));
}

#[test]
fn title_block_prints_all_three_rows() {
    let plain = good(&with("\\title{Abc}", "\\maketitle"));
    let more = good(&with("\\title{Abc}\\author{Def}", "\\maketitle"));
    let most = good(&with("\\title{Abc}\\author{Def}\\date{Ghi}", "\\maketitle"));
    assert_eq!(plain.glyphs(), 3);
    assert_eq!(more.glyphs(), 6);
    assert_eq!(most.glyphs(), 9);
    assert_eq!(most.baselines(0).len(), 3);
}

#[test]
fn title_needs_maketitle() {
    assert_eq!(good(&with("\\title{Abc}", "x")).glyphs(), 1);
}

#[test]
fn title_is_centered() {
    let d = good(&with("\\title{A}", "\\maketitle"));
    let middle = d.laid().w / 2;
    assert!((d.left_edge(0) - middle).abs() < 800);
}

#[test]
fn title_sits_below_the_top_of_the_frame() {
    let titled = good(&with("\\title{A}", "\\maketitle"));
    let plain = body("A");
    assert!(titled.top_edge(0) > plain.top_edge(0) + 2000);
}

#[test]
fn two_column_title_spans_the_page() {
    let d = good("\\documentclass[twocolumn]{article}\\title{A}\\begin{document}\\maketitle\\end{document}");
    assert!((d.left_edge(0) - d.laid().w / 2).abs() < 800);
}

#[test]
fn two_column_text_starts_below_the_title() {
    let s = format!("\\documentclass[twocolumn]{{article}}\\title{{A}}\\begin{{document}}\\maketitle\n{}\\end{{document}}", words(2000));
    let d = good(&s);
    let title = d.top_edge(0);
    let mid = d.laid().w / 2;
    let right_top = d.items(0).iter().filter(|i| i.x > mid + 100).map(|i| i.y).min().unwrap();
    assert!(right_top > title + 2000, "{title} {right_top}");
}

#[test]
fn abstract_is_narrower_than_the_text() {
    let text = words(120);
    let d = body(&format!("\\begin{{abstract}}{text}\\end{{abstract}}"));
    let p = body(&text);
    assert!(d.left_edge(0) > p.left_edge(0));
    assert!(d.right_edge(0) < p.right_edge(0));
}

#[test]
fn abstract_has_a_heading() {
    let d = body("\\begin{abstract}x\\end{abstract}");
    assert!(d.glyphs() > 1);
    assert!(d.baselines(0).len() >= 2);
}

#[test]
fn comments_are_ignored() {
    assert_eq!(body("x%hidden\n").glyphs(), 1);
    assert_eq!(body("% a whole line\nx").glyphs(), 1);
    assert_eq!(good("% before\n\\documentclass{article}\n\\begin{document}\nx\n\\end{document}\n").glyphs(), 1);
}

#[test]
fn text_after_end_document_is_ignored() {
    assert_eq!(good(&(article("x") + "ignored words here")).glyphs(), 1);
}

#[test]
fn empty_document_has_one_page() {
    assert_eq!(body("").pages, 1);
}
