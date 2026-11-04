use super::*;
use crate::data::*;
use crate::tex::lookup;

fn working_form(name: &str) -> Result<String, String> {
    let forms = [format!("\\{name}"), name.to_string(), format!("\\text{{\\{name}}}"), format!("\\text{{{name}}}")];
    forms.iter().find(|s| inline(s).is_ok()).cloned().ok_or_else(|| "no form renders".to_string())
}

fn name_of(e: &Entry) -> &'static str {
    core::str::from_utf8(&NAMES[e.off as usize..(e.off + e.len) as usize]).unwrap()
}

#[test]
fn every_entry_has_a_name() {
    assert!(ENTRIES.len() > 1500);
    let mut empty = 0;
    for e in &ENTRIES {
        empty += (e.len == 0) as u32;
        assert!((e.off + e.len) as usize <= NAMES.len());
    }
    assert_eq!(empty, 1);
}

#[test]
fn every_name_is_found_by_lookup() {
    for e in &ENTRIES {
        let name = &NAMES[e.off as usize..(e.off + e.len) as usize];
        let found = lookup(name).unwrap_or_else(|| panic!("{} not found", name_of(e)));
        assert_eq!(found.kind, e.sym.kind, "{}", name_of(e));
        assert_eq!(found.glyph, e.sym.glyph, "{}", name_of(e));
    }
}

#[test]
fn names_are_unique() {
    let mut names: Vec<&str> = ENTRIES.iter().map(name_of).collect();
    names.sort();
    let before = names.len();
    names.dedup();
    assert_eq!(before, names.len());
}

#[test]
fn unknown_names_are_not_found() {
    for name in ["notacommand", "alphaa", "alph", "Alpha1", "frac ", "qqquad", "x y"] {
        assert!(lookup(name.as_bytes()).is_none(), "{name}");
    }
}

#[test]
fn lookup_is_case_sensitive() {
    assert_ne!(lookup(b"delta").unwrap().glyph, lookup(b"Delta").unwrap().glyph);
    assert_ne!(lookup(b"sigma").unwrap().glyph, lookup(b"Sigma").unwrap().glyph);
    assert_ne!(lookup(b"omega").unwrap().glyph, lookup(b"Omega").unwrap().glyph);
}

#[test]
fn every_glyph_stays_inside_the_outline_table() {
    assert!(GLYPHS.len() > 1000);
    for (i, g) in GLYPHS.iter().enumerate() {
        assert!(g.off as usize + g.len as usize <= PATHS.len(), "glyph {i}");
    }
}

#[test]
fn every_outline_is_path_data() {
    for (i, g) in GLYPHS.iter().enumerate() {
        let bytes = &PATHS[g.off as usize..g.off as usize + g.len as usize];
        assert!(bytes.iter().all(|c| b"MmLlHhVvCcSsQqTtZz0123456789 ,.-".contains(c)), "glyph {i}");
    }
}

#[test]
fn every_symbol_points_at_a_real_glyph() {
    for e in &ENTRIES {
        if e.sym.kind == K_SYM {
            assert!((e.sym.glyph as usize) < GLYPHS.len(), "{}", name_of(e));
        }
    }
}

#[test]
fn every_printable_ascii_character_has_a_defined_result() {
    for c in 32u8..127 {
        let s = (c as char).to_string();
        match inline(&s) {
            Ok(svg) => well_formed(&svg).unwrap_or_else(|e| panic!("{s}: {e}")),
            Err((message, _)) => assert!(!message.is_empty(), "{s}"),
        }
    }
}

#[test]
fn letters_and_digits_all_draw_ink() {
    for c in ('a'..='z').chain('A'..='Z').chain('0'..='9') {
        let s = c.to_string();
        let svg = inline(&s).unwrap();
        assert_eq!(paths(&svg), 1, "{s}");
        assert!(size_of(&svg).width > 0, "{s}");
    }
}

#[test]
fn every_symbol_renders_alone() {
    let rows: Vec<&str> = ENTRIES.iter().filter(|e| e.sym.kind == K_SYM).map(name_of).collect();
    assert!(rows.len() > 800);
    check(&rows, |name| valid(&working_form(name)?));
}

#[test]
fn every_symbol_renders_inside_a_formula() {
    let rows: Vec<&str> = ENTRIES.iter().filter(|e| e.sym.kind == K_SYM).map(name_of).collect();
    check(&rows, |name| {
        let form = working_form(name)?;
        valid(&format!("a{form} b^{{{form}}}_{{{form}}}"))
    });
}

#[test]
fn every_command_gives_output_or_an_error_for_every_argument_shape() {
    let shapes = ["\\N", "\\N{x}", "\\N{x}{y}", "\\N{x}{y}{z}", "\\N x", "\\N[2]{x}", "\\N{}", "\\N{red}{x}", "\\N{1em}", "x\\N", "{\\N}", "\\N\\N", "\\N^2", "\\N_2"];
    for e in &ENTRIES {
        for shape in shapes {
            let s = shape.replace("N", name_of(e));
            for rendered in [inline(&s), display(&s)] {
                match rendered {
                    Ok(svg) => well_formed(&svg).unwrap_or_else(|why| panic!("{s}: {why}")),
                    Err((message, at)) => {
                        assert!(!message.is_empty(), "{s}");
                        assert!(at <= s.len(), "{s}: position {at}");
                    }
                }
            }
        }
    }
}

#[test]
fn almost_every_command_has_a_working_form() {
    let shapes = ["\\N", "\\N{x}", "\\N{x}{y}", "\\N{x}{y}{z}", "\\N x", "\\N{red}{x}", "\\N{1em}", "\\N{1em}{1em}", "\\N(", "\\left(x\\N", "\\left(x\\N|\\right)", "\\N[1]{x}", "\\N{x}[1]", "\\begin{matrix}a\\N b\\end{matrix}", "\\begin{array}{c}\\N a\\end{array}", "\\N 3", "N", "\\text{\\N}", "\\text{N}", "\\text{\\N a}"];
    let mut dead = Vec::new();
    for e in &ENTRIES {
        let name = name_of(e);
        if !shapes.iter().any(|shape| inline(&shape.replace("N", name)).is_ok()) {
            dead.push(name);
        }
    }
    assert!(dead.is_empty(), "{dead:?}");
}

#[test]
fn every_accent_sits_above_the_letter() {
    let plain = measure("x");
    let rows: Vec<&str> = ENTRIES.iter().filter(|e| e.sym.kind == K_ACCENT && name_of(e).chars().all(|c| c.is_ascii_alphabetic())).map(name_of).collect();
    assert!(rows.len() >= 10);
    check(&rows, |name| {
        let form = format!("\\{name}{{x}}");
        let Ok(svg) = inline(&form) else { return Ok(()) };
        let got = size_of(&svg);
        if got.height > plain.height { Ok(()) } else { Err(format!("{got:?}")) }
    });
}

#[test]
fn every_accent_glyph_is_drawn_high() {
    let rows: Vec<&Entry> = ENTRIES.iter().filter(|e| e.sym.kind == K_ACCENT).collect();
    let names: Vec<&str> = rows.iter().map(|e| name_of(e)).collect();
    for (e, name) in rows.iter().zip(&names) {
        let g = &GLYPHS[e.sym.glyph as usize];
        assert!(g.sy <= -430, "{name} sits at {}", g.sy);
    }
}
