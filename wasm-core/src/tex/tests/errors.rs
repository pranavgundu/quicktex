use super::*;

fn fails(rows: &[(&str, &str)]) {
    check(rows, |r| match inline(r.0) {
        Err((message, at)) if message == r.1 && at <= r.0.len() => Ok(()),
        Err((message, at)) => Err(format!("got {message} at {at}")),
        Ok(_) => Err("rendered".into()),
    });
}

fn at(rows: &[(&str, usize)]) {
    check(rows, |r| match inline(r.0) {
        Err((_, found)) if found == r.1 => Ok(()),
        Err((message, found)) => Err(format!("{message} at {found}")),
        Ok(_) => Err("rendered".into()),
    });
}

#[test]
fn unknown_commands() {
    fails(&[("\\notacommand", "unknown command"), ("x\\foo y", "unknown command"), ("\\frac{\\foo}{b}", "unknown command"), ("\\text{\\foo}", "unknown command"), ("\\alpha\\betaa", "unknown command"), ("\\sqrt{\\nope}", "unknown command")]);
}

#[test]
fn unknown_command_position_is_the_command() {
    at(&[("\\foo", 0), ("x\\foo", 1), ("xy+\\foo", 3), ("\\frac{a}{\\foo}", 9)]);
}

#[test]
fn unbalanced_braces() {
    fails(&[("}", "unbalanced braces"), ("x}", "unbalanced braces"), ("{x}}", "unbalanced braces")]);
}

#[test]
fn missing_closers_are_end_of_input() {
    fails(&[("{", "unexpected end of input"), ("{x", "unexpected end of input"), ("\\frac{a", "unexpected end of input"), ("\\frac{a}{b", "unexpected end of input"), ("\\sqrt{", "unexpected end of input"), ("x^{", "unexpected end of input"), ("\\left(", "unexpected end of input"), ("\\begin{matrix}", "unexpected end of input")]);
}

#[test]
fn double_scripts() {
    fails(&[("x^2^3", "double script"), ("x_1_2", "double script"), ("x^a_b^c", "double script"), ("x_a^b_c", "double script")]);
}

#[test]
fn double_script_position_is_the_second_script() {
    at(&[("x^2^3", 3), ("x_1_2", 3)]);
}

#[test]
fn missing_arguments() {
    fails(&[("\\frac", "missing argument"), ("\\sqrt", "missing argument"), ("x^", "missing argument"), ("x_", "missing argument"), ("\\hat", "missing argument"), ("\\mathbf", "missing argument"), ("\\text", "missing argument")]);
}

#[test]
fn delimiters_must_be_delimiters() {
    fails(&[("\\left x\\right)", "invalid delimiter"), ("\\left(x\\right y", "invalid delimiter"), ("\\big x", "invalid delimiter"), ("\\left\\alpha x\\right.", "invalid delimiter")]);
}

#[test]
fn stray_closers_are_misplaced() {
    fails(&[("\\right)", "misplaced token"), ("\\middle|", "misplaced token"), ("&", "misplaced token"), ("a&b", "misplaced token")]);
}

#[test]
fn environments_must_match() {
    fails(&[("\\begin{nope}x\\end{nope}", "unknown or mismatched environment"), ("\\begin{matrix}a\\end{pmatrix}", "unknown or mismatched environment"), ("\\end{matrix}", "misplaced token")]);
}

#[test]
fn colors_must_not_be_empty() {
    fails(&[("\\textcolor{}{x}", "invalid color"), ("\\color{}x", "invalid color")]);
}

#[test]
fn dimensions_must_parse() {
    fails(&[("\\hspace{abc}", "invalid dimension"), ("\\hspace{1}", "invalid dimension"), ("\\hspace{1furlong}", "invalid dimension"), ("\\rule{x}{1em}", "invalid dimension")]);
}

#[test]
fn packages_must_exist() {
    fails(&[("\\usepackage{notapackage}", "unknown package"), ("\\usepackage{amsmath,notapackage}", "unknown package")]);
}

#[test]
fn deep_nesting_is_refused() {
    for depth in [2000, 10000, 50000] {
        let brace = format!("{}x{}", "{".repeat(depth), "}".repeat(depth));
        assert_eq!(inline(&brace).unwrap_err().0, "nesting too deep", "{depth}");
        let frac = format!("{}x{}", "\\frac{".repeat(depth), "}{y}".repeat(depth));
        assert_eq!(inline(&frac).unwrap_err().0, "nesting too deep", "{depth}");
        let sup = format!("{}x{}", "x^{".repeat(depth), "}".repeat(depth));
        assert_eq!(inline(&sup).unwrap_err().0, "nesting too deep", "{depth}");
    }
}

#[test]
fn recursive_macros_stop() {
    check(&["\\def\\a{\\a}\\a", "\\def\\a{\\a\\a}\\a", "\\newcommand{\\a}{\\a}\\a", "\\def\\a{\\b}\\def\\b{\\a}\\a", "\\def\\a{x\\a}\\a", "\\newcommand{\\a}[1]{\\a{#1#1}}\\a{x}"], |s| if inline(s).is_err() { Ok(()) } else { Err("rendered".into()) });
}

#[test]
fn every_error_position_is_inside_the_input() {
    check(&["\\foo", "{", "}", "x^", "\\frac", "\\left(", "\\begin{x}", "\\hspace{q}", "x^1^2", "é\\foo", "日本語\\foo"], |s| match inline(s) {
        Err((_, at)) if at <= s.len() => Ok(()),
        Err((_, at)) => Err(format!("position {at}")),
        Ok(_) => Err("rendered".into()),
    });
}

#[test]
fn error_position_counts_bytes() {
    assert_eq!(inline("\u{3b1}\\foo").unwrap_err(), ("unknown command".to_string(), 2));
    assert_eq!(inline("\u{2211}\\foo").unwrap_err(), ("unknown command".to_string(), 3));
}

#[test]
fn error_messages_are_lowercase_sentences() {
    check(&["\\foo", "{", "}", "x^", "\\left x", "x^1^2", "\\begin{q}"], |s| {
        let (message, _) = inline(s).unwrap_err();
        if message.chars().next().unwrap().is_lowercase() && !message.ends_with('.') { Ok(()) } else { Err(message) }
    });
}

#[test]
fn colors_cannot_break_out_of_the_attribute() {
    check(&["\\textcolor{red\" onclick=\"x}{y}", "\\color{\"/><script>x</script>}y", "\\textcolor{a'b}{y}", "\\textcolor{a<b}{y}", "\\textcolor{a>b}{y}", "\\textcolor{a b}{y}", "\\textcolor{a&b}{y}", "\\textcolor{a;b}{y}", "\\textcolor{a(b)}{y}", "\\textcolor{a/b}{y}", "\\textcolor{a=b}{y}", "\\definecolor{m}{HTML}{\"><b>}\\textcolor{m}{y}"], |s| match inline(s) {
        Err((message, _)) if message == "invalid color" => Ok(()),
        Err((message, _)) => Err(message),
        Ok(svg) => Err(svg[..120].to_string()),
    });
}

#[test]
fn error_position_never_passes_the_end() {
    check(&["\\usepackage{siunitx}\\SI{1e3}{\\kilo\\metre\\", "\\", "x\\", "\\frac{a}{\\", "\\text{\\"], |s| match inline(s) {
        Err((_, at)) if at <= s.len() => Ok(()),
        Err((_, at)) => Err(format!("position {at}")),
        Ok(_) => Ok(()),
    });
}

#[test]
fn unsupported_characters_are_pointed_at_directly() {
    assert_eq!(inline("ab\u{4e16}c").unwrap_err(), ("unsupported character".to_string(), 2));
    assert_eq!(inline("\u{1f600}").unwrap_err(), ("unsupported character".to_string(), 0));
}
