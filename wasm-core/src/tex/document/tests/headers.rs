use super::*;

fn glyphs(text: &str) -> Vec<i32> {
    body(text).items(0).iter().filter(|i| i.kind == IT_GLYPH).map(|i| i.a).collect()
}

fn row(d: &Done, page: usize, y: i32) -> Vec<i32> {
    let mut items: Vec<_> = d.raw_items(page).iter().filter(|i| i.kind == IT_GLYPH && i.y == y).collect();
    items.sort_by_key(|i| i.x);
    items.iter().map(|i| i.a).collect()
}

#[test]
fn fancy_header_and_footer_render_at_the_page_edges() {
    let mut d = good(&with(
        r"\usepackage{fancyhdr}\pagestyle{fancy}\fancyhf{}\fancyhead[L]{HEADER}\fancyfoot[R]{Page \thepage}",
        "Body\\newpage Body",
    ));
    assert!(d.e.warned.is_empty(), "{:?}", d.e.warned);
    for page in 0..2 {
        let ys: Vec<_> = d.raw_items(page).iter().filter(|i| i.kind == IT_GLYPH).map(|i| i.y).collect();
        let top = *ys.iter().min().unwrap();
        let bottom = *ys.iter().max().unwrap();
        assert_eq!(row(&d, page, top), glyphs("HEADER"));
        assert_eq!(row(&d, page, bottom), glyphs(&format!("Page {}", page + 1)));
        let head = d.raw_items(page).iter().find(|i| i.kind == IT_GLYPH && i.y == top).unwrap();
        let foot = d.raw_items(page).iter().find(|i| i.kind == IT_GLYPH && i.y == bottom).unwrap();
        assert!(head.x < foot.x);
        assert!(top < 12500 && bottom > 67500);
        well_formed(&d.page(page)).unwrap();
    }
}

#[test]
fn odd_even_headers_and_direct_field_commands_work() {
    let d = good(&with(r"\usepackage{fancyhdr}\pagestyle{fancy}\fancyhf{}\lhead[EVEN]{ODD}\rfoot{END}", "Body\\newpage Body"));
    for page in 0..2 {
        let top = d.raw_items(page).iter().filter(|i| i.kind == IT_GLYPH).map(|i| i.y).min().unwrap();
        assert_eq!(row(&d, page, top), glyphs(if page == 0 { "ODD" } else { "EVEN" }));
    }
}

#[test]
fn thispagestyle_empty_applies_to_one_page() {
    let d = good(&with(r"\pagestyle{fancy}\fancyhf{}\chead{HEAD}", "\\thispagestyle{empty}Body\\newpage Body"));
    assert_eq!(d.raw_items(0).iter().filter(|i| i.kind == IT_GLYPH).map(|i| i.a).collect::<Vec<_>>(), glyphs("Body"));
    let top = d.raw_items(1).iter().filter(|i| i.kind == IT_GLYPH).map(|i| i.y).min().unwrap();
    assert_eq!(row(&d, 1, top), glyphs("HEAD"));
}

#[test]
fn empty_pagestyle_suppresses_default_page_numbers() {
    let d = good(&with(r"\pagestyle{empty}", "Body"));
    assert_eq!(d.raw_items(0).iter().filter(|i| i.kind == IT_GLYPH).map(|i| i.a).collect::<Vec<_>>(), glyphs("Body"));
}

#[test]
fn fancy_rule_width_can_be_disabled() {
    let d = good(&with(r"\pagestyle{fancy}\fancyhf{}\lhead{HEAD}\renewcommand{\headrulewidth}{0pt}", "Body"));
    assert_eq!(d.raw_items(0).iter().filter(|i| i.kind == IT_RULE).count(), 0);
}

#[test]
fn fancy_headers_preserve_body_text_through_automatic_page_breaks() {
    let text = words(2500);
    let plain = body(&text);
    let d = good(&with(r"\pagestyle{fancy}\fancyhf{}\lhead{HEAD}", &text));
    assert_eq!(d.pages, plain.pages);
    for page in 0..d.pages {
        let expected: Vec<_> = plain.items(page).iter().filter(|i| i.kind == IT_GLYPH).map(|i| (i.a, i.x, i.y)).collect();
        let actual: Vec<_> = d.raw_items(page).iter().filter(|i| i.kind == IT_GLYPH && i.y >= 12500).map(|i| (i.a, i.x, i.y)).collect();
        assert_eq!(actual, expected);
    }
}
