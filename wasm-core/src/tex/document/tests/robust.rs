use super::*;

const TOKENS: &[&str] = &["{", "}", "$", "\\\\", "&", "%", "\\begin{figure}", "\\end{figure}", "\\begin{tabular}{c}", "\\end{tabular}", "\\begin{tikzpicture}", "\\end{tikzpicture}", "\\begin{axis}", "\\end{axis}", "\\draw", "\\node", "\\addplot", "--", "(", ")", "[", "]", ";", ",", "=", "\\caption{", "\\section{", "\\newpage", "\\item", "\\begin{itemize}", "\\end{itemize}", "\\usepackage{", "scale=0", "scale=1e9", "1e999", "99999999", "-99999999", "rotate=", "arc (", "circle (", "domain=", "samples=9999999", "xmin=1,xmax=0", "\\begin{scope}", "\\end{scope}", "é", "日", "\0", "\n\n"];

fn survive(s: &str, e: &mut Engine) -> bool {
    match run(e, s) {
        Ok(pages) => {
            assert!(pages >= 1);
            for p in 0..pages {
                let n = e.document_page(p);
                assert!(n > 0);
                well_formed(&output(e, n as usize)).unwrap_or_else(|why| panic!("{why}\n{s:?}"));
            }
            let size = e.document_pdf() as usize;
            let bytes = unsafe { core::slice::from_raw_parts(e.output_ptr(), size) };
            assert!(bytes.starts_with(b"%PDF-1.4"));
            true
        }
        Err((message, at)) => {
            assert!(!message.is_empty());
            assert!(at <= s.len(), "position {at} of {} in {s:?}", s.len());
            false
        }
    }
}

fn fuzz(base: &str, seed: u64, rounds: usize) {
    let mut rng = Rng(seed);
    let mut e = Engine::new();
    let mut good = 0;
    for _ in 0..rounds {
        let s = mutate(&mut rng, base, TOKENS);
        good += survive(&s, &mut e) as usize;
    }
    assert!(good > 0);
}

#[test]
fn mutated_lab_documents_never_panic() {
    fuzz(LAB, 0x51ab, 1500);
}

#[test]
fn mutated_shape_documents_never_panic() {
    fuzz(SHAPES, 0x5a9e, 3000);
}

#[test]
fn mutated_sample_documents_never_panic() {
    fuzz(SAMPLE, 0x1234, 1500);
}

#[test]
fn random_drawing_statements_never_panic() {
    let mut rng = Rng(0xfeed);
    let mut e = Engine::new();
    for _ in 0..4000 {
        let body: String = (0..rng.below(14)).map(|_| TOKENS[rng.below(TOKENS.len())]).collect();
        let s = with("\\usepackage{pgfplots}", &format!("\\begin{{tikzpicture}}{body}\\end{{tikzpicture}}"));
        survive(&s, &mut e);
    }
}

#[test]
fn every_prefix_of_the_shapes_document_is_handled() {
    let mut e = Engine::new();
    for at in 0..=SHAPES.len() {
        if SHAPES.is_char_boundary(at) {
            survive(&SHAPES[..at], &mut e);
        }
    }
}

#[test]
fn every_deletion_from_the_shapes_document_is_handled() {
    let mut e = Engine::new();
    for at in 0..SHAPES.len() - 1 {
        if SHAPES.is_char_boundary(at) && SHAPES.is_char_boundary(at + 1) {
            survive(&format!("{}{}", &SHAPES[..at], &SHAPES[at + 1..]), &mut e);
        }
    }
}

#[test]
fn extreme_drawing_numbers_stay_finite() {
    let mut e = Engine::new();
    for line in ["\\draw (0,0) -- (1e9,1e9);", "\\draw (0,0) -- (1e300,0);", "\\draw[scale=1e9] (0,0) -- (1,1);", "\\draw[line width=1e9pt] (0,0) -- (1,0);", "\\draw (0,0) circle (1e9);", "\\draw (0,0) arc (0:1e9:1);", "\\draw[rotate=1e9] (0,0) -- (1,0);", "\\draw[scale=0] (0,0) -- (1,1);", "\\draw (0,0) ellipse (0 and 0);", "\\draw (0,0) -- ({1/0},0);", "\\draw (0,0) -- ({ln(0)},0);", "\\draw (0,0) -- ({sqrt(-1)},0);"] {
        let s = with("\\usepackage{tikz}", &format!("\\begin{{tikzpicture}}{line}\\end{{tikzpicture}}"));
        survive(&s, &mut e);
    }
}

#[test]
fn extreme_plot_settings_stay_finite() {
    let mut e = Engine::new();
    for options in ["xmin=1,xmax=0", "xmin=0,xmax=0", "ymin=1e300,ymax=1e301", "xmin=-1e300,xmax=1e300", "width=0cm", "height=0cm", "width=1e9cm", "width=-5cm"] {
        let s = with("\\usepackage{pgfplots}", &format!("\\begin{{tikzpicture}}\\begin{{axis}}[{options}]\\addplot coordinates {{(0,0) (1,1)}};\\end{{axis}}\\end{{tikzpicture}}"));
        survive(&s, &mut e);
    }
}

#[test]
fn huge_sample_counts_are_bounded() {
    let d = good(&with("\\usepackage{pgfplots}", "\\begin{tikzpicture}\\begin{axis}\\addplot[domain=0:1,samples=99999999] {x};\\end{axis}\\end{tikzpicture}"));
    assert!(d.laid().segs.len() < 5000);
}

#[test]
fn deeply_nested_scopes_stop_or_render() {
    let mut e = Engine::new();
    let open = "\\begin{scope}".repeat(5000);
    let shut = "\\end{scope}".repeat(5000);
    let s = with("\\usepackage{tikz}", &format!("\\begin{{tikzpicture}}{open}\\draw (0,0) -- (1,0);{shut}\\end{{tikzpicture}}"));
    survive(&s, &mut e);
}

#[test]
fn deeply_nested_lists_stop_or_render() {
    let mut e = Engine::new();
    let open = "\\begin{itemize}\\item ".repeat(3000);
    let shut = "\\end{itemize}".repeat(3000);
    survive(&article(&format!("{open}x{shut}")), &mut e);
}

#[test]
fn long_documents_compile() {
    let d = body(&words(200000));
    assert!(d.pages > 100);
}

#[test]
fn a_word_wider_than_the_page_stays_finite() {
    let d = body(&"x".repeat(5000));
    assert_eq!(d.glyphs(), 5000);
}

#[test]
fn a_table_wider_than_the_page_renders() {
    let row = "wide&".repeat(29) + "end";
    let s = format!("\\begin{{tabular}}{{{}}}{row}\\end{{tabular}}", "c".repeat(30));
    body(&s);
}

#[test]
fn tables_stop_at_thirty_two_columns() {
    body(&format!("\\begin{{tabular}}{{{}}}a\\end{{tabular}}", "c".repeat(32)));
    assert!(compile(&article(&format!("\\begin{{tabular}}{{{}}}a\\end{{tabular}}", "c".repeat(33)))).is_err());
}

#[test]
fn a_float_taller_than_the_page_renders() {
    good(&with("\\usepackage{tikz}", "\\begin{figure}\\begin{tikzpicture}\\draw (0,0) -- (0,100);\\end{tikzpicture}\\end{figure}"));
}

#[test]
fn errors_point_at_the_right_line() {
    let s = article("one\n\ntwo\n\n\\foo\n\nthree");
    let (message, at) = compile(&s).err().unwrap();
    assert_eq!(message, "unknown command");
    assert_eq!(&s[at..at + 4], "\\foo");
}

#[test]
fn mismatched_environments_are_errors() {
    for s in ["\\begin{center}x\\end{quote}", "\\begin{center}x", "\\end{center}", "\\begin{nope}x\\end{nope}"] {
        assert!(compile(&article(s)).is_err(), "{s}");
    }
}

#[test]
fn unclosed_math_is_an_error() {
    for s in ["$x", "\\[x", "\\(x", "\\begin{equation}x", "\\begin{align}x"] {
        assert!(compile(&article(s)).is_err(), "{s}");
    }
}
