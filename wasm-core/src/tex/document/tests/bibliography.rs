use super::*;

fn project(e: &mut Engine, name: &[u8], bytes: &[u8]) {
    e.assets.files.push((name.to_vec(), bytes.to_vec()));
}
const BIB: &[u8] = br#"@article{doe, author={Jane Doe and John Roe}, title={A {Nested} Title}, journal="Journal", year=2024}
@book{smith, author={Alice Smith}, title={The Book}, publisher={Press}, year=2023}"#;

#[test]
fn bibliography_assets_render_references_and_natbib_without_warnings() {
    let mut e = Engine::new();
    project(&mut e, b"refs.bib", BIB);
    let source = with(
        r"\usepackage[numbers]{natbib}",
        r"\citet{doe} and \citep[see][p.~4]{smith,doe}. \citeauthor*{doe}, \citeyear{smith}. \bibliographystyle{plainnat}\bibliography{refs}",
    );
    assert!(run(&mut e, &source).unwrap() > 0);
    assert!(e.warned.is_empty(), "{:?}", e.warned);
    assert!(e.label_bytes.windows(b"cite:doe".len()).any(|w| w == b"cite:doe"));
    assert!(e.label_bytes.windows(b"cite:smith".len()).any(|w| w == b"cite:smith"));
}

#[test]
fn included_tex_can_request_nested_bibliography_assets() {
    let mut e = Engine::new();
    project(&mut e, b"data/refs.bib", BIB);
    project(&mut e, b"chapter.tex", br"\citep{doe}\bibliography{data/refs}");
    run(&mut e, &article(r"\input{chapter}")).unwrap();
    assert!(e.label_bytes.windows(b"cite:doe".len()).any(|w| w == b"cite:doe"));
    assert!(e.warned.is_empty(), "warnings: {:?}", e.warned);
}

#[test]
fn missing_files_and_unknown_styles_report_actionable_errors() {
    let missing = compile(&article(r"\bibliography{missing}")).err().unwrap();
    assert!(missing.0.contains("missing") && missing.0.contains("project files"));
    let style = compile(&article(r"\bibliographystyle{custom}")).err().unwrap();
    assert!(style.0.contains("unsupported bibliography style 'custom'"));
}

#[test]
fn nocite_all_and_biblatex_resource_commands_generate_entries() {
    let mut e = Engine::new();
    project(&mut e, b"refs.bib", BIB);
    run(&mut e, &with(r"\addbibresource{refs.bib}", r"\nocite{*}\printbibliography")).unwrap();
    assert!(e.label_bytes.windows(b"cite:doe".len()).any(|w| w == b"cite:doe"));
    assert!(e.label_bytes.windows(b"cite:smith".len()).any(|w| w == b"cite:smith"));
    assert!(e.warned.is_empty(), "warnings: {:?}", e.warned);
}

#[test]
fn bibliography_state_is_cleared_between_documents() {
    let mut e = Engine::new();
    project(&mut e, b"refs.bib", BIB);
    run(&mut e, &article(r"\cite{doe}\bibliography{refs}")).unwrap();
    e.clear_files();
    run(&mut e, &article(r"\begin{thebibliography}{9}\bibitem{x}Manual reference\end{thebibliography}\cite{x}")).unwrap();
    assert!(e.warned.is_empty(), "warnings: {:?}", e.warned);
}

#[test]
fn unused_macro_citations_do_not_create_reference_entries() {
    let mut e = Engine::new();
    project(&mut e, b"refs.bib", BIB);
    run(&mut e, &with(r"\newcommand{\unused}{\cite{smith}}", r"\cite{doe}\bibliography{refs}")).unwrap();
    assert!(e.label_bytes.windows(8).any(|w| w == b"cite:doe"));
    assert!(!e.label_bytes.windows(10).any(|w| w == b"cite:smith"));
    assert!(e.warned.is_empty());
}

#[test]
fn uncollected_dynamic_citations_and_conditionals_warn() {
    let mut e = Engine::new();
    project(&mut e, b"refs.bib", BIB);
    run(&mut e, &with(r"\newcommand{\dynamiccite}{\cite{doe}}", r"\dynamiccite\bibliography{refs}")).unwrap();
    assert!(!e.warned.is_empty());
    run(&mut e, &article(r"\iffalse\cite{smith}\fi\cite{doe}\bibliography{refs}")).unwrap();
    assert!(!e.warned.is_empty());
}

#[test]
fn bibliography_commands_in_comments_and_verbatim_are_not_resources() {
    let mut e = Engine::new();
    run(
        &mut e,
        &article("%\\bibliography{absent}\n\\verb|\\bibliography{absent}|\n\\begin{verbatim}\\bibliography{absent}\\end{verbatim}"),
    )
    .unwrap();
    assert!(e.warned.is_empty());
}

#[test]
fn resources_after_document_end_and_in_false_branches_are_ignored() {
    compile(&format!("{}\\bibliography{{missing}}", article("text"))).unwrap();
    compile(&article(r"\iffalse\bibliography{missing}\fi text")).unwrap();
}

#[test]
fn unsupported_crossref_inheritance_is_diagnosed() {
    let mut e = Engine::new();
    project(&mut e, b"refs.bib", br"@inproceedings{a, title={Paper}, crossref={proceedings}}");
    let error = run(&mut e, &article(r"\cite{a}\bibliography{refs}")).unwrap_err();
    assert!(error.0.contains("inheritance is unsupported"));
}
