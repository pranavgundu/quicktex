use super::*;

fn plot(options: &str, body: &str) -> Done {
    good(&with(
        "\\usepackage{pgfplots}",
        &format!("\\begin{{tikzpicture}}\\begin{{axis}}[{options}]{body}\\end{{axis}}\\end{{tikzpicture}}"),
    ))
}

const LINE: &str = "\\addplot coordinates {(0,0) (1,1) (2,4)};";

fn lines(d: &Done, color: u32) -> usize {
    d.all_art().iter().filter(|a| a.stroke == color).count()
}

#[test]
fn an_axis_draws_a_frame_and_tick_labels() {
    let d = plot("", LINE);
    assert!(!d.all_art().is_empty());
    assert!(d.glyphs() >= 4);
}

#[test]
fn default_plots_are_blue() {
    assert!(lines(&plot("", LINE), 0x0000ff) >= 1);
}

#[test]
fn plots_take_a_color() {
    assert!(lines(&plot("", "\\addplot[red] coordinates {(0,0) (1,1)};"), 0xff0000) >= 1);
}

#[test]
fn cycle_list_changes_the_color() {
    let d = plot("", "\\addplot coordinates {(0,0) (1,1)};\\addplot coordinates {(0,1) (1,0)};");
    assert!(lines(&d, 0x0000ff) >= 1);
    assert!(lines(&d, 0xff0000) >= 1);
}

#[test]
fn labels_print() {
    let bare = plot("", LINE).glyphs();
    assert_eq!(plot("xlabel={abc}", LINE).glyphs(), bare + 3);
    assert_eq!(plot("title={abcd}", LINE).glyphs(), bare + 4);
}

#[test]
fn y_labels_are_turned() {
    let d = plot("ylabel={abc}", LINE);
    assert_eq!(d.laid().spun.len(), 3);
    assert!(d.all_art().iter().any(|a| a.text && a.cos == 0));
}

#[test]
fn width_and_height_set_the_size() {
    let frame = |o: &str| {
        let d = plot(o, LINE);
        let points: Vec<(u8, i32, i32)> = d.all_art().iter().flat_map(|a| d.segs(a)).collect();
        let (xs, ys): (Vec<i32>, Vec<i32>) = points.iter().map(|p| (p.1, p.2)).unzip();
        (xs.iter().max().unwrap() - xs.iter().min().unwrap(), ys.iter().max().unwrap() - ys.iter().min().unwrap())
    };
    let small = frame("width=6cm,height=4cm");
    let large = frame("width=12cm,height=8cm");
    assert!(large.0 > small.0 && large.1 > small.1);
}

#[test]
fn linewidth_sizes_relative_to_the_column() {
    let a = plot("width=\\linewidth", LINE);
    let b = plot("width=0.5\\linewidth", LINE);
    let w = |d: &Done| {
        d.all_art().iter().flat_map(|a| d.segs(a)).map(|p| p.1).max().unwrap()
            - d.all_art().iter().flat_map(|a| d.segs(a)).map(|p| p.1).min().unwrap()
    };
    assert!(w(&a) > w(&b));
}

#[test]
fn explicit_limits_clip_the_data() {
    let d = plot("xmin=0,xmax=1,ymin=0,ymax=1", "\\addplot coordinates {(0,0) (5,5)};");
    let curve = d.all_art().iter().find(|a| a.stroke == 0x0000ff).unwrap();
    let frame: Vec<(u8, i32, i32)> = d.all_art().iter().filter(|a| a.stroke == 0).flat_map(|a| d.segs(a)).collect();
    let right = frame.iter().map(|p| p.1).max().unwrap();
    assert!(d.segs(curve).iter().all(|p| p.1 <= right + 20));
}

#[test]
fn functions_are_sampled() {
    let d = plot("", "\\addplot[blue,domain=0:1,samples=20] {x^2};");
    let curve = d.all_art().iter().find(|a| a.stroke == 0x0000ff).unwrap();
    assert_eq!(d.segs(curve).len(), 20);
}

#[test]
fn default_samples_are_twenty_five() {
    let d = plot("", "\\addplot[blue,domain=0:1] {x};");
    let curve = d.all_art().iter().find(|a| a.stroke == 0x0000ff).unwrap();
    assert_eq!(d.segs(curve).len(), 25);
}

#[test]
fn functions_can_use_every_operator() {
    for f in [
        "x",
        "x+1",
        "2*x",
        "x/2",
        "x^2",
        "sin(x)",
        "cos(x)",
        "tan(x/2)",
        "exp(x)",
        "ln(x+2)",
        "sqrt(x+2)",
        "abs(x)",
        "0.4896*x + 0.0290",
        "sin(deg(x))",
        "-x",
        "floor(x)",
        "ceil(x)",
        "max(x,1)",
        "min(x,1)",
        "pow(x,2)",
        "pi*x",
    ] {
        plot("", &format!("\\addplot[domain=0:1] {{{f}}};"));
    }
}

#[test]
fn bad_functions_warn() {
    for f in ["x+", "nope(x)", "(x", "y"] {
        assert!(
            !good(&with(
                "\\usepackage{pgfplots}",
                &format!("\\begin{{tikzpicture}}\\begin{{axis}}\\addplot {{{f}}};\\end{{axis}}\\end{{tikzpicture}}")
            ))
            .e
            .warned
            .is_empty(),
            "{f}"
        );
    }
}

#[test]
fn undefined_samples_are_skipped() {
    plot("", "\\addplot[domain=-1:1] {ln(x)};");
    plot("", "\\addplot[domain=-1:1] {1/x};");
    plot("", "\\addplot[domain=-1:1] {sqrt(x)};");
}

#[test]
fn only_marks_has_no_line() {
    let d = plot("", "\\addplot[only marks,mark=*] coordinates {(0,0) (1,1)};");
    let joined = plot("", "\\addplot[mark=*] coordinates {(0,0) (1,1)};");
    assert!(d.all_art().len() < joined.all_art().len());
}

#[test]
fn every_mark_draws() {
    for m in ["*", "o", "square*", "square", "triangle*", "triangle", "x", "+"] {
        let d = plot("", &format!("\\addplot[only marks,mark={m}] coordinates {{(0,0) (1,1)}};"));
        let bare = plot("", "\\addplot[no marks] coordinates {(0,0) (1,1)};");
        assert!(d.all_art().len() >= bare.all_art().len(), "{m}");
    }
}

#[test]
fn grid_adds_lines() {
    let plain = plot("", LINE).all_art().len();
    let major = plot("grid=major", LINE).all_art().len();
    let both = plot("grid=both", LINE).all_art().len();
    assert!(major > plain);
    assert!(both >= major);
}

#[test]
fn legends_list_every_entry() {
    let bare = plot("", LINE).glyphs();
    assert_eq!(plot("", &format!("{LINE}\\addlegendentry{{abc}}")).glyphs(), bare + 3);
}

#[test]
fn legend_samples_match_the_plot() {
    let d = plot("", "\\addplot[red] coordinates {(0,0) (1,1)};\\addlegendentry{a}");
    assert!(lines(&d, 0xff0000) >= 2);
}

#[test]
fn legend_position_moves_the_box() {
    let q = body("Q").all_items()[0].a;
    let spot = |p: &str| {
        let d = plot(&format!("legend pos={p}"), &format!("{LINE}\\addlegendentry{{Q}}"));
        let g = *d.all_items().iter().find(|i| i.kind == IT_GLYPH && i.a == q).unwrap();
        (g.x, g.y)
    };
    let (ne, nw, se) = (spot("north east"), spot("north west"), spot("south east"));
    assert!(nw.0 < ne.0, "{nw:?} {ne:?}");
    assert!(se.1 > ne.1, "{se:?} {ne:?}");
}

#[test]
fn axes_hold_nodes() {
    let bare = plot("", LINE).glyphs();
    let d = plot("", &format!("{LINE}\\node at (rel axis cs:0.5,0.5) {{abc}};"));
    assert_eq!(d.glyphs(), bare + 3);
}

#[test]
fn nodes_can_use_data_coordinates() {
    let bare = plot("", LINE).glyphs();
    assert_eq!(plot("", &format!("{LINE}\\node at (axis cs:1,1) {{a}};")).glyphs(), bare + 1);
}

#[test]
fn tick_label_font_applies() {
    let d = plot("tick label style={font=\\footnotesize}", LINE);
    assert!(d.laid().groups.iter().any(|g| g.scale < 1000));
}

#[test]
fn single_points_get_a_usable_range() {
    plot("", "\\addplot coordinates {(1,1)};");
}

#[test]
fn constant_data_gets_a_usable_range() {
    plot("", "\\addplot coordinates {(0,2) (1,2)};");
}

#[test]
fn empty_axes_render() {
    plot("", "");
}

#[test]
fn unsupported_axis_options_warn() {
    assert!(
        !good(&with(
            "\\usepackage{pgfplots}",
            "\\begin{tikzpicture}\\begin{axis}[wobble=3]\\addplot coordinates {(0,0)};\\end{axis}\\end{tikzpicture}"
        ))
        .e
        .warned
        .is_empty()
    );
}

#[test]
fn unclosed_axes_are_errors() {
    assert!(
        compile(&with("\\usepackage{pgfplots}", "\\begin{tikzpicture}\\begin{axis}\\addplot coordinates {(0,0)};\\end{tikzpicture}"))
            .is_err()
    );
}

#[test]
fn malformed_coordinates_warn() {
    assert!(
        !good(&with(
            "\\usepackage{pgfplots}",
            "\\begin{tikzpicture}\\begin{axis}\\addplot coordinates {(0,};\\end{axis}\\end{tikzpicture}"
        ))
        .e
        .warned
        .is_empty()
    );
}
