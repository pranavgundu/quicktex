use super::*;

#[test]
fn document_builtin_counters_drive_numbering() {
    let d = good(
        r"\documentclass{article}\setcounter{section}{4}\setcounter{equation}{6}\begin{document}\section{Next}\subsection{Child}\begin{equation}x=1\end{equation}\end{document}",
    );
    assert_eq!(d.e.counter_value(b"section"), 5);
    assert_eq!(d.e.counter_value(b"subsection"), 1);
    assert_eq!(d.e.counter_value(b"equation"), 7);
    assert!(d.e.warned.is_empty(), "{:?}", d.e.warned);
}

#[test]
fn custom_counters_and_loops_work_in_document_paragraphs() {
    let d =
        good(r"\documentclass{article}\newcount\i\begin{document}Start \i=0\loop\the\i\advance\i by1\ifnum\i<4\repeat end.\end{document}");
    assert_eq!(d.e.counter_value(b"i"), 4);
    assert!(d.e.warned.is_empty(), "{:?}", d.e.warned);
}

#[test]
fn setcounter_page_changes_printed_page_number() {
    let d = good(r"\documentclass{article}\setcounter{page}{8}\begin{document}One.\newpage Two.\end{document}");
    assert_eq!(d.pages, 2);
    assert_eq!(d.e.counter_value(b"page"), 9);
}

#[test]
fn secnumdepth_suppresses_numbering_and_section_steps_reset_children() {
    let d = good(
        r"\documentclass{article}\begin{document}\section{One}\subsection{One}\section{Two}\setcounter{secnumdepth}{0}\section{Unnumbered}\end{document}",
    );
    assert_eq!(d.e.counter_value(b"section"), 2);
    assert_eq!(d.e.counter_value(b"subsection"), 0);
}
