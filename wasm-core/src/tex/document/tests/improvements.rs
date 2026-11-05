use super::*;

#[test]
fn footnotes_are_at_the_bottom_and_leave_body_room() {
    let d = body("Before\\footnote{Bottom note.}\n\nAfter.");
    let ys = d.baselines(0);
    assert!(ys.iter().any(|y| *y > 65000), "{ys:?}");
    assert!(ys.iter().filter(|y| **y < 20000).count() >= 2);
    let separator = d.items(0).iter().find(|it| it.kind == IT_RULE).unwrap();
    assert!(separator.y > 64000);
    assert!(d.items(0).iter().filter(|it| it.kind == IT_GLYPH).all(|it| it.y < 68000));
}
#[test]
fn notes_stay_in_the_column_of_the_marker() {
    let source = format!("{}\\footnote{{First note}}\n\n{}\\footnote{{Second note}}", words(200), words(1800));
    let d = body(&source);
    assert!(d.pages > 1);
    assert_eq!(d.all_items().iter().filter(|it| it.kind == IT_RULE).count(), 2);
    assert!(d.items(0).iter().any(|it| it.kind == IT_RULE));
}
#[test]
fn paragraph_columns_wrap_and_push_the_next_row_down() {
    let d = body("\\begin{tabular}{p{1cm}l}alpha beta gamma delta&x\\\\next&y\\end{tabular}");
    assert!(d.baselines(0).len() >= 5);
    let first = d.items(0).iter().find(|it| it.b == b'a' as i32).unwrap();
    let x = d.items(0).iter().find(|it| it.b == b'x' as i32).unwrap();
    assert!(x.x - first.x < 5000);
    assert_eq!(d.e.warned.len(), 0);
}
#[test]
fn x_columns_can_shrink_to_fit_the_requested_width() {
    let d = body("\\begin{tabularx}{4cm}{lX}A&alpha beta gamma delta epsilon zeta eta theta\\end{tabularx}");
    assert!(d.baselines(0).len() > 1);
    assert!(d.right_edge(0) - d.left_edge(0) < 12000);
}
#[test]
fn top_and_bottom_float_placement_is_observable() {
    let top = body("Before.\\begin{figure}[t]Top.\\end{figure}After.");
    assert_eq!(top.pages, 2);
    assert!(top.items(0).iter().any(|it| it.b == b'A' as i32));
    assert!(top.items(1).iter().any(|it| it.b == b'T' as i32));
    let bottom = body("Before.\\begin{figure}[b]Bottom.\\end{figure}After.");
    assert_eq!(bottom.pages, 1);
    let b = bottom.items(0).iter().find(|it| it.b == b'B' as i32 && it.y > 30000).unwrap();
    assert!(b.y > 60000);
}
#[test]
fn supported_legacy_drawings_produce_geometry() {
    for source in [
        "\\begin{picture}(100,40)\\put(0,0){\\line(1,0){100}}\\put(50,20){\\circle{20}}\\end{picture}",
        "\\begin{pspicture}(0,0)(2,2)\\psline(0,0)(1,1)\\pscircle(1,1){0.5}\\end{pspicture}",
        "\\begin{forest}[Root [Left] [Right]]\\end{forest}",
        "\\chemfig{H_3C-CH_2-OH}",
        "\\begin{tikzpicture}\\tkzDefPoint(0,0){A}\\tkzDefPoint(2,0){B}\\tkzDrawSegment(A,B)\\tkzLabelPoint(A){A}\\end{tikzpicture}",
        "\\begin{tikzpicture}\\tkzDefPoints{0/0/A, 2/0/B, 1/1/C}\\tkzDrawPolygon(A,B,C)\\tkzDrawPoints(A,B,C)\\end{tikzpicture}",
    ] {
        let d = body(source);
        assert!(!d.all_art().is_empty(), "{source}");
        assert_eq!(d.e.warned.len(), 0, "{source}");
    }
}
#[test]
fn unknown_legacy_commands_still_warn() {
    let d = body("\\begin{picture}(1,1)\\unknownthing{abc}\\end{picture}");
    assert!(!d.e.warned.is_empty());
}
#[test]
fn english_hyphenation_avoids_an_overfull_long_word() {
    let d = good("\\documentclass{article}\\geometry{left=270pt,right=270pt}\\begin{document}extraordinary hyphenation\\end{document}");
    assert!(d.baselines(0).len() > 2);
    assert!(d.right_edge(0) - d.left_edge(0) < 7200);
}

#[test]
fn cjk_cells_wrap_without_synthetic_spaces_or_negative_offsets() {
    let mut engine = Engine::new();
    for cp in [0x4e2du32, 0x6587] {
        let mut bytes = Vec::new();
        for value in [cp as i32, 1000, 700, 0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(b"m0 -700h800v700h-800z");
        let pointer = engine.input_buffer(bytes.len());
        unsafe {
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer, bytes.len());
        }
        assert_eq!(engine.register_glyph(bytes.len()), 0);
    }
    run(&mut engine, &article("\\begin{tabular}{p{1cm}}中文中文中文中文\\end{tabular}")).unwrap();
    let letters: Vec<_> = engine.laid.items.iter().filter(|it| it.kind == IT_GLYPH && it.b > 127).collect();
    assert_eq!(letters.len(), 8);
    let low = letters.iter().map(|it| it.x).min().unwrap();
    let high = letters.iter().map(|it| it.x).max().unwrap();
    assert_eq!(high - low, 1000);
    let mut lines: Vec<_> = letters.iter().map(|it| it.y).collect();
    lines.sort();
    lines.dedup();
    assert_eq!(lines.len(), 4);
}
