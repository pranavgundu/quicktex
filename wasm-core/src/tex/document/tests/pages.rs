use super::*;

#[test]
fn every_fixture_page_is_well_formed() {
    for s in [LAB, SHAPES, SAMPLE] {
        let mut d = good(s);
        for p in 0..d.pages {
            well_formed(&d.page(p)).unwrap();
        }
    }
}

#[test]
fn fixtures_have_the_expected_page_counts() {
    assert_eq!(good(LAB).pages, 2);
    assert_eq!(good(SHAPES).pages, 1);
}

#[test]
fn header_gives_the_page_size() {
    let mut d = body("x");
    let svg = d.page(0);
    assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 61429 79497\""), "{}", &svg[..100]);
}

#[test]
fn missing_pages_are_empty() {
    let mut d = body("x");
    assert_eq!(d.e.document_page(1), 0);
    assert_eq!(d.e.document_page(99), 0);
    assert_eq!(d.e.document_page(usize::MAX), 0);
}

#[test]
fn pages_split_into_one_element_per_line() {
    for s in [LAB, SHAPES, SAMPLE] {
        let mut d = good(s);
        for p in 0..d.pages {
            let svg = d.page(p);
            let lines: Vec<&str> = svg.split('\n').collect();
            assert!(lines[0].starts_with("<svg "));
            assert_eq!(*lines.last().unwrap(), "</svg>");
            for line in &lines[1..lines.len() - 1] {
                let single = line.starts_with("<path") && line.ends_with("/>") && line.matches("<path").count() == 1;
                let group = line.starts_with("<g ") && line.ends_with("</g>");
                assert!(single || group, "{}", &line[..line.len().min(80)]);
            }
        }
    }
}

#[test]
fn lines_are_split_into_bands() {
    let mut d = body(&words(400));
    let svg = d.page(0);
    assert!(svg.matches("<path").count() > 20);
}

#[test]
fn no_band_is_huge() {
    let mut d = good(LAB);
    let svg = d.page(0);
    assert!(svg.split('\n').all(|line| line.len() < 40000), "{}", svg.split('\n').map(str::len).max().unwrap());
}

#[test]
fn an_edit_only_changes_nearby_bands() {
    let text = format!("{}\n\nedit here\n\n{}", words(100), words(100));
    let mut a = body(&text);
    let mut b = body(&text.replace("edit here", "edit hire"));
    let (x, y) = (a.page(0), b.page(0));
    let old: std::collections::HashSet<&str> = x.split('\n').collect();
    let changed = y.split('\n').filter(|l| !old.contains(l)).count();
    assert!(changed > 0 && changed <= 2, "{changed}");
}

#[test]
fn pages_can_be_read_in_any_order_and_repeatedly() {
    let mut d = body(&words(4000));
    let (a, b) = (d.page(0), d.page(1));
    assert_ne!(a, b);
    assert_eq!(d.page(1), b);
    assert_eq!(d.page(0), a);
    assert_eq!(d.page(1), b);
}

#[test]
fn drawings_come_before_text() {
    let mut d = good(&with(
        "\\usepackage{tikz}",
        "\\begin{tikzpicture}\\fill[white] (0,0) rectangle (1,1);\\node at (0.5,0.5) {x};\\end{tikzpicture}",
    ));
    let svg = d.page(0);
    let lines: Vec<&str> = svg.split('\n').collect();
    assert!(lines[1].contains("fill=\"#ffffff\""), "{}", lines[1]);
}

#[test]
fn drawings_carry_their_style() {
    let mut d =
        good(&with("\\usepackage{tikz}", "\\begin{tikzpicture}\\draw[red,dashed,line cap=round] (0,0) -- (1,0);\\end{tikzpicture}"));
    let svg = d.page(0);
    for want in ["stroke=\"#ff0000\"", "stroke-dasharray=", "stroke-linecap=\"round\"", "fill=\"none\""] {
        assert!(svg.contains(want), "{want}");
    }
}

#[test]
fn turned_text_is_a_matrix_group() {
    let mut d = good(&with("\\usepackage{tikz}", "\\begin{tikzpicture}\\node[rotate=90] at (0,0) {x};\\end{tikzpicture}"));
    assert!(d.page(0).contains("<g transform=\"matrix("));
}

#[test]
fn colored_text_keeps_its_color() {
    let mut d = body("$\\textcolor{red}{x}$");
    assert!(d.page(0).contains("fill=\"red\""));
}

#[test]
fn compiles_are_repeatable() {
    for s in [LAB, SHAPES, SAMPLE] {
        let mut e = Engine::new();
        let pages = run(&mut e, s).unwrap();
        let first: Vec<String> = (0..pages)
            .map(|p| {
                let n = e.document_page(p);
                output(&e, n as usize)
            })
            .collect();
        run(&mut e, s).unwrap();
        let second: Vec<String> = (0..pages)
            .map(|p| {
                let n = e.document_page(p);
                output(&e, n as usize)
            })
            .collect();
        assert_eq!(first, second);
    }
}

#[test]
fn a_failed_compile_keeps_the_last_good_pages() {
    let mut e = Engine::new();
    let pages = run(&mut e, LAB).unwrap();
    let n = e.document_page(0);
    let before = output(&e, n as usize);
    assert!(run(&mut e, &LAB.replace("\\end{document}", "{")).is_err());
    let n = e.document_page(0);
    assert_eq!(output(&e, n as usize), before);
    assert_eq!(e.laid.pages.len(), pages);
}

#[test]
fn a_good_compile_after_a_failure_is_clean() {
    let mut e = Engine::new();
    run(&mut e, LAB).unwrap();
    let n = e.document_page(1);
    let want = output(&e, n as usize);
    for bad in ["\\begin{document}\\begin{figure}", "\\begin{document}$", "\\begin{document}\\begin{tikzpicture}\\draw[wobble] (0,0);"] {
        assert!(run(&mut e, bad).is_err(), "{bad}");
        run(&mut e, LAB).unwrap();
        let n = e.document_page(1);
        assert_eq!(output(&e, n as usize), want, "{bad}");
    }
}

#[test]
fn counters_reset_between_compiles() {
    let mut e = Engine::new();
    let s = article("\\section{A}\\begin{figure}x\\caption{C}\\end{figure}\\begin{equation}x\\end{equation}");
    run(&mut e, &s).unwrap();
    let n = e.document_page(0);
    let first = output(&e, n as usize);
    run(&mut e, &s).unwrap();
    let n = e.document_page(0);
    assert_eq!(output(&e, n as usize), first);
}

#[test]
fn document_and_math_modes_do_not_interfere() {
    let mut e = Engine::new();
    run(&mut e, LAB).unwrap();
    let n = e.document_page(0);
    let want = output(&e, n as usize);
    let a = crate::tex::tests::render_with(&mut e, "\\frac{a}{b}", true).unwrap();
    let n = e.document_page(0);
    assert_eq!(output(&e, n as usize), want);
    assert_eq!(crate::tex::tests::render_with(&mut e, "\\frac{a}{b}", true).unwrap(), a);
}
