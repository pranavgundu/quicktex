use super::*;

struct Pdf {
    bytes: Vec<u8>,
}

impl Pdf {
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.bytes).into_owned()
    }

    fn find_last(&self, needle: &[u8]) -> usize {
        self.bytes.windows(needle.len()).rposition(|w| w == needle).unwrap()
    }

    fn offsets(&self) -> Vec<usize> {
        let start = self.find_last(b"\nxref\n") + 1;
        let text = String::from_utf8_lossy(&self.bytes[start..]).into_owned();
        let mut lines = text.lines();
        lines.next();
        let header = lines.next().unwrap();
        let count: usize = header.split(' ').nth(1).unwrap().parse().unwrap();
        lines.take(count).skip(1).map(|l| l[..10].parse().unwrap()).collect()
    }
}

fn pdf_of(s: &str) -> Pdf {
    Pdf { bytes: good(s).pdf() }
}

fn number_after(text: &str, key: &str) -> usize {
    let at = text.find(key).unwrap_or_else(|| panic!("no {key}")) + key.len();
    text[at..].trim_start().chars().take_while(char::is_ascii_digit).collect::<String>().parse().unwrap()
}

#[test]
fn starts_and_ends_like_a_pdf() {
    let p = pdf_of(&article("x"));
    assert!(p.bytes.starts_with(b"%PDF-1.4\n"));
    assert!(p.text().trim_end().ends_with("%%EOF"));
}

#[test]
fn startxref_points_at_the_table() {
    for s in [LAB, SHAPES, SAMPLE] {
        let p = pdf_of(s);
        let text = p.text();
        let at = number_after(&text[text.rfind("startxref").unwrap()..], "startxref");
        assert!(p.bytes[at..].starts_with(b"xref\n"));
    }
}

#[test]
fn every_xref_offset_points_at_its_object() {
    for s in [LAB, SHAPES, SAMPLE] {
        let p = pdf_of(s);
        let offsets = p.offsets();
        assert!(offsets.len() > 10);
        for (i, at) in offsets.iter().enumerate() {
            let want = format!("{} 0 obj", i + 1);
            assert!(p.bytes[*at..].starts_with(want.as_bytes()), "object {}", i + 1);
        }
    }
}

#[test]
fn trailer_size_matches_the_table() {
    let p = pdf_of(LAB);
    let text = p.text();
    let size = number_after(&text[text.rfind("trailer").unwrap()..], "/Size");
    assert_eq!(size, p.offsets().len() + 1);
}

#[test]
fn every_stream_length_is_exact() {
    for s in [LAB, SHAPES, SAMPLE] {
        let p = pdf_of(s);
        let mut at = 0;
        let mut seen = 0;
        while let Some(found) = p.bytes[at..].windows(7).position(|w| w == b"stream\n") {
            let start = at + found + 7;
            if p.bytes[..start].ends_with(b"endstream\n") {
                at = start;
                continue;
            }
            let head = String::from_utf8_lossy(&p.bytes[start.saturating_sub(120)..start]).into_owned();
            let length = number_after(&head[head.rfind("/Length").unwrap()..], "/Length");
            assert!(p.bytes[start + length..].starts_with(b"\nendstream") || p.bytes[start + length..].starts_with(b"endstream"));
            at = start + length;
            seen += 1;
        }
        assert!(seen > 5);
    }
}

#[test]
fn page_count_matches() {
    for (s, pages) in [(article("x"), 1), (article("a\\newpage b\\newpage c"), 3), (LAB.to_string(), 2)] {
        let text = pdf_of(&s).text();
        assert_eq!(number_after(&text, "/Count"), pages);
        assert_eq!(
            text.matches("/Type/Page/").count()
                + text.matches("/Type /Page\n").count()
                + text.matches("/Type/Page ").count()
                + text.matches("/Type /Page ").count()
                + text.matches("/Type /Page/").count(),
            pages
        );
    }
}

#[test]
fn media_box_is_the_page() {
    assert!(
        pdf_of(&article("x")).text().contains("/MediaBox [0 0 612 792]") || pdf_of(&article("x")).text().contains("/MediaBox[0 0 612 792]")
    );
}

#[test]
fn a4_media_box_follows_the_option() {
    let t = pdf_of("\\documentclass[a4paper]{article}\\begin{document}x\\end{document}").text();
    assert!(t.contains("595") && t.contains("842"));
}

#[test]
fn glyphs_are_shared_between_pages() {
    let one = pdf_of(&article("a")).bytes.len();
    let many = pdf_of(&article(&"a ".repeat(2000))).bytes.len();
    assert!(many < one + 40000 + 2000 * 40, "{one} {many}");
}

#[test]
fn drawings_reach_the_pdf() {
    let a = pdf_of(&with("\\usepackage{tikz}", "\\begin{tikzpicture}\\draw (0,0) -- (1,0);\\end{tikzpicture}")).bytes.len();
    let b = pdf_of(&with("\\usepackage{tikz}", "\\begin{tikzpicture}\\draw (0,0) -- (1,0);\\draw (0,1) -- (1,1);\\end{tikzpicture}"))
        .bytes
        .len();
    assert!(b > a);
}

#[test]
fn colors_reach_the_pdf() {
    let t = pdf_of(&with("\\usepackage{tikz}", "\\begin{tikzpicture}\\draw[red] (0,0) -- (1,0);\\end{tikzpicture}")).text();
    assert!(t.contains("1 0 0 RG") || t.contains("1.000 0.000 0.000 RG") || t.contains("1 0 0 rg"), "no red");
}

#[test]
fn pdf_is_repeatable() {
    let mut d = good(LAB);
    let a = d.pdf();
    let b = d.pdf();
    assert_eq!(a, b);
}

#[test]
fn pdf_after_a_failed_compile_is_the_last_good_one() {
    let mut e = Engine::new();
    run(&mut e, LAB).unwrap();
    let size = e.document_pdf() as usize;
    let a = unsafe { core::slice::from_raw_parts(e.output_ptr(), size) }.to_vec();
    assert!(run(&mut e, "\\begin{document}{").is_err());
    let size = e.document_pdf() as usize;
    let b = unsafe { core::slice::from_raw_parts(e.output_ptr(), size) }.to_vec();
    assert_eq!(a, b);
}

#[test]
fn pages_still_work_after_a_pdf() {
    let mut d = good(LAB);
    let a = d.page(0);
    d.pdf();
    assert_eq!(d.page(0), a);
}

#[test]
fn page_size_scales_with_the_font_option() {
    let t = pdf_of("\\documentclass[12pt]{article}\\begin{document}x\\end{document}").text();
    assert!(t.contains("612") && t.contains("792"));
}

#[test]
fn href_and_url_create_pdf_link_annotations() {
    let p = pdf_of(
        r"\documentclass{article}\begin{document}\href{https://example.com/a(b)}{A styled \textbf{link}} and \url{https://example.org}\end{document}",
    );
    let text = p.text();
    assert!(text.contains("/Subtype /Link"));
    assert!(text.contains("/URI <68747470733A2F2F6578616D706C652E636F6D2F61286229>"));
    assert!(text.contains("/URI <68747470733A2F2F6578616D706C652E6F7267>"));
    assert!(text.contains("/Border [0 0 0] /Rect ["));
}

#[test]
fn wrapped_links_get_rectangles_on_each_line_and_page() {
    let words = "linked words ".repeat(150);
    let source = format!(
        r"\documentclass{{article}}\usepackage[left=220pt,right=220pt,top=330pt,bottom=330pt]{{geometry}}\begin{{document}}\href{{https://example.com}}{{{words}}}\end{{document}}"
    );
    let mut d = good(&source);
    assert!(d.pages > 1);
    let page_count = d.pages;
    let text = String::from_utf8_lossy(&d.pdf()).into_owned();
    assert_eq!(text.matches("/Annots [").count(), page_count);
    assert!(text.matches("/Subtype /Link").count() > page_count);
    for rect in text.split("/Rect [").skip(1) {
        let rect = rect.split(']').next().unwrap();
        let coords: Vec<f64> = rect.split_whitespace().map(|s| s.parse().unwrap()).collect();
        assert_eq!(coords.len(), 4);
        assert!(coords[0] >= 0. && coords[0] < coords[2] && coords[2] <= 612.);
        assert!(coords[1] >= 0. && coords[1] < coords[3] && coords[3] <= 792.);
    }
}

#[test]
fn nolinkurl_and_plain_text_create_no_annotations() {
    let p = pdf_of(r"\documentclass{article}\begin{document}\nolinkurl{https://example.com} plain text\end{document}");
    assert!(!p.text().contains("/Annots"));
}

#[test]
fn pdf_links_escape_tex_uri_characters_and_encode_unicode() {
    let p = pdf_of(r"\documentclass{article}\begin{document}\href{https://example.com/café?x=1\&y=2\#part}{linked} plain\end{document}");
    let text = p.text();
    let uri = "https://example.com/caf%C3%A9?x=1&y=2#part";
    let hex: String = uri.bytes().map(|b| format!("{b:02X}")).collect();
    assert!(text.contains(&format!("/URI <{hex}>")));
    assert!(text.contains("/Annots ["));
    let annotations = text.split("/Annots [").nth(1).unwrap().split(']').next().unwrap();
    assert!(!annotations.contains("<<"), "annotation dictionaries must be indirect");
    let d = good(r"\documentclass{article}\begin{document}\href{https://example.com}{linked} plain\end{document}");
    let glyph_links: Vec<u32> = d.raw_items(0).iter().filter(|i| i.kind == IT_GLYPH).map(|i| d.laid().groups[i.g as usize].link).collect();
    assert!(glyph_links[..6].iter().all(|id| *id != 0));
    assert!(glyph_links[6..].iter().all(|id| *id == 0));
}

#[test]
fn pdf_integer_writer_preserves_decimal_boundaries_and_extremes() {
    let mut values = vec![i32::MIN, i32::MIN + 1, i32::MAX - 1, i32::MAX];
    let mut power = 1i32;
    for _ in 0..10 {
        for delta in -1..=1 {
            if let Some(value) = power.checked_add(delta) {
                values.push(value);
                values.push(-value);
            }
        }
        if let Some(next) = power.checked_mul(10) {
            power = next;
        }
    }
    // Every low digit pair, including interior zeros and transitions between
    // one- and two-digit chunks, must retain exactly the original PDF bytes.
    values.extend(-10_000..=10_000);
    let mut random = Rng(0xd28e_4a71_95fb_6043);
    values.extend((0..10_000).map(|_| random.next() as i32));
    let mut actual = Vec::new();
    for value in values {
        actual.clear();
        actual.extend_from_slice(b"prefix ");
        crate::tex::pdf::int(&mut actual, value);
        assert_eq!(actual, format!("prefix {value}").as_bytes(), "{value}");
    }
}
