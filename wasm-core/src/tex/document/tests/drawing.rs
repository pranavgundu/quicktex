use super::*;

const CM_UNITS: i32 = 2845;

fn pic(body: &str) -> Done {
    good(&with("\\usepackage{tikz}", &format!("\\begin{{tikzpicture}}{body}\\end{{tikzpicture}}")))
}

fn pic_with(options: &str, body: &str) -> Done {
    good(&with("\\usepackage{tikz}", &format!("\\begin{{tikzpicture}}[{options}]{body}\\end{{tikzpicture}}")))
}

fn fails(body: &str) -> String {
    compile(&with("\\usepackage{tikz}", &format!("\\begin{{tikzpicture}}{body}\\end{{tikzpicture}}"))).err().map(|e| e.0).unwrap_or_else(|| "rendered".into())
}

fn outline(d: &Done, a: &Art) -> Vec<(i32, i32)> {
    let segs = d.segs(a);
    let mut points = Vec::new();
    let mut k = 0;
    while k < segs.len() {
        let g = segs[k];
        if g.0 == SEG_CURVE && k + 2 < segs.len() {
            let (from, p, q, to) = (*points.last().unwrap(), (g.1, g.2), (segs[k + 1].1, segs[k + 1].2), (segs[k + 2].1, segs[k + 2].2));
            let (from, p, q, to): ((i32, i32), (i32, i32), (i32, i32), (i32, i32)) = (from, p, q, to);
            for step in 1..=32 {
                let t = step as f64 / 32.0;
                let u = 1.0 - t;
                let at = |a: i32, b: i32, c: i32, e: i32| (u * u * u * a as f64 + 3.0 * u * u * t * b as f64 + 3.0 * u * t * t * c as f64 + t * t * t * e as f64) as i32;
                points.push((at(from.0, p.0, q.0, to.0), at(from.1, p.1, q.1, to.1)));
            }
            k += 3;
            continue;
        }
        if g.0 != SEG_CLOSE {
            points.push((g.1, g.2));
        }
        k += 1;
    }
    points
}

fn span(d: &Done) -> (i32, i32) {
    let points: Vec<(i32, i32)> = d.all_art().iter().flat_map(|a| outline(d, a)).collect();
    let (xs, ys): (Vec<i32>, Vec<i32>) = points.into_iter().unzip();
    (xs.iter().max().unwrap() - xs.iter().min().unwrap(), ys.iter().max().unwrap() - ys.iter().min().unwrap())
}

const FRAME: &str = "\\draw (-3,-3) rectangle (3,3);";

fn spot(option: &str) -> (i32, i32) {
    let d = pic(&format!("{FRAME}\\node[{option}] at (0,0) {{x}};"));
    let i = *d.all_items().iter().find(|i| i.kind == IT_GLYPH).unwrap();
    (i.x, i.y)
}

fn close(a: i32, b: i32) -> bool {
    (a - b).abs() <= 6
}

fn ends(d: &Done) -> ((i32, i32), (i32, i32)) {
    let s = d.segs(&d.all_art()[0]);
    ((s[0].1, s[0].2), (s[s.len() - 1].1, s[s.len() - 1].2))
}

#[test]
fn a_line_is_a_move_and_a_line() {
    let d = pic("\\draw (0,0) -- (1,0);");
    assert_eq!(d.all_art().len(), 1);
    let s = d.segs(&d.all_art()[0]);
    assert_eq!(s.iter().map(|g| g.0).collect::<Vec<_>>(), [SEG_MOVE, SEG_LINE]);
}

#[test]
fn one_centimetre_is_the_right_length() {
    assert!(close(span(&pic("\\draw (0,0) -- (1,0);")).0, CM_UNITS));
    assert!(close(span(&pic("\\draw (0,0) -- (0,1);")).1, CM_UNITS));
    assert!(close(span(&pic("\\draw (0,0) -- (2.5,0);")).0, CM_UNITS * 5 / 2));
}

#[test]
fn coordinates_accept_units() {
    assert!(close(span(&pic("\\draw (0,0) -- (1cm,0);")).0, CM_UNITS));
    assert!(close(span(&pic("\\draw (0,0) -- (10mm,0);")).0, CM_UNITS));
    assert!(close(span(&pic("\\draw (0,0) -- (1in,0);")).0, 7227));
    assert!(close(span(&pic("\\draw (0,0) -- (10pt,0);")).0, 1000));
}

#[test]
fn y_points_up() {
    let (a, b) = ends(&pic("\\draw (0,0) -- (0,1);"));
    assert!(b.1 < a.1);
}

#[test]
fn x_points_right() {
    let (a, b) = ends(&pic("\\draw (0,0) -- (1,0);"));
    assert!(b.0 > a.0);
}

#[test]
fn polar_coordinates_work() {
    let (x, y) = span(&pic("\\draw (0,0) -- (0:1);"));
    assert!(close(x, CM_UNITS) && close(y, 0));
    let (x, y) = span(&pic("\\draw (0,0) -- (90:1);"));
    assert!(close(x, 0) && close(y, CM_UNITS));
}

#[test]
fn relative_points_move_on() {
    assert!(close(span(&pic("\\draw (0,0) -- ++(1,0) -- ++(1,0);")).0, 2 * CM_UNITS));
}

#[test]
fn plus_points_do_not_move_on() {
    assert!(close(span(&pic("\\draw (0,0) -- +(1,0) -- +(0,1);")).0, CM_UNITS));
}

#[test]
fn corners_turn_correctly() {
    let d = pic("\\draw (0,0) |- (1,1);");
    let s = d.segs(&d.all_art()[0]);
    assert_eq!(s.len(), 3);
    assert_eq!(s[1].1, s[0].1);
    let d = pic("\\draw (0,0) -| (1,1);");
    let s = d.segs(&d.all_art()[0]);
    assert_eq!(s[1].2, s[0].2);
}

#[test]
fn cycle_closes_the_path() {
    let d = pic("\\draw (0,0) -- (1,0) -- (1,1) -- cycle;");
    assert_eq!(d.segs(&d.all_art()[0]).last().unwrap().0, SEG_CLOSE);
}

#[test]
fn rectangles_have_the_right_size() {
    let d = pic("\\draw (0,0) rectangle (2,1);");
    let (x, y) = span(&d);
    assert!(close(x, 2 * CM_UNITS) && close(y, CM_UNITS));
    assert_eq!(d.segs(&d.all_art()[0]).last().unwrap().0, SEG_CLOSE);
}

#[test]
fn circles_are_round() {
    let (x, y) = span(&pic("\\draw (0,0) circle (1);"));
    assert!(close(x, 2 * CM_UNITS) && close(y, 2 * CM_UNITS), "{x} {y}");
}

#[test]
fn circles_use_curves() {
    let d = pic("\\draw (0,0) circle (1);");
    assert!(d.segs(&d.all_art()[0]).iter().any(|g| g.0 == SEG_CURVE));
}

#[test]
fn ellipses_have_two_radii() {
    let (x, y) = span(&pic("\\draw (0,0) ellipse (2 and 1);"));
    assert!(close(x, 4 * CM_UNITS) && close(y, 2 * CM_UNITS), "{x} {y}");
}

#[test]
fn arcs_end_in_the_right_place() {
    let d = pic("\\draw (1,0) arc (0:90:1);");
    let (a, b) = ends(&d);
    assert!(close(a.0 - b.0, CM_UNITS) && close(a.1 - b.1, CM_UNITS), "{a:?} {b:?}");
}

#[test]
fn curves_keep_their_controls() {
    let d = pic("\\draw (0,0) .. controls (1,1) and (2,1) .. (3,0);");
    assert!(d.segs(&d.all_art()[0]).iter().any(|g| g.0 == SEG_CURVE));
    assert!(close(span(&d).0, 3 * CM_UNITS));
}

#[test]
fn to_draws_a_line() {
    assert!(close(span(&pic("\\draw (0,0) to (1,0);")).0, CM_UNITS));
}

#[test]
fn several_paths_make_several_shapes() {
    assert_eq!(pic("\\draw (0,0) -- (1,0);\\draw (0,1) -- (1,1);\\draw (0,2) -- (1,2);").all_art().len(), 3);
}

#[test]
fn draw_strokes_and_does_not_fill() {
    let d = pic("\\draw (0,0) rectangle (1,1);");
    let a = d.all_art()[0];
    assert_eq!(a.stroke, 0);
    assert_eq!(a.fill, NO_COLOR);
}

#[test]
fn fill_fills_and_does_not_stroke() {
    let d = pic("\\fill[red] (0,0) rectangle (1,1);");
    let a = d.all_art()[0];
    assert_eq!(a.fill, 0xff0000);
    assert_eq!(a.stroke, NO_COLOR);
}

#[test]
fn filldraw_does_both() {
    let d = pic("\\filldraw[fill=blue,draw=red] (0,0) rectangle (1,1);");
    let a = d.all_art()[0];
    assert_eq!(a.fill, 0x0000ff);
    assert_eq!(a.stroke, 0xff0000);
}

#[test]
fn draw_can_take_a_fill() {
    let d = pic("\\draw[fill=gray!20] (0,0) rectangle (1,1);");
    assert_eq!(d.all_art()[0].fill, 0xe6e6e6);
    assert_eq!(d.all_art()[0].stroke, 0);
}

#[test]
fn path_alone_draws_nothing() {
    assert_eq!(pic("\\path (0,0) -- (1,0);").all_art().len(), 0);
}

#[test]
fn colors_by_name() {
    for (name, want) in [("red", 0xff0000), ("green", 0x00ff00), ("blue", 0x0000ff), ("gray", 0x808080), ("black", 0), ("white", 0xffffff), ("orange", 0xff8000)] {
        assert_eq!(pic(&format!("\\draw[{name}] (0,0) -- (1,0);")).all_art()[0].stroke, want, "{name}");
    }
}

#[test]
fn colors_can_be_mixed() {
    assert_eq!(pic("\\draw[red!50] (0,0) -- (1,0);").all_art()[0].stroke, 0xff8080);
    assert_eq!(pic("\\draw[red!50!blue] (0,0) -- (1,0);").all_art()[0].stroke, 0x800080);
    assert_eq!(pic("\\draw[color=green!50!black] (0,0) -- (1,0);").all_art()[0].stroke, 0x008000);
}

#[test]
fn line_widths_are_in_order() {
    let w = |o: &str| pic(&format!("\\draw[{o}] (0,0) -- (1,0);")).all_art()[0].width;
    let order = ["ultra thin", "very thin", "thin", "semithick", "thick", "very thick", "ultra thick"];
    let widths: Vec<i32> = order.iter().map(|o| w(o)).collect();
    assert!(widths.windows(2).all(|p| p[0] < p[1]), "{widths:?}");
    assert_eq!(w("thin"), pic("\\draw (0,0) -- (1,0);").all_art()[0].width);
}

#[test]
fn explicit_line_width_is_used() {
    let w = pic("\\draw[line width=2pt] (0,0) -- (1,0);").all_art()[0].width;
    assert!((w - 200).abs() <= 2, "{w}");
}

#[test]
fn dashes_are_set() {
    assert_eq!(pic("\\draw (0,0) -- (1,0);").all_art()[0].dash.0, 0);
    assert!(pic("\\draw[dashed] (0,0) -- (1,0);").all_art()[0].dash.0 > 0);
    let dot = pic("\\draw[dotted] (0,0) -- (1,0);").all_art()[0].dash;
    let dash = pic("\\draw[dashed] (0,0) -- (1,0);").all_art()[0].dash;
    assert!(dot.0 < dash.0);
}

#[test]
fn line_cap_and_join_are_kept() {
    assert!(pic("\\draw[line cap=round] (0,0) -- (1,0);").all_art()[0].round);
    assert!(!pic("\\draw (0,0) -- (1,0);").all_art()[0].round);
}

#[test]
fn arrows_add_tips() {
    let plain = pic("\\draw (0,0) -- (1,0);").all_art().len();
    let one = pic("\\draw[->] (0,0) -- (1,0);").all_art().len();
    let both = pic("\\draw[<->] (0,0) -- (1,0);").all_art().len();
    assert!(one > plain && both > one);
}

#[test]
fn every_tip_style_works() {
    for t in ["->", "<-", "<->", "-to", "-stealth", "-latex", "-Stealth", "-Latex", "-{Stealth[length=5pt]}", "stealth-stealth"] {
        pic(&format!("\\draw[{t}] (0,0) -- (1,0);"));
    }
}

#[test]
fn default_arrow_style_applies() {
    let a = pic("\\draw[->] (0,0) -- (1,0);");
    let b = pic_with(">=Stealth", "\\draw[->] (0,0) -- (1,0);");
    assert_ne!(a.segs(&a.all_art()[1]).len() + a.all_art().len(), 0);
    assert_ne!(a.segs(&a.all_art()[1]), b.segs(&b.all_art()[1]));
}

#[test]
fn picture_scale_multiplies() {
    assert!(close(span(&pic_with("scale=2", "\\draw (0,0) -- (1,0);")).0, 2 * CM_UNITS));
    assert!(close(span(&pic_with("scale=0.5", "\\draw (0,0) -- (1,0);")).0, CM_UNITS / 2));
}

#[test]
fn axis_scales_are_separate() {
    let (x, y) = span(&pic_with("xscale=2,yscale=3", "\\draw (0,0) rectangle (1,1);"));
    assert!(close(x, 2 * CM_UNITS) && close(y, 3 * CM_UNITS));
}

#[test]
fn negative_scale_mirrors() {
    let (a, b) = ends(&pic_with("xscale=-1", "\\draw (0,0) -- (1,0);"));
    assert!(b.0 < a.0);
}

#[test]
fn rotate_turns_the_path() {
    let (x, y) = span(&pic_with("rotate=90", "\\draw (0,0) -- (1,0);"));
    assert!(close(x, 0) && close(y, CM_UNITS), "{x} {y}");
}

#[test]
fn scopes_nest_and_end() {
    let d = pic("\\begin{scope}[scale=2]\\begin{scope}[scale=2]\\draw (0,0) -- (1,0);\\end{scope}\\draw (0,1) -- (1,1);\\end{scope}\\draw (0,2) -- (1,2);");
    let len = |i: usize| {
        let s = d.segs(&d.all_art()[i]);
        s[1].1 - s[0].1
    };
    assert!(close(len(0), 4 * CM_UNITS) && close(len(1), 2 * CM_UNITS) && close(len(2), CM_UNITS));
}

#[test]
fn shift_moves_a_scope() {
    let d = pic("\\draw (0,0) -- (1,0);\\begin{scope}[shift={(3,0)}]\\draw (0,0) -- (1,0);\\end{scope}");
    let x = |i: usize| d.segs(&d.all_art()[i])[0].1;
    assert!(close(x(1) - x(0), 3 * CM_UNITS));
}

#[test]
fn xshift_and_yshift_move() {
    let d = pic("\\draw (0,0) -- (1,0);\\begin{scope}[xshift=1cm,yshift=1cm]\\draw (0,0) -- (1,0);\\end{scope}");
    let p = |i: usize| d.segs(&d.all_art()[i])[0];
    assert!(close(p(1).1 - p(0).1, CM_UNITS));
    assert!(close(p(0).2 - p(1).2, CM_UNITS));
}

#[test]
fn named_coordinates_resolve() {
    assert!(close(span(&pic("\\coordinate (a) at (2,0);\\draw (0,0) -- (a);")).0, 2 * CM_UNITS));
}

#[test]
fn inline_coordinates_resolve() {
    assert!(close(span(&pic("\\draw (0,0) -- ++(1,0) coordinate (a);\\draw (a) -- ++(1,0);")).0, 2 * CM_UNITS));
}

#[test]
fn coordinates_survive_a_scope() {
    assert!(close(span(&pic("\\begin{scope}[shift={(1,0)}]\\coordinate (a) at (1,0);\\end{scope}\\draw (0,0) -- (a);")).0, 2 * CM_UNITS));
}

#[test]
fn unknown_coordinates_are_errors() {
    assert_ne!(fails("\\draw (0,0) -- (nope);"), "rendered");
}

#[test]
fn calc_adds_points() {
    assert!(close(span(&pic("\\coordinate (a) at (1,0);\\draw (0,0) -- ($(a)+(1,0)$);")).0, 2 * CM_UNITS));
}

#[test]
fn calc_subtracts_points() {
    assert!(close(span(&pic("\\coordinate (a) at (3,0);\\draw (0,0) -- ($(a)-(1,0)$);")).0, 2 * CM_UNITS));
}

#[test]
fn calc_finds_the_midpoint() {
    assert!(close(span(&pic("\\coordinate (a) at (2,0);\\draw (0,0) -- ($(0,0)!0.5!(a)$);")).0, CM_UNITS));
}

#[test]
fn coordinates_can_be_expressions() {
    assert!(close(span(&pic("\\draw (0,0) -- ({1+1},0);")).0, 2 * CM_UNITS));
    assert!(close(span(&pic("\\draw (0,0) -- ({sqrt(4)},0);")).0, 2 * CM_UNITS));
}

#[test]
fn nodes_print_their_text() {
    assert_eq!(pic("\\node at (0,0) {abc};").glyphs(), 3);
}

#[test]
fn nodes_hold_math() {
    assert_eq!(pic("\\node at (0,0) {$x^2$};").glyphs(), 2);
}

#[test]
fn nodes_center_on_the_point() {
    let d = pic("\\draw (0,0) -- (1,0);\\node at (0,0) {x};");
    let i = d.all_items().iter().find(|i| i.kind == IT_GLYPH).unwrap();
    let x0 = d.segs(&d.all_art()[0])[0].1;
    assert!((i.x + 286 - x0).abs() < 60, "{} {}", i.x, x0);
}

#[test]
fn placement_moves_the_text() {
    let (c, a, b, l, r) = (spot("anchor=center"), spot("above"), spot("below"), spot("left"), spot("right"));
    assert!(a.1 < c.1 && b.1 > c.1, "{a:?} {b:?} {c:?}");
    assert!(l.0 < c.0 && r.0 > c.0, "{l:?} {r:?} {c:?}");
}

#[test]
fn placement_distance_is_used() {
    assert!(spot("above=10pt").1 < spot("above=1pt").1);
}

#[test]
fn diagonal_placement_moves_both_ways() {
    let c = spot("anchor=center");
    let ar = spot("above right");
    assert!(ar.0 > c.0 && ar.1 < c.1);
    let bl = spot("below left");
    assert!(bl.0 < c.0 && bl.1 > c.1);
}

#[test]
fn every_anchor_works() {
    for a in ["north", "south", "east", "west", "center", "north east", "north west", "south east", "south west"] {
        pic(&format!("\\node[anchor={a}] at (0,0) {{x}};"));
    }
}

#[test]
fn drawn_nodes_get_a_border() {
    assert_eq!(pic("\\node at (0,0) {x};").all_art().len(), 0);
    assert_eq!(pic("\\node[draw] at (0,0) {x};").all_art().len(), 1);
}

#[test]
fn filled_nodes_get_a_background() {
    assert_eq!(pic("\\node[fill=yellow] at (0,0) {x};").all_art()[0].fill, 0xffff00);
}

#[test]
fn circle_nodes_cover_their_corners() {
    let d = pic("\\node[draw,circle] at (0,0) {Abc};");
    let s = span(&d);
    assert!((s.0 - s.1).abs() <= 6, "{s:?}");
    let glyph_span = d.all_items().iter().map(|i| i.x).max().unwrap() - d.all_items().iter().map(|i| i.x).min().unwrap();
    assert!(s.0 > glyph_span + 700);
}

#[test]
fn inner_sep_grows_the_border() {
    let w = |o: &str| span(&pic(&format!("\\node[draw,{o}] at (0,0) {{x}};"))).0;
    assert!(w("inner sep=10pt") > w("inner sep=1pt"));
}

#[test]
fn rotated_nodes_become_turned_text() {
    let d = pic("\\node[rotate=30] at (0,0) {x};");
    assert_eq!(d.laid().spun.len(), 1);
    assert_eq!(d.all_art().iter().filter(|a| a.text).count(), 1);
    assert_eq!(d.glyphs(), 0);
}

#[test]
fn turned_text_keeps_a_unit_rotation() {
    let d = pic("\\node[rotate=30] at (0,0) {x};");
    let a = d.all_art().iter().find(|a| a.text).unwrap();
    let length = ((a.cos as f64).powi(2) + (a.sin as f64).powi(2)).sqrt();
    assert!((length - 1000.0).abs() < 3.0, "{length}");
    assert!((a.cos - 866).abs() <= 1, "{}", a.cos);
}

#[test]
fn path_nodes_land_at_the_end() {
    let d = pic("\\draw (0,0) -- (3,0) node {x};");
    let i = d.all_items().iter().find(|i| i.kind == IT_GLYPH).unwrap();
    let (a, b) = ends(&d);
    assert!((i.x - b.0).abs() < 500 && i.x > a.0);
}

#[test]
fn midway_nodes_land_in_the_middle() {
    let d = pic("\\draw (0,0) -- (3,0) node[midway] {x};");
    let i = d.all_items().iter().find(|i| i.kind == IT_GLYPH).unwrap();
    let (a, b) = ends(&d);
    assert!((i.x - (a.0 + b.0) / 2).abs() < 500);
}

#[test]
fn lines_stop_at_box_borders() {
    let d = pic("\\node[draw] (a) at (0,0) {x};\\draw (a) -- (3,0);");
    let border = d.segs(&d.all_art()[0]);
    let right = border.iter().map(|g| g.1).max().unwrap();
    let line = d.all_art().iter().rfind(|a| !a.text).unwrap();
    let s = d.segs(line);
    assert!((s[0].1 - right).abs() < 40, "{} {right}", s[0].1);
}

#[test]
fn lines_stop_at_circle_borders() {
    let d = pic("\\node[draw,circle] (a) at (0,0) {x};\\draw (a) -- (3,0);");
    let right = d.segs(&d.all_art()[0]).iter().map(|g| g.1).max().unwrap();
    let line = d.all_art().iter().rfind(|a| !a.text).unwrap();
    assert!((d.segs(line)[0].1 - right).abs() < 40);
}

#[test]
fn lines_reach_named_coordinates_exactly() {
    let d = pic("\\coordinate (a) at (0,0);\\draw (a) -- (1,0);");
    assert!(close(span(&d).0, CM_UNITS));
}

#[test]
fn lines_between_two_nodes_touch_both() {
    let d = pic("\\node[draw] (a) at (0,0) {x};\\node[draw] (b) at (3,0) {x};\\draw (a) -- (b);");
    let line = d.all_art().iter().rfind(|a| !a.text).unwrap();
    let s = d.segs(line);
    let center = |i: usize| {
        let b = d.segs(&d.all_art()[i]);
        (b.iter().map(|g| g.1).max().unwrap() + b.iter().map(|g| g.1).min().unwrap()) / 2
    };
    assert!(s[0].1 > center(0) && s[1].1 < center(1));
}

#[test]
fn picture_options_style_every_node() {
    let d = pic_with("every node/.style={font=\\small}", "\\node at (0,0) {x};");
    assert!(d.laid().groups.iter().any(|g| g.scale < 1000));
}

#[test]
fn named_styles_apply() {
    let d = pic_with("mine/.style={red,thick}", "\\draw[mine] (0,0) -- (1,0);");
    assert_eq!(d.all_art()[0].stroke, 0xff0000);
    assert!(d.all_art()[0].width > pic("\\draw (0,0) -- (1,0);").all_art()[0].width);
}

#[test]
fn styles_can_be_overridden_at_use() {
    let d = pic_with("mine/.style={red}", "\\draw[mine,blue] (0,0) -- (1,0);");
    assert_eq!(d.all_art()[0].stroke, 0x0000ff);
}

#[test]
fn styles_can_use_styles() {
    let d = pic_with("a/.style={red},b/.style={a,thick}", "\\draw[b] (0,0) -- (1,0);");
    assert_eq!(d.all_art()[0].stroke, 0xff0000);
}

#[test]
fn self_referencing_styles_stop() {
    assert_ne!(compile(&with("\\usepackage{tikz}", "\\begin{tikzpicture}[a/.style={a}]\\draw[a] (0,0) -- (1,0);\\end{tikzpicture}")).err().map(|e| e.0).unwrap_or_default(), "panic");
}

#[test]
fn unsupported_options_are_named_errors() {
    for o in ["wobble", "decorate", "snake it", "pattern=dots", "shading=axis"] {
        assert_eq!(fails(&format!("\\draw[{o}] (0,0) -- (1,0);")), "unsupported drawing option", "{o}");
    }
}

#[test]
fn unsupported_option_points_at_the_line() {
    let s = with("\\usepackage{tikz}", "\\begin{tikzpicture}\n\\draw (0,0) -- (1,0);\n\\draw[wobble] (0,0) -- (1,0);\n\\end{tikzpicture}");
    let (_, at) = compile(&s).err().unwrap();
    assert_eq!(s[..at].matches('\n').count(), 5);
}

#[test]
fn syntax_errors_are_errors() {
    for s in ["\\draw (0,0) --", "\\draw (0,0", "\\draw[red (0,0);", "\\node at (0,0) {x", "\\draw (a,b,c) -- (1,0);"] {
        assert_ne!(fails(s), "rendered", "{s}");
    }
}

#[test]
fn unclosed_pictures_are_errors() {
    assert!(compile(&with("\\usepackage{tikz}", "\\begin{tikzpicture}\\draw (0,0) -- (1,0);")).is_err());
}

#[test]
fn unclosed_scopes_are_errors() {
    assert_ne!(fails("\\begin{scope}\\draw (0,0) -- (1,0);"), "rendered");
}

#[test]
fn pictures_push_the_following_text_down() {
    let short = pic("\\draw (0,0) -- (0,1);");
    let tall = pic("\\draw (0,0) -- (0,5);");
    let _ = (short, tall);
    let y = |h: &str| {
        let d = good(&with("\\usepackage{tikz}", &format!("\\begin{{tikzpicture}}\\draw (0,0) -- (0,{h});\\end{{tikzpicture}}\n\nx")));
        d.all_items().iter().find(|i| i.kind == IT_GLYPH).unwrap().y
    };
    assert!(y("5") > y("1") + 3 * CM_UNITS);
}

#[test]
fn pictures_are_centered_when_asked() {
    let d = good(&with("\\usepackage{tikz}", "\\begin{center}\\begin{tikzpicture}\\draw (0,0) -- (2,0);\\end{tikzpicture}\\end{center}"));
    let (a, b) = ends(&d);
    assert!(((a.0 + b.0) / 2 - d.laid().w / 2).abs() < 300);
}

#[test]
fn art_goes_on_the_right_page() {
    let d = good(&with("\\usepackage{tikz}", "a\\newpage\\begin{tikzpicture}\\draw (0,0) -- (1,0);\\end{tikzpicture}"));
    assert_eq!(d.art(0).len(), 0);
    assert_eq!(d.art(1).len(), 1);
}

#[test]
fn tikz_needs_the_package_in_math_only() {
    assert!(crate::tex::tests::inline("\\usepackage{tikz}x").is_err());
}
