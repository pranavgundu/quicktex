use super::*;
use crate::tex::*;

#[test]
fn cached_ascii_metrics_match_uncached_font_metrics_for_every_key() {
    let mut engine = Engine::new();
    assert_eq!(engine.text_metric(0, 0, 0).map(|metric| metric.glyph), glyph_id(0, 0));
    for pt in [10, 11, 12, 10] {
        engine.doc.pt = pt;
        for scale in [1000, 700, 1440, 500, 1000] {
            for font in [FONT_MAIN_REGULAR, FONT_MAIN_BOLD, FONT_MAIN_ITALIC, FONT_TYPEWRITER_REGULAR] {
                for cp in 32..127 {
                    let glyph = glyph_id(font, cp).or_else(|| glyph_id(FONT_MAIN_REGULAR, cp));
                    let Some(glyph) = glyph else {
                        continue;
                    };
                    let original = engine.glyph_metric(glyph as usize);
                    let natural = sc(original.w as i32, scale);
                    let width = engine.advance(font, cp, natural, scale);
                    for _ in 0..2 {
                        let cached = engine.text_metric(font, cp, scale).unwrap();
                        assert_eq!(
                            (cached.glyph, cached.natural, cached.width, cached.h, cached.d),
                            (glyph, natural, width, sc(original.h as i32, scale), sc(original.d as i32, scale))
                        );
                    }
                }
            }
        }
    }
    assert_eq!(engine.text_metrics.len(), TEXT_METRIC_SLOTS);
}

#[test]
fn cached_metrics_and_line_scratch_preserve_pdf_output_across_edits_and_point_sizes() {
    let mut warm = Engine::new();
    for pt in [10, 11, 12, 10] {
        for word in ["first", "edited", "first"] {
            let source = format!(
                r"\documentclass[{pt}pt]{{article}}\begin{{document}}\section{{Heading}}
                AVATAR ffi office \textit{{italic words}} \textbf{{bold}} \texttt{{typewriter}} {word}.
                \par \small A different scale with AV kerning. \normalsize Restored.
                \end{{document}}"
            );
            let mut cold = Engine::new();
            for engine in [&mut warm, &mut cold] {
                let len = feed(engine, &source);
                assert!(engine.render_document(len) > 0);
                assert!(engine.document_pdf() > 0);
            }
            assert_eq!(warm.out, cold.out);
        }
    }
}
