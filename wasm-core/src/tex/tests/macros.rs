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
    check(&[("\\DeclareMathOperator{\\foo}{foo}\\foo", "\\operatorname{foo}"), ("\\DeclareMathOperator{\\Tr}{Tr}\\Tr x", "\\operatorname{Tr}x"), ("\\DeclareMathOperator*{\\am}{argmax}\\am_x", "\\operatorname*{argmax}_x")], |r| same(r.0, r.1));
}

#[test]
fn paired_delimiters_work() {
    check(&[("\\DeclarePairedDelimiter{\\abs}{\\lvert}{\\rvert}\\abs{x}", "\\lvert x\\rvert"), ("\\DeclarePairedDelimiter{\\abs}{\\lvert}{\\rvert}\\abs*{x}", "\\left\\lvert x\\right\\rvert")], |r| same(r.0, r.1));
}

#[test]
fn document_commands_take_arguments() {
    check(&[("\\NewDocumentCommand{\\a}{m}{#1^2}\\a{y}", "y^2"), ("\\NewDocumentCommand{\\a}{m m}{#2#1}\\a{x}{y}", "yx"), ("\\NewDocumentCommand{\\a}{s m}{\\IfBooleanTF{#1}{#2}{#2#2}}\\a{y}", "yy"), ("\\NewDocumentCommand{\\a}{s m}{\\IfBooleanTF{#1}{#2}{#2#2}}\\a*{y}", "y"), ("\\NewDocumentCommand{\\a}{o m}{\\IfNoValueTF{#1}{#2}{#1}}\\a{y}", "y"), ("\\NewDocumentCommand{\\a}{o m}{\\IfNoValueTF{#1}{#2}{#1}}\\a[z]{y}", "z")], |r| same(r.0, r.1));
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
