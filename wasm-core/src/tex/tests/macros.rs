use super::*;

#[test]
fn definitions_expand() {
    check(
        &[
            ("\\newcommand{\\a}{x}\\a", "x"),
            ("\\newcommand\\a{x}\\a", "x"),
            ("\\renewcommand{\\alpha}{x}\\alpha", "x"),
            ("\\providecommand{\\a}{x}\\a", "x"),
            ("\\def\\a{x}\\a", "x"),
            ("\\newcommand{\\a}[1]{#1^2}\\a{y}", "y^2"),
            ("\\newcommand{\\a}[2]{#2#1}\\a{x}{y}", "yx"),
            ("\\newcommand{\\a}[1]{#1#1}\\a{y}", "yy"),
            ("\\newcommand{\\a}[2][z]{#1#2}\\a{y}", "zy"),
            ("\\newcommand{\\a}[2][z]{#1#2}\\a[w]{y}", "wy"),
            ("\\def\\a#1{#1^2}\\a{y}", "y^2"),
            ("\\def\\a#1#2{#2#1}\\a{x}{y}", "yx"),
            ("\\newcommand{\\a}{x}\\newcommand{\\b}{\\a\\a}\\b", "xx"),
            ("\\newcommand{\\R}{\\mathbb{R}}x\\in\\R", "x\\in\\mathbb{R}"),
            ("\\newcommand{\\a}[1]{\\frac{#1}{2}}\\a{\\a{x}}", "\\frac{\\frac{x}{2}}{2}"),
            ("\\def\\a{x}\\def\\a{y}\\a", "y"),
            ("\\newcommand{\\a}{x}\\renewcommand{\\a}{y}\\a", "y"),
        ],
        |r| same(r.0, r.1),
    );
}

#[test]
fn provide_does_not_replace() {
    same("\\newcommand{\\a}{x}\\providecommand{\\a}{y}\\a", "x").unwrap();
}

#[test]
fn declared_operators_match_operatorname() {
    check(
        &[
            ("\\DeclareMathOperator{\\foo}{foo}\\foo", "\\operatorname{foo}"),
            ("\\DeclareMathOperator{\\Tr}{Tr}\\Tr x", "\\operatorname{Tr}x"),
            ("\\DeclareMathOperator*{\\am}{argmax}\\am_x", "\\operatorname*{argmax}_x"),
        ],
        |r| same(r.0, r.1),
    );
}

#[test]
fn paired_delimiters_work() {
    check(
        &[
            ("\\DeclarePairedDelimiter{\\abs}{\\lvert}{\\rvert}\\abs{x}", "\\lvert x\\rvert"),
            ("\\DeclarePairedDelimiter{\\abs}{\\lvert}{\\rvert}\\abs*{x}", "\\left\\lvert x\\right\\rvert"),
        ],
        |r| same(r.0, r.1),
    );
}

#[test]
fn document_commands_take_arguments() {
    check(
        &[
            ("\\NewDocumentCommand{\\a}{m}{#1^2}\\a{y}", "y^2"),
            ("\\NewDocumentCommand{\\a}{m m}{#2#1}\\a{x}{y}", "yx"),
            ("\\NewDocumentCommand{\\a}{s m}{\\IfBooleanTF{#1}{#2}{#2#2}}\\a{y}", "yy"),
            ("\\NewDocumentCommand{\\a}{s m}{\\IfBooleanTF{#1}{#2}{#2#2}}\\a*{y}", "y"),
            ("\\NewDocumentCommand{\\a}{o m}{\\IfNoValueTF{#1}{#2}{#1}}\\a{y}", "y"),
            ("\\NewDocumentCommand{\\a}{o m}{\\IfNoValueTF{#1}{#2}{#1}}\\a[z]{y}", "z"),
        ],
        |r| same(r.0, r.1),
    );
}

#[test]
fn defined_colors_work() {
    same("\\definecolor{mine}{HTML}{FF0000}\\textcolor{mine}{x}", "\\textcolor{#ff0000}{x}").unwrap();
    same("\\definecolor{mine}{RGB}{255,0,0}\\textcolor{mine}{x}", "\\textcolor{#ff0000}{x}").unwrap();
    same("\\definecolor{mine}{rgb}{1,0,0}\\textcolor{mine}{x}", "\\textcolor{#ff0000}{x}").unwrap();
}

#[test]
fn definitions_do_not_leak_between_renders() {
    let mut e = Engine::new();
    render_with(&mut e, "\\newcommand{\\a}{x}\\a", false).unwrap();
    assert_eq!(render_with(&mut e, "\\a", false).unwrap_err().0, "unknown command");
    render_with(&mut e, "\\renewcommand{\\alpha}{x}\\alpha", false).unwrap();
    assert_eq!(render_with(&mut e, "\\alpha", false).unwrap(), inline("\\alpha").unwrap());
}

#[test]
fn macro_errors_point_at_the_call() {
    let s = "\\newcommand{\\a}{\\nope}xy\\a";
    assert_eq!(inline(s).unwrap_err(), ("unknown command".to_string(), s.len()));
}

#[test]
fn macro_argument_can_be_a_single_token() {
    same("\\newcommand{\\a}[1]{#1^2}\\a y", "y^2").unwrap();
    same("\\newcommand{\\a}[2]{#1#2}\\a xy", "xy").unwrap();
}

#[test]
fn many_macros_can_coexist() {
    let mut s = String::new();
    let mut want = String::new();
    for c in 'a'..='z' {
        s.push_str(&format!("\\newcommand{{\\m{c}}}{{{c}}}"));
    }
    for c in 'a'..='z' {
        s.push_str(&format!("\\m{c} "));
        want.push(c);
    }
    same(&s, &want).unwrap();
}

#[test]
fn counters_and_integer_registers_work() {
    check(
        &[
            (r"\newcounter{n}\setcounter{n}{12}\arabic{n}", "12"),
            (r"\newcounter{n}\addtocounter{n}{-2}\then", "-2"),
            (r"\newcounter{n}\stepcounter{n}\alph{n}\Alph{n}", "aA"),
            (r"\newcounter{n}\setcounter{n}{14}\roman{n}\Roman{n}", "xivXIV"),
            (r"\newcounter{p}\newcounter{c}[p]\setcounter{c}{7}\stepcounter{p}\arabic{p}\arabic{c}", "10"),
            (r"\newcount\i\i=2\advance\i by 3\the\i", "5"),
            (r"\newcount\i\i=-2\advance\i 1\number\i", "-1"),
            (r"\newcounter{n}\setcounter{n}{2}\newcounter{m}\setcounter{m}{\value{n}}\arabic{m}", "2"),
            (r"\text{\newcounter{n}\stepcounter{n}\arabic{n}}", r"\text{1}"),
        ],
        |r| same(r.0, r.1),
    );
}

#[test]
fn tex_loops_evaluate_state_each_iteration() {
    check(
        &[
            (r"\newcount\i\i=0\loop\the\i\advance\i by 1\ifnum\i<4\repeat", "0123"),
            (r"\newcounter{n}\loop\stepcounter{n}\arabic{n}\ifnum\value{n}<3\repeat", "123"),
            (r"\newcount\i\i=0\loop\advance\i by1\ifnum\i<3 x\repeat", "xx"),
            (r"\newcount\i\newcount\j\i=0\loop\j=0\loop x\advance\j by1\ifnum\j<2\repeat\advance\i by1\ifnum\i<2\repeat", "xxxx"),
        ],
        |r| same(r.0, r.1),
    );
}

#[test]
fn counter_errors_and_loops_are_bounded() {
    for source in [r"\setcounter{missing}{1}", r"\newcounter{n}\setcounter{n}{oops}", r"\newcount\i\i=2147483647\advance\i by1"] {
        assert_eq!(inline(source).unwrap_err().0, "invalid or undefined counter", "{source}");
    }
    for source in [r"\loop x\repeat", r"\loop x\ifnum1<2", r"\loop\ifnum1<2\repeat"] {
        assert_eq!(inline(source).unwrap_err().0, "invalid loop or iteration limit exceeded", "{source}");
    }
    let mut engine = Engine::new();
    render_with(&mut engine, r"\newcounter{n}\stepcounter{n}\arabic{n}", false).unwrap();
    assert_eq!(render_with(&mut engine, r"\arabic{n}", false).unwrap_err().0, "invalid or undefined counter");
}

#[test]
fn deeply_nested_counter_values_fail_with_a_depth_error() {
    let source = format!("\\newcounter{{n}}\\setcounter{{n}}{}1{}", "{".repeat(5000), "}".repeat(5000));
    assert_eq!(inline(&source).unwrap_err().0, "nesting too deep");
}

#[test]
fn user_counter_formatting_macros_take_precedence() {
    same(r"\setcounter{section}{4}\setcounter{subsection}{2}\setcounter{subsubsection}{3}\thesubsection\thesubsubsection", "4.24.2.3")
        .unwrap();
    same(r"\setcounter{section}{-2147483648}\arabic{section}", "-2147483648").unwrap();
    same(r"\newcounter{foo}\stepcounter{foo}\renewcommand{\thefoo}{z}\thefoo", "z").unwrap();
    same(r"\setcounter{section}{4}\renewcommand{\thesection}{\Roman{section}}\thesection", "IV").unwrap();
}
