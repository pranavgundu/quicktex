use super::*;

#[test]
fn scripts_accept_single_tokens_and_groups() {
    check(&[("x^{2}", "x^2"), ("x_{i}", "x_i"), ("x^{\\alpha}", "x^\\alpha"), ("x_{\\beta}", "x_\\beta")], |r| same(r.0, r.1));
    check(&[("x_i^2", "x^2_i"), ("x_{i}^{2}", "x^{2}_{i}"), ("\\sum_{i}^{n}", "\\sum^{n}_{i}"), ("\\int_0^1", "\\int^1_0")], |r| {
        if measure_display(r.0) == measure_display(r.1) { Ok(()) } else { Err("sizes differ".into()) }
    });
}

#[test]
fn whitespace_does_not_matter_in_math() {
    check(
        &[
            ("a  b", "ab"),
            ("a\nb", "ab"),
            ("a\tb", "ab"),
            (" a ", "a"),
            ("\\alpha  x", "\\alpha x"),
            ("x ^ 2", "x^2"),
            ("x _ i", "x_i"),
            ("\\frac {a} {b}", "\\frac{a}{b}"),
            ("\\sqrt {x}", "\\sqrt{x}"),
            ("a + b", "a+b"),
            ("a = b", "a=b"),
            ("\\sum _ { i }", "\\sum_{i}"),
        ],
        |r| same(r.0, r.1),
    );
}

#[test]
fn comments_are_dropped() {
    check(
        &[("a%comment\nb", "ab"), ("a%", "a"), ("a% x^2 \\frac{\nb", "ab"), ("%only\nx", "x"), ("\\frac{a}%c\n{b}", "\\frac{a}{b}")],
        |r| same(r.0, r.1),
    );
}

#[test]
fn fractions_accept_short_arguments() {
    check(
        &[
            ("\\frac12", "\\frac{1}{2}"),
            ("\\frac ab", "\\frac{a}{b}"),
            ("\\frac1b", "\\frac{1}{b}"),
            ("\\frac\\alpha\\beta", "\\frac{\\alpha}{\\beta}"),
            ("\\sqrt2", "\\sqrt{2}"),
            ("\\sqrt\\alpha", "\\sqrt{\\alpha}"),
            ("\\binom nk", "\\binom{n}{k}"),
            ("\\hat x", "\\hat{x}"),
            ("\\vec v", "\\vec{v}"),
            ("\\mathbf x", "\\mathbf{x}"),
        ],
        |r| same(r.0, r.1),
    );
}

#[test]
fn infix_forms_match_the_commands() {
    check(
        &[
            ("{a \\over b}", "\\frac{a}{b}"),
            ("{n \\choose k}", "\\binom{n}{k}"),
            ("a \\over b", "\\frac{a}{b}"),
            ("{a+b \\over c+d}", "\\frac{a+b}{c+d}"),
        ],
        |r| same(r.0, r.1),
    );
}

#[test]
fn style_commands_match_the_shorthand() {
    check(
        &[
            ("\\dfrac{a}{b}", "\\displaystyle\\frac{a}{b}"),
            ("\\displaystyle x", "x"),
            ("\\displaystyle\\sum_i", "\\sum_i"),
            ("\\dbinom{n}{k}", "\\displaystyle\\binom{n}{k}"),
        ],
        |r| same(r.0, r.1),
    );
    let a = inline("\\tfrac{a}{b}").unwrap();
    let b = inline("\\frac{a}{b}").unwrap();
    assert_eq!(a, b);
}

#[test]
fn symbol_aliases_match() {
    check(
        &[
            ("\\le", "\\leq"),
            ("\\ge", "\\geq"),
            ("\\ne", "\\neq"),
            ("\\to", "\\rightarrow"),
            ("\\gets", "\\leftarrow"),
            ("\\land", "\\wedge"),
            ("\\lor", "\\vee"),
            ("\\lnot", "\\neg"),
            ("\\lbrace", "\\{"),
            ("\\rbrace", "\\}"),
            ("\\lbrack", "["),
            ("\\rbrack", "]"),
            ("\\vert", "\\lvert"),
            ("\\Vert", "\\lVert"),
            ("\\|", "\\Vert"),
            ("\\owns", "\\ni"),
            ("\\iff", "\\;\\Longleftrightarrow\\;"),
            ("\\implies", "\\;\\Longrightarrow\\;"),
            ("\\impliedby", "\\;\\Longleftarrow\\;"),
        ],
        |r| same(r.0, r.1),
    );
}

#[test]
fn unicode_input_matches_commands() {
    check(
        &[
            ("α", "\\alpha"),
            ("β", "\\beta"),
            ("γ", "\\gamma"),
            ("π", "\\pi"),
            ("θ", "\\theta"),
            ("ω", "\\omega"),
            ("Σ", "\\Sigma"),
            ("Ω", "\\Omega"),
            ("∑", "\\sum"),
            ("∫", "\\int"),
            ("∏", "\\prod"),
            ("∞", "\\infty"),
            ("→", "\\rightarrow"),
            ("←", "\\leftarrow"),
            ("≤", "\\leq"),
            ("≥", "\\geq"),
            ("≠", "\\neq"),
            ("×", "\\times"),
            ("÷", "\\div"),
            ("±", "\\pm"),
            ("∈", "\\in"),
            ("∂", "\\partial"),
            ("∇", "\\nabla"),
            ("∀", "\\forall"),
            ("∃", "\\exists"),
        ],
        |r| same(r.0, r.1),
    );
}

#[test]
fn spacing_commands_match_their_lengths() {
    check(
        &[
            ("a\\quad b", "a\\hspace{1em}b"),
            ("a\\qquad b", "a\\hspace{2em}b"),
            ("a\\,b", "a\\thinspace b"),
            ("a\\:b", "a\\>b"),
            ("a\\:b", "a\\medspace b"),
            ("a\\;b", "a\\thickspace b"),
            ("a\\!b", "a\\negthinspace b"),
            ("a\\hspace{1em}b", "a\\kern1em b"),
            ("a\\hspace{10pt}b", "a\\hskip10pt b"),
        ],
        |r| same(r.0, r.1),
    );
}

#[test]
fn mkern_matches_named_spaces_within_rounding() {
    check(&[("a\\,b", "a\\mkern3mu b"), ("a\\:b", "a\\mkern4mu b"), ("a\\;b", "a\\mkern5mu b"), ("a\\!b", "a\\mkern-3mu b")], |r| {
        let gap = (measure(r.0).width - measure(r.1).width).abs();
        if gap <= 1 { Ok(()) } else { Err(format!("off by {gap}")) }
    });
}

#[test]
fn old_font_switches_match_new_commands() {
    check(
        &[
            ("{\\rm x}", "\\mathrm{x}"),
            ("{\\bf x}", "\\mathbf{x}"),
            ("{\\it x}", "\\mathit{x}"),
            ("{\\sf x}", "\\mathsf{x}"),
            ("{\\tt x}", "\\mathtt{x}"),
            ("{\\cal X}", "\\mathcal{X}"),
        ],
        |r| same(r.0, r.1),
    );
}

#[test]
fn operators_match_operatorname() {
    check(
        &[
            "sin", "cos", "tan", "log", "ln", "exp", "det", "dim", "ker", "deg", "arg", "cot", "sec", "csc", "sinh", "cosh", "tanh",
            "arcsin", "arccos", "arctan", "gcd", "hom",
        ],
        |n| same(&format!("\\{n}"), &format!("\\operatorname{{{n}}}")),
    );
}

#[test]
fn delimiters_can_be_written_two_ways() {
    check(
        &[
            ("\\left(x\\right)", "\\left( x \\right)"),
            ("\\left\\{x\\right\\}", "\\left\\lbrace x\\right\\rbrace"),
            ("\\left[x\\right]", "\\left\\lbrack x\\right\\rbrack"),
            ("\\left|x\\right|", "\\left\\vert x\\right\\vert"),
            ("\\left\\|x\\right\\|", "\\left\\Vert x\\right\\Vert"),
        ],
        |r| same(r.0, r.1),
    );
}

#[test]
fn environments_ignore_trailing_row_break() {
    check(
        &[
            ("\\begin{matrix}a\\\\b\\\\\\end{matrix}", "\\begin{matrix}a\\\\b\\end{matrix}"),
            ("\\begin{cases}a\\\\b\\\\\\end{cases}", "\\begin{cases}a\\\\b\\end{cases}"),
            ("\\begin{aligned}a&=b\\\\\\end{aligned}", "\\begin{aligned}a&=b\\end{aligned}"),
        ],
        |r| same(r.0, r.1),
    );
}

#[test]
fn matrix_shorthands_add_delimiters() {
    check(
        &[
            ("\\begin{pmatrix}a\\end{pmatrix}", "\\left(\\begin{matrix}a\\end{matrix}\\right)"),
            ("\\begin{bmatrix}a\\end{bmatrix}", "\\left[\\begin{matrix}a\\end{matrix}\\right]"),
            ("\\begin{Bmatrix}a\\end{Bmatrix}", "\\left\\{\\begin{matrix}a\\end{matrix}\\right\\}"),
            ("\\begin{vmatrix}a\\end{vmatrix}", "\\left|\\begin{matrix}a\\end{matrix}\\right|"),
            ("\\begin{Vmatrix}a\\end{Vmatrix}", "\\left\\|\\begin{matrix}a\\end{matrix}\\right\\|"),
        ],
        |r| same(r.0, r.1),
    );
}

#[test]
fn colors_can_be_written_every_way() {
    check(&[("\\textcolor{red}{x}", "\\color{red}x"), ("{\\color{blue}x}", "\\textcolor{blue}{x}")], |r| same(r.0, r.1));
    check(&[("\\textcolor{red}{x}", "\\textcolor{#ff0000}{x}"), ("\\textcolor{#f00}{x}", "\\textcolor{#FF0000}{x}")], |r| {
        if measure(r.0) == measure(r.1) { Ok(()) } else { Err("sizes differ".into()) }
    });
}

#[test]
fn colored_output_carries_the_color() {
    check(&["red", "blue", "green", "#123456", "#abcdef"], |c| {
        let svg = inline(&format!("\\textcolor{{{c}}}{{x}}")).map_err(|e| format!("{e:?}"))?;
        if svg.contains("fill=\"") && svg.matches("fill=\"").count() == 2 { Ok(()) } else { Err(svg[..200.min(svg.len())].into()) }
    });
}

#[test]
fn valid_formulas_render() {
    check(
        &[
            "",
            " ",
            "x",
            "x+y=z",
            "E=mc^2",
            "a^2+b^2=c^2",
            "\\frac{-b\\pm\\sqrt{b^2-4ac}}{2a}",
            "\\int_{-\\infty}^{\\infty}e^{-x^2}\\,dx=\\sqrt{\\pi}",
            "\\sum_{n=1}^{\\infty}\\frac{1}{n^2}=\\frac{\\pi^2}{6}",
            "\\prod_{i=1}^n x_i",
            "\\lim_{x\\to0}\\frac{\\sin x}{x}=1",
            "e^{i\\pi}+1=0",
            "\\nabla\\cdot\\mathbf{E}=\\frac{\\rho}{\\varepsilon_0}",
            "\\left(\\frac{a}{b}\\right)^2",
            "\\left\\{x\\middle|x>0\\right\\}",
            "\\left.\\frac{a}{b}\\right|_{x=0}",
            "\\begin{matrix}1&2\\\\3&4\\end{matrix}",
            "\\begin{pmatrix}a&b\\\\c&d\\end{pmatrix}",
            "\\begin{bmatrix}1\\\\2\\\\3\\end{bmatrix}",
            "\\begin{vmatrix}a&b\\\\c&d\\end{vmatrix}",
            "\\begin{smallmatrix}a&b\\\\c&d\\end{smallmatrix}",
            "\\begin{cases}x&x>0\\\\-x&x\\le0\\end{cases}",
            "\\begin{dcases}x\\\\y\\end{dcases}",
            "\\begin{rcases}x\\\\y\\end{rcases}",
            "\\begin{aligned}a&=b\\\\c&=d\\end{aligned}",
            "\\begin{align}a&=b\\end{align}",
            "\\begin{align*}a&=b\\end{align*}",
            "\\begin{split}a&=b\\\\&=c\\end{split}",
            "\\begin{gathered}a\\\\b\\end{gathered}",
            "\\begin{gather}a\\\\b\\end{gather}",
            "\\begin{equation}x\\end{equation}",
            "\\begin{alignat}{2}a&=b&c&=d\\end{alignat}",
            "\\begin{array}{lcr}a&b&c\\end{array}",
            "\\begin{array}{|c|c|}\\hline a&b\\\\\\hline\\end{array}",
            "\\overbrace{a+b}^{n}",
            "\\underbrace{a+b}_{n}",
            "\\overline{ab}",
            "\\underline{ab}",
            "\\widehat{abc}",
            "\\widetilde{abc}",
            "\\overrightarrow{AB}",
            "\\overleftarrow{AB}",
            "\\xrightarrow{f}",
            "\\xleftarrow[g]{f}",
            "\\sqrt[3]{x}",
            "\\sqrt[n]{1+x}",
            "\\boxed{x}",
            "\\usepackage{cancel}\\cancel{x}",
            "\\text{hello world}",
            "\\text{a $x$ b}",
            "\\mathbb{R}",
            "\\mathcal{L}",
            "\\mathfrak{g}",
            "\\mathscr{F}",
            "\\mathbf{v}",
            "\\mathit{v}",
            "\\mathrm{d}x",
            "\\mathsf{A}",
            "\\mathtt{A}",
            "\\boldsymbol{\\alpha}",
            "\\operatorname{foo}(x)",
            "\\operatorname*{argmax}_x f",
            "\\mathop{X}\\limits_{a}",
            "\\sum\\nolimits_i",
            "\\int\\limits_0^1",
            "\\iint",
            "\\iiint",
            "\\oint",
            "\\binom{n}{k}",
            "\\stackrel{a}{b}",
            "\\overset{a}{b}",
            "\\underset{a}{b}",
            "\\substack{a\\\\b}",
            "\\sum_{\\substack{i<n\\\\j<m}}",
            "\\pmod{n}",
            "a\\bmod b",
            "\\big(\\Big(\\bigg(\\Bigg(",
            "\\bigl(\\bigr)",
            "\\phantom{x}",
            "\\hphantom{x}",
            "\\vphantom{x}",
            "\\smash{x}",
            "\\rlap{x}",
            "\\llap{x}",
            "\\tiny x",
            "\\scriptsize x",
            "\\footnotesize x",
            "\\small x",
            "\\normalsize x",
            "\\large x",
            "\\Large x",
            "\\LARGE x",
            "\\huge x",
            "\\Huge x",
            "x'",
            "x''",
            "x'''",
            "x'^2",
            "f'(x)",
            "\\dots",
            "\\ldots",
            "\\cdots",
            "\\vdots",
            "\\ddots",
            "\\%\\$\\#\\&\\_\\{\\}",
            "\\rule{1em}{1em}",
            "\\mathbin{x}",
            "\\mathrel{x}",
            "\\mathopen{x}",
            "\\mathclose{x}",
            "\\mathpunct{x}",
            "\\mathinner{x}",
            "\\mathord{x}",
            "x^{y^{z^{w}}}",
            "x_{y_{z_{w}}}",
            "\\frac{\\frac{\\frac{a}{b}}{c}}{d}",
            "\\sqrt{\\sqrt{\\sqrt{x}}}",
            "{{{{x}}}}",
        ],
        |s| valid(s),
    );
}

#[test]
fn empty_groups_are_harmless() {
    check(
        &[
            "{}",
            "{}{}",
            "x{}",
            "{}x",
            "{}^2",
            "{}_2",
            "x^{}",
            "x_{}",
            "\\frac{}{}",
            "\\sqrt{}",
            "\\left(\\right)",
            "\\text{}",
            "\\mathbf{}",
            "\\begin{matrix}\\end{matrix}",
            "\\overline{}",
        ],
        |s| valid(s),
    );
}

#[test]
fn empty_input_has_no_ink() {
    let svg = inline("").unwrap();
    assert_eq!(paths(&svg), 0);
    assert_eq!(size_of(&svg).width, 0);
}

#[test]
fn text_mode_keeps_spaces() {
    assert!(measure("\\text{a b}").width > measure("\\text{ab}").width);
}

#[test]
fn escapes_draw_ink() {
    check(&["\\%", "\\$", "\\#", "\\&", "\\_", "\\{", "\\}"], |s| {
        let svg = inline(s).map_err(|e| format!("{e:?}"))?;
        if paths(&svg) == 1 { Ok(()) } else { Err("no ink".into()) }
    });
}

#[test]
fn math_font_commands_change_the_glyphs() {
    let plain = inline("x").unwrap();
    check(&["\\mathbf{x}", "\\mathrm{x}", "\\mathsf{x}", "\\mathtt{x}", "\\mathfrak{x}", "\\boldsymbol{x}"], |s| {
        if inline(s).map_err(|e| format!("{e:?}"))? != plain { Ok(()) } else { Err("same as plain".into()) }
    });
}

#[test]
fn distinct_letters_draw_distinct_shapes() {
    let mut seen: Vec<String> = ('a'..='z').map(|c| inline(&c.to_string()).unwrap()).collect();
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), 26);
}
