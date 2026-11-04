use super::*;
use crate::packages::*;

#[test]
fn every_package_loads() {
    let names: Vec<&str> = PACKAGES.iter().map(|p| p.name).collect();
    assert!(names.len() >= 20);
    check(&names, |n| valid(&format!("\\usepackage{{{n}}}x")));
}

#[test]
fn every_ignored_package_loads() {
    check(NOOPS, |n| valid(&format!("\\usepackage{{{n}}}x")));
}

#[test]
fn a_package_can_be_loaded_twice() {
    check(&PACKAGES.iter().map(|p| p.name).collect::<Vec<_>>(), |n| valid(&format!("\\usepackage{{{n}}}\\usepackage{{{n}}}x")));
}

#[test]
fn packages_can_be_listed_together() {
    valid("\\usepackage{physics,siunitx,mhchem}\\dv{x}{t}\\si{m}\\ce{H2O}").unwrap();
}

#[test]
fn package_options_are_accepted() {
    valid("\\usepackage[version=4]{mhchem}\\ce{H2O}").unwrap();
    valid("\\usepackage[utf8]{inputenc}x").unwrap();
}

#[test]
fn every_package_command_has_a_working_form() {
    let shapes = ["\\N", "\\N{x}", "\\N{x}{y}", "\\N{x}{y}{z}", "\\N*{x}", "\\N[2]{x}", "\\N[2]{x}{y}", "\\N{1}{m}", "\\N{1}", "\\N{m}", "\\N{H2O}", "\\N{1}{2}{m}", "\\N(x)", "\\N[x]", "\\N{\\frac{a}{b}}"];
    let mut dead = Vec::new();
    for p in PACKAGES {
        for (name, _, _) in p.defs {
            if !shapes.iter().any(|s| inline(&format!("\\usepackage{{{}}}{}", p.name, s.replace("N", name))).is_ok()) {
                dead.push(format!("{}:{name}", p.name));
            }
        }
        for (name, _, _) in p.natives {
            if !shapes.iter().any(|s| inline(&format!("\\usepackage{{{}}}{}", p.name, s.replace("N", name))).is_ok()) {
                dead.push(format!("{}:{name}", p.name));
            }
        }
    }
    assert!(dead.is_empty(), "{dead:?}");
}

#[test]
fn package_commands_need_the_package() {
    check(&[("\\dv{x}{t}", "physics"), ("\\ket{x}", "physics"), ("\\ce{H2O}", "mhchem"), ("\\si{m}", "siunitx"), ("\\num{1}", "siunitx")], |r| {
        if inline(r.0).is_ok() {
            return Err("works without the package".into());
        }
        inline(&format!("\\usepackage{{{}}}{}", r.1, r.0)).map(|_| ()).map_err(|e| format!("{e:?}"))
    });
}

#[test]
fn packages_do_not_leak_between_renders() {
    let mut e = Engine::new();
    render_with(&mut e, "\\usepackage{physics}\\ket{x}", false).unwrap();
    assert!(render_with(&mut e, "\\ket{x}", false).is_err());
}

#[test]
fn session_packages_last_until_replaced() {
    let mut e = Engine::new();
    let n = feed(&mut e, "physics");
    assert_eq!(e.use_session_package(n), 0);
    render_with(&mut e, "\\ket{x}", false).unwrap();
    render_with(&mut e, "\\ket{y}", false).unwrap();
}

#[test]
fn unknown_session_package_is_refused() {
    let mut e = Engine::new();
    let n = feed(&mut e, "notapackage");
    assert_ne!(e.use_session_package(n), 0);
}

#[test]
fn physics_expands_as_documented() {
    check(
        &[
            ("\\ket{x}", "\\left|x\\right\\rangle"),
            ("\\bra{x}", "\\left\\langle x\\right|"),
            ("\\dv{x}{t}", "\\frac{\\mathrm{d}x}{\\mathrm{d}t}"),
            ("\\pdv{x}{t}", "\\frac{\\partial x}{\\partial t}"),
            ("\\abs{x}", "\\left\\lvert x\\right\\rvert"),
            ("\\norm{x}", "\\left\\lVert x\\right\\rVert"),
            ("\\vb{x}", "\\mathbf{x}"),
            ("\\comm{a}{b}", "\\left[a,b\\right]"),
            ("\\grad", "\\nabla"),
            ("\\tr", "\\operatorname{tr}"),
        ],
        |r| same(&format!("\\usepackage{{physics}}{}", r.0), r.1),
    );
}

#[test]
fn number_formats_like_siunitx() {
    check(&[("1", "1"), ("1.5", "1.5"), ("-3", "-3")], |r| {
        let mut out = Vec::new();
        number(r.0.as_bytes(), &mut out);
        same(&String::from_utf8(out).unwrap(), r.1)
    });
}

#[test]
fn numbers_with_exponents_use_times_ten() {
    let mut out = Vec::new();
    number(b"1e3", &mut out);
    let s = String::from_utf8(out).unwrap();
    assert!(s.contains("\\times") && s.contains("10"), "{s}");
}

#[test]
fn unit_macros_build_names() {
    check(&[("\\si{\\metre}", "\\si{m}"), ("\\si{\\kilo\\metre}", "\\si{km}"), ("\\si{\\second}", "\\si{s}"), ("\\si{\\kilo\\gram}", "\\si{kg}"), ("\\si{\\newton}", "\\si{N}")], |r| same(&format!("\\usepackage{{siunitx}}{}", r.0), &format!("\\usepackage{{siunitx}}{}", r.1)));
}

#[test]
fn chemistry_subscripts_digits() {
    check(&["H2O", "CO2", "H2SO4", "C6H12O6", "NaCl", "Fe^3+", "SO4^2-", "2H2 + O2 -> 2H2O", "A <=> B"], |s| valid(&format!("\\usepackage{{mhchem}}\\ce{{{s}}}")));
    assert!(measure("\\usepackage{mhchem}\\ce{H2O}").depth > measure("\\usepackage{mhchem}\\ce{HO}").depth);
}

#[test]
fn closing_brace_finds_the_match() {
    assert_eq!(closing_brace(b"{a}b", 1), 2);
    assert_eq!(closing_brace(b"{a{b}c}d", 1), 6);
}

#[test]
fn package_lookup_is_exact() {
    assert!(find_package(b"physics").is_some());
    assert!(find_package(b"physic").is_none());
    assert!(find_package(b"Physics").is_none());
    assert!(is_noop(b"amssymb"));
    assert!(!is_noop(b"notapackage"));
}
