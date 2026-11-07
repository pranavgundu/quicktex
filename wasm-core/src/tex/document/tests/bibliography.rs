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
    let missing = compile(&article(r"\cite{needed}\bibliography{missing}")).err().unwrap();
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
fn unused_missing_database_warns_without_inventing_a_bibliography() {
    let mut d = good(&article(r"Text\bibliography{missing}"));
    assert!(!d.e.warned.is_empty());
    assert_eq!(d.glyphs(), 4);
    assert!(!d.page(0).is_empty());
    assert!(compile(&article(r"\nocite{*}\bibliography{missing}")).is_err());
    assert!(compile(&with(r"\newcommand{\request}{\cite{needed}}", r"\request\bibliography{missing}")).is_err());
}

#[test]
fn missing_crossref_parent_is_diagnosed() {
    let mut e = Engine::new();
    project(&mut e, b"refs.bib", br"@inproceedings{a, title={Paper}, crossref={proceedings}}");
    let error = run(&mut e, &article(r"\cite{a}\bibliography{refs}")).unwrap_err();
    assert!(error.0.contains("crossref parent was not found"));
}

#[test]
fn unused_inherited_entries_do_not_block_independent_references() {
    let mut e = Engine::new();
    project(
        &mut e,
        b"refs.bib",
        br"@book{used, author={Jane Doe}, title={Independent}, year=2024}
        @inproceedings{unused, title={Paper}, crossref={proceedings}}",
    );
    run(&mut e, &article(r"\cite{used}\bibliography{refs}")).unwrap();
    assert!(e.warned.is_empty());
    assert!(e.label_bytes.windows(9).any(|w| w == b"cite:used"));
    for request in [r"\nocite{unused}", r"\nocite{*}"] {
        let error = run(&mut e, &article(&format!("{request}\\bibliography{{refs}}"))).unwrap_err();
        assert!(error.0.contains("crossref parent was not found"));
    }
}

#[test]
fn bibliography_and_included_sources_share_normalized_paths() {
    let mut e = Engine::new();
    project(&mut e, b"refs/db.bib", BIB);
    project(&mut e, b"chapters/one.tex", br"\cite{doe}\bibliography{refs/../refs/./db}");
    run(&mut e, &article(r"\input{./chapters/../chapters/one}\bibliography{./refs/db.bib}")).unwrap();
    assert!(e.warned.is_empty(), "{:?}", e.warned);
    assert!(e.label_bytes.windows(8).any(|w| w == b"cite:doe"));
}

#[test]
fn biblatex_url_counters_enable_breaks_without_inserted_hyphens() {
    for (counter, url) in [
        ("biburllcpenalty", "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz"),
        ("biburlucpenalty", "ABCDEFGHIJKLMNOPQRSTUVWXYZABCDEFGHIJKLMNOPQRSTUVWXYZ"),
        ("biburlnumpenalty", "01234567890123456789012345678901234567890123456789012345"),
    ] {
        let preamble = r"\usepackage[left=277pt,right=277pt]{geometry}\usepackage{biblatex}\setlength{\parindent}{0pt}\biburlsetup";
        let disabled = good(&with(preamble, &format!(r"\url{{{url}}}")));
        assert_eq!(disabled.baselines(0).len(), 1);
        let enabled = good(&with(preamble, &format!(r"\setcounter{{{counter}}}{{7000}}\url{{{url}}}")));
        assert!(enabled.baselines(0).len() > 2);
        assert_eq!(enabled.glyphs(), url.len());
        assert!(enabled.right_edge(0) < enabled.left_edge(0) + 6500);
        let forbidden = good(&with(preamble, &format!(r"\setcounter{{{counter}}}{{10000}}\url{{{url}}}")));
        assert_eq!(forbidden.baselines(0).len(), 1);
    }
}

#[test]
fn biblatex_url_counter_lifetime_and_defaults_follow_package_loading() {
    let mut e = Engine::new();
    run(&mut e, &with(r"\usepackage{biblatex}", "Text")).unwrap();
    for (name, value) in [
        (b"biburlbigbreakpenalty".as_slice(), 100),
        (b"biburlbreakpenalty", 200),
        (b"biburllcpenalty", 0),
        (b"biburlucpenalty", 0),
        (b"biburlnumpenalty", 0),
    ] {
        assert_eq!(e.counter_value(name), value);
    }
    run(&mut e, &with(r"\usepackage{biblatex}\setcounter{biburllcpenalty}{7000}\usepackage{biblatex}", "Text")).unwrap();
    assert_eq!(e.counter_value(b"biburllcpenalty"), 7000);
    assert!(run(&mut e, &article(r"\setcounter{biburllcpenalty}{7000}")).is_err());
}

#[test]
fn biblatex_punctuation_break_penalties_control_url_wrapping() {
    let preamble = r"\usepackage[left=277pt,right=277pt]{geometry}\usepackage{biblatex}\setlength{\parindent}{0pt}\biburlsetup";
    for (counter, separator) in [("biburlbreakpenalty", "."), ("biburlbigbreakpenalty", ":")] {
        let url = ["zzzz"; 8].join(separator);
        let ordinary = good(&with(preamble, &format!(r"\url{{{url}}}")));
        assert!(ordinary.baselines(0).len() > 2);
        assert_eq!(ordinary.glyphs(), url.len());
        let prohibited = good(&with(preamble, &format!(r"\setcounter{{{counter}}}{{10000}}\url{{{url}}}")));
        assert_eq!(prohibited.baselines(0).len(), 1);
    }
}

#[test]
fn biblatex_url_setup_is_local_and_not_automatic_in_body_text() {
    let setup =
        r"\usepackage[left=277pt,right=277pt]{geometry}\usepackage{biblatex}\setlength{\parindent}{0pt}\setcounter{biburllcpenalty}{7000}";
    let url = "ABCDEFGHIJKLMNOPQRSTUVWXYZABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let enabled = r"\setcounter{biburlucpenalty}{7000}";
    let ordinary = good(&with(setup, &format!(r"{enabled}\url{{{url}}}")));
    assert_eq!(ordinary.baselines(0).len(), 1);
    let scoped = good(&with(setup, &format!(r"{enabled}{{\biburlsetup\url{{{url}}}}}\par\url{{{url}}}")));
    let lines = scoped.baselines(0).len();
    let active = good(&with(setup, &format!(r"{enabled}\biburlsetup\url{{{url}}}")));
    assert_eq!(lines, active.baselines(0).len() + 1);
}
