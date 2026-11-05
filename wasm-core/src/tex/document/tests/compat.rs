use super::*;

fn clean(s: &str) -> Done {
    let d = good(s);
    assert_eq!(d.e.warned.len(), 0, "{s}");
    d
}

fn clean_body(s: &str) -> Done {
    clean(&article(s))
}

fn warnings(s: &str) -> usize {
    good(&article(s)).e.warned.len()
}

fn xs(d: &Done) -> Vec<i32> {
    d.all_items().iter().filter(|i| i.kind == IT_GLYPH).map(|i| i.x).collect()
}

fn lines(d: &Done) -> Vec<(i32, i32)> {
    let mut rows: Vec<(i32, i32)> = Vec::new();
    for i in d.all_items().iter().filter(|i| i.kind == IT_GLYPH) {
        match rows.iter_mut().find(|row| row.0 == i.y) {
            Some(row) => row.1 = row.1.min(i.x),
            None => rows.push((i.y, i.x)),
        }
    }
    rows
}

const TREE: &str = "\\documentclass[border=10pt]{standalone}
\\usepackage{tikz}
\\definecolor{leaf}{RGB}{20,130,70}
\\newcommand{\\branch}[1]{%
  \\ifnum#1>0
    \\pgfmathtruncatemacro{\\next}{#1-1}
    \\pgfmathsetmacro{\\len}{0.72^(6-#1)}
    \\draw[leaf, line width=0.25pt] (0,0) -- (0,\\len);
    \\begin{scope}[shift={(0,\\len)}]
      \\begin{scope}[rotate=25]\\branch{\\next}\\end{scope}
      \\begin{scope}[rotate=-25]\\branch{\\next}\\end{scope}
    \\end{scope}
  \\fi
}
\\begin{document}
\\begin{tikzpicture}\\branch{6}\\end{tikzpicture}
\\end{document}
";

#[test]
fn recursive_drawing_macros_draw_every_branch() {
    let d = clean(TREE);
    assert_eq!(d.all_art().len(), 63);
    let green = d.all_art()[0].stroke;
    assert_ne!(green, 0);
    assert!(d.all_art().iter().all(|a| a.stroke == green));
}

#[test]
fn standalone_crops_the_page_to_the_drawing() {
    let d = clean(TREE);
    assert!(d.laid().w < 61200 && d.laid().h < 79200);
}

#[test]
fn standalone_keeps_every_point_on_the_page() {
    let d = clean(TREE);
    let (w, h) = (d.laid().w, d.laid().h);
    for a in d.all_art() {
        for (_, x, y) in d.segs(a) {
            assert!((0..=w).contains(&x) && (0..=h).contains(&y), "({x},{y}) outside {w}x{h}");
        }
    }
}

#[test]
fn standalone_fills_the_page_down_to_its_border() {
    let d = clean(TREE);
    let points: Vec<_> = d.all_art().iter().flat_map(|a| d.segs(a)).collect();
    let top = points.iter().map(|p| p.2).min().unwrap();
    let bottom = points.iter().map(|p| p.2).max().unwrap();
    assert_eq!(top, d.laid().h - bottom);
}

#[test]
fn standalone_text_sits_on_the_page_with_its_drawing() {
    let d = clean(
        "\\documentclass{standalone}\\usepackage{tikz}\\begin{document}\\begin{tikzpicture}\\draw (0,0) -- (0,3);\\node at (0,4) {Top};\\end{tikzpicture}\\end{document}",
    );
    let line = d.all_art().iter().flat_map(|a| d.segs(a)).map(|p| p.2).min().unwrap();
    for i in d.all_items().iter().filter(|i| i.kind == IT_GLYPH) {
        assert!(i.y > 0 && i.y < line, "{} above {line}", i.y);
    }
}

#[test]
fn foreach_repeats_the_body() {
    let d = clean_body("\\begin{tikzpicture}\\foreach \\x in {1,2,3}{\\draw (\\x,0) -- (\\x,1);}\\end{tikzpicture}");
    assert_eq!(d.all_art().len(), 3);
}

#[test]
fn foreach_ranges_expand() {
    let d = clean_body("\\begin{tikzpicture}\\foreach \\x in {1,...,5}{\\draw (\\x,0) -- (\\x,1);}\\end{tikzpicture}");
    assert_eq!(d.all_art().len(), 5);
}

#[test]
fn conditionals_pick_one_branch() {
    assert_eq!(clean_body("\\ifnum 1>0 ab\\else abcd\\fi").glyphs(), 2);
    assert_eq!(clean_body("\\ifnum 0>1 ab\\else abcd\\fi").glyphs(), 4);
}

#[test]
fn nested_conditionals_skip_whole_branches() {
    assert_eq!(clean_body("\\ifnum 0>1 \\ifnum 1>0 ab\\fi cd\\else e\\fi").glyphs(), 1);
}

#[test]
fn newif_flags_can_be_set() {
    assert_eq!(clean(&with("\\newif\\ifwide\\widetrue", "\\ifwide ab\\else abc\\fi")).glyphs(), 2);
    assert_eq!(clean(&with("\\newif\\ifwide\\widefalse", "\\ifwide ab\\else abc\\fi")).glyphs(), 3);
}

#[test]
fn macros_in_text_understand_every_text_command() {
    let d = clean_body("\\def\\c#1{\\texttt{\\color{blue}\\footnotesize #1}}\\c{ab} cd");
    assert_eq!(d.glyphs(), 4);
}

#[test]
fn macro_arguments_do_not_fuse_with_a_command_before_them() {
    let d = clean_body("\\newcommand{\\s}[1]{\\small#1}\\s{abc}");
    assert_eq!(d.glyphs(), 3);
}

#[test]
fn macro_arguments_after_plain_letters_stay_joined() {
    let d = clean_body("\\newcommand{\\s}[1]{ab#1}\\s{cd}");
    assert_eq!(xs(&d), xs(&clean_body("abcd")));
}

#[test]
fn macro_text_breaks_across_lines() {
    let d = clean_body(&format!("\\newcommand{{\\s}}{{{}}}\\s", words(80)));
    assert!(lines(&d).len() > 2);
}

#[test]
fn a_bad_paragraph_opening_with_a_macro_is_not_fatal() {
    let s = article("\\newcommand{\\s}{\\frac}\\s\n\nafter");
    let d = good(&s);
    assert!(d.glyphs() >= 5);
}

#[test]
fn macros_with_blocks_inside_table_cells_do_not_crash() {
    let s = "\\newcommand{\\arrow}[1]{\n\n#1}\n\\newcommand{\\row}[2]{\\begin{tabular*}{\\textwidth}{ll}&\\arrow{a} #1\\\\[3pt]&\\arrow{b} #2\\\\\\end{tabular*}}\n\\row{x}{y}";
    good(&article(s));
}

#[test]
fn a_blank_line_in_a_macro_inside_a_cell_keeps_the_cells_before_it() {
    let s = "\\newcommand{\\gap}[1]{\n\n#1}\n\\begin{tabular}{ll}xy & \\gap{z} \\\\\n\\end{tabular}";
    let d = good(&article(s));
    assert_eq!(d.e.warned.len(), 1);
    assert_eq!(d.glyphs(), 2);
}

#[test]
fn wide_tables_close_under_their_own_name() {
    for env in ["tabularx}{\\textwidth", "tabular*}{\\textwidth", "tabulary}{\\linewidth", "xltabular}{\\linewidth"] {
        let name = &env[..env.find('}').unwrap()];
        let spec = if name == "tabulary" { "LL" } else { "ll" };
        let d = clean_body(&format!("\\begin{{{env}}}{{{spec}}}a & b \\\\\n\\end{{{name}}}\nafter"));
        assert_eq!(d.glyphs(), 7, "{name}");
    }
}

#[test]
fn long_tables_keep_the_text_after_them() {
    let d = clean_body("\\begin{longtable}{ll}a & b \\\\\n\\end{longtable}\nafter\n\nmore");
    assert_eq!(d.glyphs(), 11);
}

#[test]
fn tabularx_grows_the_flexible_column_to_the_full_width() {
    let d = body("\\begin{tabularx}{\\textwidth}{lX}a & b \\\\\n\\hline\n\\end{tabularx}");
    let rule = d.all_items().iter().find(|i| i.kind == IT_RULE).unwrap();
    let width = d.laid().w - 2 * (d.all_items()[0].x - 600).max(0);
    assert!(rule.a > 30000, "{} of {width}", rule.a);
}

#[test]
fn starred_tables_push_columns_apart_with_fill() {
    let spec = "l@{\\extracolsep{\\fill}}r";
    let wide = body(&format!("\\begin{{tabular*}}{{\\textwidth}}{{{spec}}}a & b \\\\\n\\end{{tabular*}}"));
    let tight = body("\\begin{tabular}{lr}a & b \\\\\n\\end{tabular}");
    let gap = |d: &Done| xs(d)[1] - xs(d)[0];
    assert!(gap(&wide) > 10 * gap(&tight));
}

#[test]
fn tables_narrower_than_their_contents_do_not_shrink() {
    let spec = "l@{\\extracolsep{\\fill}}r";
    let d = body(&format!("\\begin{{tabular*}}{{1pt}}{{{spec}}}abc & def \\\\\n\\end{{tabular*}}"));
    let x = xs(&d);
    assert!(x.windows(2).all(|w| w[1] > w[0]));
}

#[test]
fn array_specs_accept_groups() {
    assert_eq!(clean_body("$\\begin{array}{@{}r@{}c}a & b \\\\\n\\end{array}$").glyphs(), 2);
    assert_eq!(clean_body("$\\begin{array}{*{3}{c}}a & b & c \\\\\n\\end{array}$").glyphs(), 3);
    assert_eq!(clean_body("$\\begin{array}{p{1cm}c}a & b \\\\\n\\end{array}$").glyphs(), 2);
}

#[test]
fn array_alignment_survives_a_group_in_the_spec() {
    let table = |spec: &str| xs(&body(&format!("$\\begin{{array}}{{{spec}}}aaa & bbb \\\\\nc & d \\\\\n\\end{{array}}$")));
    assert_eq!(table("rr"), table("@{}rr"));
    assert_eq!(table("rr"), table("*{2}{r}"));
    assert_eq!(table("rr"), table("r>{}r"));
    assert_eq!(table("rr"), table(">{\\centering}r@{\\hspace{1cm}}r"));
}

#[test]
fn comments_may_hide_brackets_in_package_options() {
    let d = clean(
        "\\documentclass[a4paper,% not ] this\n11pt]{article}\n\\usepackage[%% nor ] that\nmargin=1in]{geometry}\n\\begin{document}x\\end{document}",
    );
    assert_eq!(d.laid().pt, 11);
    assert_eq!(d.glyphs(), 1);
}

#[test]
fn a_group_may_wrap_an_environment() {
    let d = clean_body("{\\small\n\\begin{itemize}\n\\item a\n\\end{itemize}\n}\nb");
    assert_eq!(d.glyphs(), 3);
}

#[test]
fn a_group_around_blocks_restores_the_size_after_it() {
    let inside = clean_body("{\\small\n\\begin{center}\nx\n\\end{center}\n}\nb");
    let plain = clean_body("b");
    let scale = |d: &Done| d.laid().groups.iter().filter(|g| g.scale != 0).map(|g| g.scale).min().unwrap_or(0);
    assert!(scale(&inside) <= scale(&plain));
    let last = inside.all_items().iter().filter(|i| i.kind == IT_GLYPH).last().unwrap().g;
    let only = plain.all_items().iter().find(|i| i.kind == IT_GLYPH).unwrap().g;
    assert_eq!(inside.laid().groups[last as usize].scale, plain.laid().groups[only as usize].scale);
}

#[test]
fn a_plain_group_at_a_paragraph_start_stays_in_the_paragraph() {
    let d = clean_body("{\\bf Lead} text");
    assert_eq!(lines(&d).len(), 1);
}

#[test]
fn beamer_overlays_are_dropped() {
    let d = clean_body("\\only<2>{a}\\uncover<3->{b}\\alert<4>{c}\\onslide<2> d\\pause");
    assert_eq!(d.glyphs(), 4);
}

#[test]
fn frames_may_use_command_form() {
    let d = clean("\\documentclass{beamer}\\begin{document}\\frame{a}\\frame[plain]{b}\\end{document}");
    assert_eq!(d.pages, 2);
}

#[test]
fn frame_environments_take_overlays() {
    let d = clean("\\documentclass{beamer}\\begin{document}\\begin{frame}<1>[plain]{T}x\\end{frame}\\end{document}");
    assert_eq!(d.pages, 1);
    assert_eq!(d.glyphs(), 1);
}

#[test]
fn algorithms_are_numbered_floats() {
    let one = clean_body("\\begin{algorithm}\\caption{C}x\\end{algorithm}");
    assert_eq!(one.glyphs(), "Algorithm1:Cx".len());
    let two = clean_body("\\begin{algorithm}\\caption{C}\\end{algorithm}\\begin{algorithm}\\caption{C}\\end{algorithm}");
    assert_eq!(two.glyphs(), 2 * "Algorithm1:C".len());
}

#[test]
fn algorithm_blocks_indent_their_bodies() {
    let d = clean_body("\\begin{algorithmic}\\For{a}\\If{b}\\State c\\EndIf\\EndFor\\State d\\end{algorithmic}");
    let rows = lines(&d);
    assert_eq!(rows.len(), 6);
    assert!(rows[1].1 > rows[0].1);
    assert!(rows[2].1 > rows[1].1);
    assert_eq!(rows[3].1, rows[1].1);
    assert_eq!(rows[4].1, rows[0].1);
    assert_eq!(rows[5].1, rows[0].1);
}

#[test]
fn upper_case_algorithm_commands_match() {
    let d = clean_body("\\begin{algorithmic}\\FOR{a}\\STATE b\\ENDFOR\\end{algorithmic}");
    assert_eq!(lines(&d).len(), 3);
}

#[test]
fn algorithm_words_stay_on_their_own_lines() {
    let d = clean_body("\\begin{algorithmic}\\State a\\State b\\end{algorithmic}");
    assert_eq!(lines(&d).len(), 2);
}

#[test]
fn algorithm_commands_outside_algorithmic_warn() {
    assert!(warnings("\\State a") > 0);
}

#[test]
fn contents_list_numbered_headings() {
    let d = clean_body("\\tableofcontents\\section{Aa}\\subsection{Bb}\\section*{Cc}");
    let without = clean_body("\\section{Aa}\\subsection{Bb}\\section*{Cc}");
    let added = d.glyphs() - without.glyphs();
    assert_eq!(added, "Contents".len() + "1Aa1".len() + "1.1Bb1".len());
}

#[test]
fn contents_report_the_page_of_each_heading() {
    let d = clean_body("\\tableofcontents\\section{A}\\newpage\\section{B}\\newpage\\section{C}");
    let right: Vec<_> = d.items(0).iter().filter(|i| i.kind == IT_GLYPH).map(|i| i.x).collect();
    assert!(right.iter().filter(|x| **x == *right.iter().max().unwrap()).count() >= 3);
}

#[test]
fn contents_account_for_their_own_length() {
    let many = (0..80).map(|i| format!("\\section{{S{i}}}")).collect::<String>();
    let d = clean_body(&format!("\\tableofcontents\\newpage {many}"));
    assert!(d.pages >= 3);
    assert!(d.e.store.contents == d.e.store.shown);
}

#[test]
fn a_document_without_contents_lays_out_once() {
    let d = clean_body("\\section{A}");
    assert!(d.e.store.contents.is_empty());
}

#[test]
fn list_macros_may_span_macro_boundaries() {
    let s = with(
        "\\newcommand{\\Items}{\\begin{itemize}}\\newcommand{\\Done}{\\end{itemize}}\\newcommand{\\One}[1]{\\item #1}",
        "\\Items\\One{a}\\One{b}\\Done\nafter",
    );
    let d = clean(&s);
    assert_eq!(d.glyphs(), 2 + 2 + 5);
}

#[test]
fn environment_macros_work_between_paragraphs() {
    let s = with("\\newcommand{\\Start}{\\begin{center}}\\newcommand{\\Stop}{\\end{center}}", "before\n\n\\Start\nhi\n\\Stop\nafter");
    assert_eq!(clean(&s).glyphs(), 6 + 2 + 5);
}

#[test]
fn self_expanding_list_macros_stop() {
    let (message, _) = compile(&article("\\def\\a{\\item \\a}\\begin{itemize}\\a\\end{itemize}")).err().unwrap();
    assert_eq!(message, "nesting too deep");
}

#[test]
fn a_blank_list_label_hides_the_bullet() {
    let bullets = clean_body("\\begin{itemize}\\item a\\end{itemize}").glyphs();
    let bare = clean_body("\\begin{itemize}[leftmargin=1em, label={}]\\item a\\end{itemize}").glyphs();
    assert_eq!(bare + 1, bullets);
}

#[test]
fn a_label_never_leaks_past_its_item() {
    let d = clean_body("\\begin{itemize}\\item\\begin{tabular}{l}a\\end{tabular}\\end{itemize}\\section{B}");
    let items = clean_body("\\begin{tabular}{l}a\\end{tabular}\\section{B}");
    assert_eq!(d.glyphs(), items.glyphs() + 1);
}

#[test]
fn an_empty_item_does_not_hand_its_bullet_to_the_next_block() {
    let d = clean_body("\\begin{itemize}\\item\\end{itemize}\\section{B}");
    assert_eq!(d.glyphs(), "1B".len());
}

#[test]
fn starred_lists_work() {
    assert_eq!(clean_body("\\begin{enumerate*}\\item a\\end{enumerate*}").glyphs(), 3);
    assert_eq!(clean_body("\\begin{itemize*}\\item a\\end{itemize*}").glyphs(), 2);
}

#[test]
fn a_bibliography_may_omit_its_width() {
    assert!(clean_body("\\begin{thebibliography}\\end{thebibliography}").glyphs() > 0);
}

#[test]
fn pictures_we_cannot_draw_are_skipped_with_a_warning() {
    let d = good(&article("\\begin{picture}(1,1)\\put(0,0){x}\\end{picture}\n\nafter"));
    assert_eq!(d.e.warned.len(), 1);
    assert_eq!(d.glyphs(), 5);
}

#[test]
fn rules_and_char_codes_work_in_text() {
    let d = clean_body("a\\rule{1cm}{2pt}b\\char65 c\\@");
    assert_eq!(d.all_items().iter().filter(|i| i.kind == IT_RULE).count(), 1);
    assert_eq!(d.glyphs(), 4);
}

#[test]
fn a_caption_outside_a_float_only_warns() {
    let d = good(&article("\\caption{C}\n\nafter"));
    assert_eq!(d.e.warned.len(), 1);
    assert_eq!(d.glyphs(), 5);
}

#[test]
fn metadata_commands_are_quiet() {
    clean_body("\\subtitle{a}\\institute{b}\\subject{c}\\titlegraphic{d}x");
}

#[test]
fn real_mistakes_are_still_errors() {
    for s in ["\\begin{center}x\\end{quote}", "\\begin{itemize}\\item x", "\\begin{tabularx}{1cm}{l}x\\end{tabular}"] {
        assert!(compile(&article(s)).is_err(), "{s}");
    }
}

#[test]
fn references_resolve_forward() {
    let d = clean_body("see \\ref{a}\\section{S}\\label{a}");
    assert_eq!(d.glyphs(), "see1".len() + "1S".len());
}

#[test]
fn missing_references_show_a_marker() {
    let d = good(&article("\\ref{nope}"));
    assert!(d.glyphs() >= 2);
}

#[test]
fn citations_number_in_bibliography_order() {
    let s = "\\cite{b}\\begin{thebibliography}{9}\\bibitem{a}A\\bibitem{b}B\\end{thebibliography}";
    let d = clean_body(s);
    assert_eq!(d.glyphs(), "[2]".len() + "References".len() + "[1]A".len() + "[2]B".len());
}

#[test]
fn footnotes_follow_their_paragraph() {
    let d = clean_body("Hi\\footnote{There}");
    assert_eq!(d.glyphs(), "Hi1".len() + "1There".len());
}

#[test]
fn new_environments_wrap_their_body() {
    let d = clean(&with("\\newenvironment{box}{Open }{ Shut}", "\\begin{box}x\\end{box}"));
    assert_eq!(d.glyphs(), 4 + 1 + 4);
}

#[test]
fn theorems_number_themselves() {
    let d = clean(&with("\\newtheorem{thm}{Theorem}", "\\begin{thm}a\\end{thm}\\begin{thm}b\\end{thm}"));
    assert_eq!(d.glyphs(), 2 * "Theorem1.".len() + 2);
}

#[test]
fn verbatim_keeps_commands_as_text() {
    let d = clean_body("\\begin{verbatim}\n\\frac{a}\n\\end{verbatim}");
    assert_eq!(d.glyphs(), "\\frac{a}".len());
}
