use super::*;

const SEEDS: &[&str] = &[
    "\\frac{-b\\pm\\sqrt{b^2-4ac}}{2a}",
    "\\begin{pmatrix}a&b\\\\c&d\\end{pmatrix}",
    "\\int_{-\\infty}^{\\infty}e^{-x^2}\\,dx",
    "\\newcommand{\\f}[2][z]{#1^{#2}}\\f{x}\\f[y]{x}",
    "\\left(\\sum_{i=1}^n\\frac{1}{i}\\right)^2",
    "\\usepackage{physics}\\dv{x}{t}\\ket{\\psi}",
    "\\usepackage{mhchem}\\ce{2H2 + O2 -> 2H2O}",
    "\\usepackage{siunitx}\\SI{1e3}{\\kilo\\metre}",
    "\\begin{cases}x&x>0\\\\-x&x\\le0\\end{cases}",
    "\\begin{array}{|l|c|r|}\\hline a&b&c\\\\\\hline\\end{array}",
    "\\text{a $x^2$ b}\\textcolor{red}{y}\\overbrace{a+b}^{n}",
];

const TOKENS: &[&str] = &["{", "}", "^", "_", "\\", "&", "\\\\", "$", "#", "#1", "%", "~", "\\left", "\\right", "\\begin{matrix}", "\\end{matrix}", "\\frac", "\\sqrt[", "]", "[", "\\def\\a{\\a}", "\\newcommand", "é", "日", "😀", "\0", "\n", "\\text{", "\\color{", "\\hspace{", "\\usepackage{", "\\middle", "\\over", "\\limits", "\\phantom", "\\rule{", "'", "-", "1e999", "99999999999"];

#[test]
fn mutated_formulas_never_panic() {
    let mut rng = Rng(0x9e3779b97f4a7c15);
    let mut e = Engine::new();
    let (mut good, mut bad) = (0, 0);
    for i in 0..60000 {
        let s = mutate(&mut rng, SEEDS[i % SEEDS.len()], TOKENS);
        match render_with(&mut e, &s, i % 2 == 0) {
            Ok(svg) => {
                good += 1;
                well_formed(&svg).unwrap_or_else(|why| panic!("{s:?}: {why}"));
            }
            Err((message, at)) => {
                bad += 1;
                assert!(!message.is_empty(), "{s:?}");
                assert!(at <= s.len(), "{s:?} position {at}");
            }
        }
    }
    assert!(good > 1000 && bad > 1000, "{good} {bad}");
}

#[test]
fn random_bytes_never_panic() {
    let mut rng = Rng(0xdeadbeefcafef00d);
    let mut e = Engine::new();
    for _ in 0..20000 {
        let bytes: Vec<u8> = (0..rng.below(80)).map(|_| rng.next() as u8).collect();
        let s = String::from_utf8_lossy(&bytes).into_owned();
        let _ = render_with(&mut e, &s, false);
    }
}

#[test]
fn random_tokens_never_panic() {
    let mut rng = Rng(0x1234567887654321);
    let mut e = Engine::new();
    for _ in 0..30000 {
        let s: String = (0..rng.below(24)).map(|_| TOKENS[rng.below(TOKENS.len())]).collect();
        let _ = render_with(&mut e, &s, true);
    }
}

#[test]
fn every_prefix_of_a_formula_is_handled() {
    let mut e = Engine::new();
    for seed in SEEDS {
        for at in 0..=seed.len() {
            if seed.is_char_boundary(at) {
                let _ = render_with(&mut e, &seed[..at], true);
            }
        }
    }
}

#[test]
fn rendering_is_deterministic() {
    let mut e = Engine::new();
    for s in SEEDS {
        let a = render_with(&mut e, s, true).unwrap();
        let b = render_with(&mut e, s, true).unwrap();
        assert_eq!(a, b, "{s}");
        assert_eq!(a, display(s).unwrap(), "{s}");
    }
}

#[test]
fn a_failed_render_leaves_nothing_behind() {
    let mut e = Engine::new();
    let want = render_with(&mut e, "\\frac{a}{b}", true).unwrap();
    for bad in ["\\foo", "{", "}", "x^1^2", "\\begin{x}", "\\left(", "\\def\\a{\\a}\\a", "\\textcolor{}{x}", "\\frac{\\textcolor{red}{"] {
        assert!(render_with(&mut e, bad, true).is_err(), "{bad}");
        assert_eq!(render_with(&mut e, "\\frac{a}{b}", true).unwrap(), want, "{bad}");
    }
}

#[test]
fn large_input_renders() {
    let s = "x+".repeat(50000) + "y";
    let svg = inline(&s).unwrap();
    well_formed(&svg).unwrap();
    assert_eq!(paths(&svg), 1);
}

#[test]
fn huge_rows_and_columns_render() {
    let row = "a&".repeat(200) + "b";
    let s = format!("\\begin{{matrix}}{}\\end{{matrix}}", vec![row; 200].join("\\\\"));
    well_formed(&inline(&s).unwrap()).unwrap();
}

#[test]
fn huge_numbers_stay_finite() {
    check(&["\\hspace{99999999999em}", "\\hspace{-99999999999em}", "\\rule{99999em}{99999em}", "\\mkern99999999mu", "x^{99999999999}", "\\sqrt[99999999]{x}"], |s| match inline(s) {
        Ok(svg) => well_formed(&svg),
        Err(_) => Ok(()),
    });
}

#[test]
fn null_bytes_do_not_end_input_early() {
    assert!(inline("a\0\\foo").is_err());
}
