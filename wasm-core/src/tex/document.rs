use super::*;

mod commands;
pub(super) use commands::{accent_glyph, composed};
mod control;
mod figure;
#[cfg(test)]
mod tests;

mod float;
mod header;
mod hyphen;
mod legacy;
mod plot;

const CENTER_GAP: i32 = 800;
const PARTOP_GAP: i32 = 200;
const CENTER_GIVE: i32 = 400;
const PARTOP_GIVE: i32 = 100;
const HEADING_GIVE: i32 = 86;
const DISPLAY_GIVE: i32 = 500;
const SHORT_DISPLAY_GIVE: i32 = 300;
const CENTER_STRETCH: i32 = 200;
const PARTOP_STRETCH: i32 = 100;
const HEADING_STRETCH: i32 = 430;
const HEADING_AFTER_STRETCH: i32 = 86;
const DISPLAY_STRETCH: i32 = 200;
const SHORT_DISPLAY_STRETCH: i32 = 300;
const PAR_STRETCH: i32 = 100;
const XSPACE: i32 = 500;
pub(super) const RAGGED_FILL: i32 = 2000;
pub(super) const CENTERED_FILL: i32 = 4000;
const INCH: i32 = 72270;
const CLUB_PENALTY: i64 = 150;
const WIDOW_PENALTY: i64 = 150;
const BROKEN_PENALTY: i64 = 100;
const NO_BREAK: i64 = 10000;
const DEPLORABLE: i64 = 100000;
const TITLE_GAP: i32 = 2800;
const SPACE: i32 = 333;
const LINESKIP: i32 = 100;
const TOPSKIP: i32 = 1000;
const PARINDENT: i32 = 1500;
const LIST_INDENT: i32 = 2500;
const LABEL_GAP: i32 = 500;
const QUOTE_INDENT: i32 = 2000;
const EMERGENCY_STRETCH: i32 = 3000;
const INF_BAD: i64 = 10_000;
const PRETOLERANCE: i64 = 100;
const TOLERANCE: i64 = 200;
const SLOPPY_TOLERANCE: i64 = 9999;
const LINE_PENALTY: i64 = 10;
const HYPHEN_PENALTY: i64 = 50;
const ADJ_DEMERITS: i64 = 10_000;
const DOUBLE_HYPHEN_DEMERITS: i64 = 10_000;
const FIT_VERY_LOOSE: usize = 0;
const FIT_LOOSE: usize = 1;
const FIT_DECENT: usize = 2;
const FIT_TIGHT: usize = 3;
const NO_PATH: i64 = i64::MAX / 4;
const OVERFULL: i64 = 1 << 40;

struct Fit {
    width: i32,
    indent: i32,
    ragged: bool,
    space: Space,
    emergency: i32,
    fill: i32,
    fixed: bool,
    closed: bool,
    plain_space: i32,
    wide_space: i32,
}

/// A font's interword space: its width, stretch, shrink and extra space.
type Space = (i32, i32, i32, i32);


impl Fit {
    /// The interword glue TeX inserts after a word with this space factor.
    #[inline(always)]
    fn glue(&self, word: &Word) -> (i32, i32, i32) {
        if self.fixed {
            return (if word.after >= 2000 { self.wide_space } else { self.plain_space }, 0, 0);
        }
        let (space, stretch, shrink, extra) = if word.space.0 == 0 { self.space } else { word.space };
        let factor = word.after as i32;
        let width = space + if factor >= 2000 { extra } else { 0 };
        (width, stretch * factor / 1000, shrink * 1000 / factor)
    }
}

/// TeX's badness: about 100 times the cube of the glue set ratio.
#[inline(always)]
fn badness(excess: i64, room: i64) -> i64 {
    if excess == 0 {
        return 0;
    }
    if room <= 0 {
        return INF_BAD;
    }
    let ratio = excess * 297 / room;
    if ratio > 1290 { INF_BAD } else { (ratio * ratio * ratio + 0o400000) / 0o1000000 }
}

/// What ending a page here costs: a break TeX forbids never wins, and a page
/// too bad to measure loses to any it can.
fn page_cost(gap: i32, glue: i32, penalty: i64) -> i64 {
    if penalty >= NO_BREAK {
        return i64::MAX;
    }
    let bad = badness(gap as i64, glue as i64);
    if bad >= INF_BAD { DEPLORABLE } else { bad + penalty }
}

#[derive(Clone, Copy)]
pub(super) struct Word {
    bx: Bx,
    forced: bool,
    gap: bool,
    hyphen: Option<Bx>,
    penalty: i16,
    literal: bool,
    after: u16,
    tied: bool,
    below: i32,
    space: Space,
}

#[derive(Clone, Copy)]
pub(super) struct Seg {
    pub(super) op: u8,
    pub(super) x: i32,
    pub(super) y: i32,
}

#[derive(Clone, Copy)]
pub(super) struct Art {
    pub(super) s: u32,
    pub(super) e: u32,
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) stroke: u32,
    pub(super) fill: u32,
    pub(super) width: i32,
    pub(super) dash: (i32, i32),
    pub(super) round: bool,
    pub(super) text: bool,
    pub(super) cos: i32,
    pub(super) sin: i32,
}

pub(super) const SEG_MOVE: u8 = 0;
pub(super) const SEG_LINE: u8 = 1;
pub(super) const SEG_CURVE: u8 = 2;
pub(super) const SEG_CLOSE: u8 = 3;
pub(super) const NO_COLOR: u32 = u32::MAX;

#[derive(Clone, Copy)]
pub(super) struct Environment {
    name: &'static [u8],
    count: usize,
    default: Option<&'static [u8]>,
    begin: &'static [u8],
    end: &'static [u8],
}

#[derive(Default)]
pub(super) struct Store {
    pic: figure::Pic,
    settings: Vec<&'static [u8]>,
    hyphenations: Vec<(Vec<u8>, Vec<usize>)>,
    notes: Vec<(u32, &'static [u8], u32)>,
    foot_items: Vec<Item>,
    environments: Vec<Environment>,
    theorems: Vec<(&'static [u8], &'static [u8], u32)>,
    groups: Vec<(Ctx, u8, bool, &'static [u8], &'static [u8], bool)>,
    contents: Vec<commands::Contents>,
    shown: Vec<commands::Contents>,
    contents_text: Vec<u8>,
    wanted: bool,
    break_best: Vec<(i64, u32, u8)>,
    held: Vec<float::Held>,
    deferred: Vec<float::Held>,
    bottom: Vec<float::Held>,
    held_items: Vec<Item>,
    gives: Vec<(u32, u32, i32)>,
    held_art: Vec<Art>,
    tail_items: Vec<Item>,
    tail_art: Vec<Art>,
    cells: Vec<(Bx, u16, u16, Vec<Word>)>,
    rows: Vec<float::TableRow>,
}

#[derive(Default)]
pub(crate) struct Laid {
    pub(super) links: Vec<Vec<u8>>,
    pub(super) items: Vec<Item>,
    pub(super) pages: Vec<u32>,
    pub(super) art: Vec<Art>,
    pub(super) segs: Vec<Seg>,
    pub(super) spun: Vec<Item>,
    pub(super) art_pages: Vec<u32>,
    pub(super) groups: Vec<Grp>,
    pub(super) colors: Vec<(u32, u32)>,
    pub(super) color_bytes: Vec<u8>,
    pub(super) w: i32,
    pub(super) h: i32,
    pub(super) pt: i32,
    // 0 = cropped standalone, 1 = US Letter, 2 = A4.
    pub(super) paper: u8,
    pub(crate) images: Vec<super::assets::Image>,
}

#[derive(Clone, Copy)]
enum Label {
    Bullet,
    Number(u32),
    Cite(u32),
    Text(&'static [u8]),
}

#[derive(Clone)]
struct Doc {
    left: i32,
    top: i32,
    text_w: i32,
    text_h: i32,
    base: i32,
    dep: i32,
    fresh: bool,
    first_line: Option<i32>,
    hyphenate: bool,
    literal_word: bool,
    sloppy: bool,
    tie_next: bool,
    open_par: bool,
    after_par: bool,
    open_heading: bool,
    after_heading: bool,
    open_vspace: bool,
    after_vspace: bool,
    page_shrink: i32,
    page_over: i32,
    page_stretch: i32,
    placed_stretch: i32,
    line_penalties: (i64, i64),
    preamble: bool,
    open_groups: u32,
    standalone: Option<i32>,
    note_h: i32,
    bottom_h: i32,
    float_counts: [u32; 3], // top, bottom, total in the current column
    body_font: u8,
    graphic_paths: &'static [u8],
    graphic_extensions: &'static [u8],
    url_setup: bool,
    lead: &'static [u8],
    flow: (i32, i32, i32),
    flowing: bool,
    table_width: i32,
    picture_unit: f64,
    bare_list: bool,
    steps: bool,
    step_level: i32,
    skip: i32,
    page: u32,
    page_style: u8,
    this_page_style: Option<u8>,
    page_fields: [[[&'static [u8]; 3]; 2]; 2],
    head_sep: i32,
    foot_skip: i32,
    noindent: bool,
    table_align: u8,
    line_fill: i32,
    center: bool,
    sections: [u32; 3],
    equation: u32,
    title: &'static [u8],
    author: &'static [u8],
    date: &'static [u8],
    word: Option<Bx>,
    glue_next: bool,
    label: Option<Label>,
    ctx: Ctx,
    columns: u8,
    column: u8,
    column_gap: i32,
    baseline: i32,
    line_end: i32,
    topskip: i32,
    frame_left: i32,
    frame_top: i32,
    frame_w: i32,
    frame_h: i32,
    parskip: i32,
    parindent: i32,
    table_gap: i32,
    float_gap: i32,
    caption_gap: i32,
    heading_gaps: [(i32, i32); 3],
    figures: u32,
    tables: u32,
    algorithms: u32,
    float: u8,
}

fn is_definition(name: &[u8]) -> bool {
    matches!(
        name,
        b"newcommand"
            | b"renewcommand"
            | b"providecommand"
            | b"def"
            | b"DeclareMathOperator"
            | b"DeclarePairedDelimiter"
            | b"NewDocumentCommand"
            | b"RenewDocumentCommand"
            | b"DeclareDocumentCommand"
            | b"ProvideDocumentCommand"
            | b"definecolor"
    )
}

fn standalone_border(options: &'static [u8], pt: i32) -> i32 {
    let mut at = 0;
    while let Some((key, value)) = figure::next_option(options, &mut at) {
        if key == b"border" {
            let first = value.split(|c| *c == b' ').next().unwrap_or(value);
            if let Some(v) = figure::length_pt(first) {
                return (v * 1000.0 / pt as f64) as i32;
            }
        }
    }
    50
}

fn block_body(body: &[u8]) -> bool {
    body.contains(&b'\\')
        && [
            &b"\\begin{"[..],
            b"\\end{",
            b"\\section",
            b"\\subsection",
            b"\\chapter",
            b"\\vspace",
            b"\\item",
            b"\\newpage",
            b"\\maketitle",
            b"\\par ",
        ]
        .iter()
        .any(|word| body.windows(word.len()).any(|w| w == *word))
}

fn open_ended(body: &[u8]) -> bool {
    let count = |word: &[u8]| body.windows(word.len()).filter(|w| *w == word).count();
    body.contains(&b'\\') && (count(b"\\item") > 0 || count(b"\\begin{") != count(b"\\end{"))
}

fn ignored_setting(name: &[u8]) -> bool {
    matches!(
        name,
        b"pagestyle"
            | b"thispagestyle"
            | b"pagenumbering"
            | b"label"
            | b"bibliographystyle"
            | b"usetikzlibrary"
            | b"pgfplotsset"
    )
}

fn font_size(name: &[u8], pt: i32) -> Option<u16> {
    let row: [i32; 10] = match pt {
        11 => [6000, 8000, 9000, 10000, 10950, 12000, 14400, 17280, 20740, 24880],
        12 => [6000, 8000, 10000, 10950, 12000, 14400, 17280, 20740, 24880, 24880],
        _ => [5000, 7000, 8000, 9000, 10000, 12000, 14400, 17280, 20740, 24880],
    };
    let at = match name {
        b"tiny" => 0,
        b"scriptsize" => 1,
        b"footnotesize" => 2,
        b"small" => 3,
        b"normalsize" => 4,
        b"large" => 5,
        b"Large" => 6,
        b"LARGE" => 7,
        b"huge" => 8,
        b"Huge" => 9,
        _ => return None,
    };
    Some((row[at] / pt) as u16)
}

fn ends_paragraph(name: &[u8]) -> bool {
    matches!(
        name,
        b"chapter"
            | b"part"
            | b"paragraph"
            | b"subparagraph"
            | b"cleardoublepage"
            | b"pagebreak"
            | b"setlength"
            | b"title"
            | b"author"
            | b"date"
            | b"bibitem"
            | b"bibliography"
            | b"bibliographystyle"
            | b"printbibliography"
            | b"nocite"
            | b"addbibresource"
            | b"tableofcontents"
            | b"frame"
            | b"newcommand"
            | b"renewcommand"
            | b"def"
            | b"let"
            | b"newenvironment"
            | b"newtheorem"
            | b"input"
            | b"include"
            | b"chemfig"
            | b"frametitle"
            | b"titlepage"
            | b"resizebox"
            | b"scalebox"
            | b"section"
            | b"subsection"
            | b"subsubsection"
            | b"begin"
            | b"end"
            | b"item"
            | b"["
            | b"newpage"
            | b"clearpage"
            | b"maketitle"
            | b"par"
            | b"noindent"
            | b"bigskip"
            | b"medskip"
            | b"smallskip"
            | b"vspace"
            | b"caption"
    )
}

fn contains(text: &[u8], part: &[u8]) -> bool {
    text.windows(part.len()).any(|w| w == part)
}

impl Engine {
    pub fn render_document(&mut self, len: usize) -> i32 {
        let source = unsafe { core::slice::from_raw_parts(self.input.as_ptr(), len) };
        match super::bibliography::Bibliography::prepare(source, &self.assets.files) {
            Ok(bibliography) => self.assets.bibliography = bibliography,
            Err((message, at)) => {
                self.out.clear();
                self.out.extend_from_slice(&message);
                self.err_pos = at;
                return -(self.out.len() as i32);
            }
        }
        self.labels.clear();
        self.label_bytes.clear();
        self.cites.clear();
        self.store.contents.clear();
        self.store.shown.clear();
        self.store.contents_text.clear();
        self.store.wanted = false;
        self.unresolved = false;
        let first = self.layout_document(len);
        if !self.unresolved || first < 0 {
            if first >= 0 {
                self.warn_missing_citations();
            }
            return first;
        }
        self.next_pass();
        let second = self.layout_document(len);
        if !self.store.wanted || second < 0 || self.store.contents == self.store.shown {
            if second >= 0 {
                self.warn_missing_citations();
            }
            return second;
        }
        self.next_pass();
        let final_pass = self.layout_document(len);
        if final_pass >= 0 {
            self.warn_missing_citations();
        }
        final_pass
    }

    fn next_pass(&mut self) {
        self.cites.clear();
        core::mem::swap(&mut self.store.contents, &mut self.store.shown);
        self.store.contents.clear();
    }

    fn layout_document(&mut self, len: usize) -> i32 {
        self.reset(len);
        self.initialize_counters();
        if self.assets.bibliography.uncertain() {
            self.note(self.assets.bibliography.warning_position());
        }
        self.lenient = true;
        self.mark = (0, [0; 3]);
        self.spliced = 0;
        self.words.clear();
        self.doc.items.clear();
        self.doc.links.clear();
        self.doc.pages.clear();
        self.doc.art.clear();
        self.doc.segs.clear();
        self.doc.spun.clear();
        self.doc.images.clear();
        self.assets.active.clear();
        self.doc.art_pages.clear();
        self.store.held.clear();
        self.store.deferred.clear();
        self.store.bottom.clear();
        self.store.settings.clear();
        self.store.notes.clear();
        self.store.hyphenations.clear();
        self.store.foot_items.clear();
        self.store.groups.clear();
        self.store.environments.clear();
        self.store.theorems.clear();
        self.store.held_items.clear();
        self.store.held_art.clear();
        let ctx = Ctx { style: ST_T, font: 0, size: UNIT, scale: UNIT, color: 0, grp: 0 };
        let mut d = Doc {
            left: 0,
            top: 0,
            text_w: 0,
            text_h: 0,
            base: 0,
            dep: 0,
            fresh: true,
            preamble: false,
            open_groups: 0,
            standalone: None,
            note_h: 0,
            bottom_h: 0,
            body_font: 0,
            graphic_paths: b"",
            url_setup: false,
            graphic_extensions: b".pdf,.png,.jpg,.mps,.jpeg,.jbig2,.jb2,.PDF,.PNG,.JPG,.JPEG,.JBIG2,.JB2",
            lead: b"",
            flow: (0, 0, 0),
            flowing: false,
            table_width: 0,
            picture_unit: 1.0,
            bare_list: false,
            steps: false,
            step_level: 0,
            skip: 0,
            page: 1,
            page_style: 1,
            this_page_style: None,
            page_fields: [[[b"", b"", b""], [b"", b"\\thepage", b""]]; 2],
            head_sep: 2500,
            foot_skip: 3000,
            noindent: false,
            table_align: b'c',
            line_fill: 0,
            center: false,
            sections: [0; 3],
            equation: 0,
            title: b"",
            author: b"",
            date: b"",
            word: None,
            glue_next: false,
            label: None,
            ctx,
            columns: 1,
            column: 0,
            column_gap: 0,
            baseline: 1200,
            first_line: None,
            hyphenate: true,
            literal_word: false,
            sloppy: false,
            tie_next: false,
            open_par: false,
            after_par: false,
            open_heading: false,
            after_heading: false,
            open_vspace: false,
            after_vspace: false,
            page_shrink: 0,
            page_over: 0,
            page_stretch: 0,
            placed_stretch: 0,
            line_penalties: (0, 0),
            line_end: 0,
            topskip: TOPSKIP,
            frame_left: 0,
            frame_top: 0,
            frame_w: 0,
            frame_h: 0,
            parskip: 0,
            parindent: PARINDENT,
            table_gap: 0,
            float_gap: 0,
            caption_gap: 0,
            heading_gaps: [(1507, 990), (1399, 646), (1399, 646)],
            figures: 0,
            tables: 0,
            algorithms: 0,
            float: 0,
            float_counts: [0; 3],
        };
        self.set_paper(&mut d, b"");
        let done = self.blocks(&mut d, 0, 0, b"");
        match done.and_then(|()| {
            self.finish_page(&mut d)?;
            while !self.store.held.is_empty() || !self.store.deferred.is_empty() {
                d.page = self.increment_counter(b"page")?.max(0) as u32;
                d.column = 0;
                self.fit_frame(&mut d);
                d.float_counts = [0; 3];
                // Float-only pages must progress even when text-page limits are zero.
                self.place_held(&mut d, true);
                self.place_deferred(&mut d, true);
                self.finish_page(&mut d)?;
            }
            Ok(())
        }) {
            Ok(()) => {
                self.doc.groups.clone_from(&self.groups);
                self.doc.colors.clone_from(&self.colors);
                self.doc.color_bytes.clone_from(&self.color_bytes);
                core::mem::swap(&mut self.doc, &mut self.laid);
                self.laid.pages.len() as i32
            }
            Err(()) => {
                self.write_error();
                -(self.out.len() as i32)
            }
        }
    }

    pub fn document_page(&mut self, page: usize) -> i32 {
        if page >= self.laid.pages.len() {
            return 0;
        }
        let s = if page == 0 { 0 } else { self.laid.pages[page - 1] as usize };
        let e = self.laid.pages[page] as usize;
        self.items.clear();
        self.items.extend_from_slice(&self.laid.items[s..e]);
        self.groups.clone_from(&self.laid.groups);
        self.colors.clone_from(&self.laid.colors);
        self.color_bytes.clone_from(&self.laid.color_bytes);
        let from = if page == 0 { 0 } else { self.laid.art_pages[page - 1] as usize };
        let to = self.laid.art_pages[page] as usize;
        self.art_svg(from, to);
        let n = self.end();
        self.emit_as(Bx { s: 0, e: n, w: self.laid.w, h: 0, d: self.laid.h }, true);
        self.out.len() as i32
    }

    fn set_paper(&mut self, d: &mut Doc, options: &[u8]) {
        let (pt, text_w) = if contains(options, b"12pt") {
            (12, 390)
        } else if contains(options, b"11pt") {
            (11, 360)
        } else {
            (10, 345)
        };
        let a4 = contains(options, b"a4paper");
        let (frame_top, frame_h) = match (pt, a4) {
            (11, false) => (130, 541),
            (12, false) => (126, 548),
            (11, true) => (127, 596),
            (12, true) => (129, 592),
            (_, true) => (126, 598),
            _ => (125, 550),
        };
        let unit = |v: i32| v * 1000 / pt;
        let two = contains(options, b"twocolumn");
        self.doc.pt = pt;
        // Layout lengths are TeX points (72.27/in), while paper sizes and
        // PDF coordinates use big points (72/in). Keep the conversion here
        // and at PDF output, rather than mixing the two in the text frame.
        self.doc.paper = if a4 { 2 } else { 1 };
        self.doc.w = if a4 { 597508 } else { 614295 } / pt;
        self.doc.h = if a4 { 845047 } else { 794970 } / pt;
        d.columns = if two { 2 } else { 1 };
        d.frame_w = unit(if two { 470 } else { text_w });
        // \\@settopoint leaves the side margin a whole number of points.
        let paper = if a4 { 597508 } else { 614295 };
        let side = (paper - 2 * INCH - d.frame_w * pt) / 2 / 1000 * 1000;
        d.frame_left = (INCH + side) / pt;
        d.frame_h = unit(frame_h);
        d.frame_top = unit(frame_top);
        d.column_gap = unit(10);
        d.baseline = match pt {
            11 => 1236,
            12 => 1208,
            _ => 1200,
        };
        // The 11pt class actually selects a 10.95pt normal text font.
        d.ctx = self.sized(d, font_size(b"normalsize", pt).unwrap_or(1000));
        d.parindent = unit(match pt {
            11 => 17,
            12 => 18,
            _ => 15,
        });
        d.table_gap = unit(6);
        d.float_gap = unit(12);
        d.caption_gap = unit(10);
        self.fit_frame(d);
    }

    fn fit_frame(&mut self, d: &mut Doc) {
        d.top = d.frame_top;
        d.text_h = d.frame_h;
        if d.columns == 2 {
            d.text_w = (d.frame_w - d.column_gap) / 2;
            d.left = d.frame_left + d.column as i32 * (d.text_w + d.column_gap);
        } else {
            d.text_w = d.frame_w;
            d.left = d.frame_left;
        }
    }

    fn set_margins(&mut self, d: &mut Doc, options: &'static [u8]) {
        let mut at = 0;
        let (mut left, mut right, mut top, mut bottom) = (None, None, None, None);
        while let Some((key, value)) = figure::next_option(options, &mut at) {
            if !matches!(
                key,
                b"margin"
                    | b"hmargin"
                    | b"vmargin"
                    | b"left"
                    | b"lmargin"
                    | b"right"
                    | b"rmargin"
                    | b"top"
                    | b"tmargin"
                    | b"bottom"
                    | b"bmargin"
            ) {
                self.note_slice(key);
                continue;
            }
            let Some(v) = figure::length_pt(value) else {
                self.note_slice(key);
                continue;
            };
            let v = Some(self.pt_units(v));
            match key {
                b"margin" => (left, right, top, bottom) = (v, v, v, v),
                b"hmargin" => (left, right) = (v, v),
                b"vmargin" => (top, bottom) = (v, v),
                b"left" | b"lmargin" => left = v,
                b"right" | b"rmargin" => right = v,
                b"top" | b"tmargin" => top = v,
                b"bottom" | b"bmargin" => bottom = v,
                _ => {}
            }
        }
        if left.is_some() || right.is_some() {
            let l = left.or(right).unwrap_or(0);
            let r = right.or(left).unwrap_or(0);
            d.frame_left = l;
            d.frame_w = self.doc.w - l - r;
        }
        if top.is_some() || bottom.is_some() {
            let t = top.or(bottom).unwrap_or(0);
            let b = bottom.or(top).unwrap_or(0);
            d.frame_top = t;
            d.frame_h = self.doc.h - t - b;
        }
        self.fit_frame(d);
    }

    fn pt_units(&self, v: f64) -> i32 {
        (v * 1000.0 / self.doc.pt as f64) as i32
    }

    fn package_line(&mut self, d: &mut Doc) -> R<()> {
        self.skip_ws();
        let options = if self.peek() == b'[' { self.loose_options() } else { b"" };
        let names = self.group_text()?;
        for part in names.split(|b| *b == b',') {
            let name = part.trim_ascii();
            self.package_diagnostics(name, options);
            match name {
                b"" | b"tikz" | b"pgfplots" => {}
                b"geometry" => self.set_margins(d, options),
                b"biblatex" => self.initialize_biblatex_counters(),
                b"parskip" => {
                    d.parskip = 618;
                    d.parindent = 0;
                }
                _ if crate::packages::is_noop(name) => {}
                _ => match crate::packages::find_package(name) {
                    Some(i) => self.load_package(i)?,
                    None => {}
                },
            }
        }
        Ok(())
    }

    // Loading a package is not proof that its layout or options are implemented.
    fn package_diagnostics(&mut self, name: &[u8], options: &'static [u8]) {
        let partial = matches!(
            name,
            b"babel"
                | b"microtype"
                | b"etoolbox"
                | b"subcaption"
                | b"mathptmx"
                | b"newtxmath"
                | b"lmodern"
                | b"hyperref"
                | b"mhchem"
                | b"siunitx"
                | b"listings"
                | b"tcolorbox"
        );
        let built_in = matches!(
            name,
            b"tikz"
                | b"pgfplots"
                | b"parskip"
                | b"longtable"
                | b"tabularx"
                | b"tabulary"
                | b"supertabular"
                | b"xltabular"
                | b"listings"
                | b"tcolorbox"
                | b"fancyhdr"
                | b"natbib"
        );
        if partial || (!built_in && !crate::packages::is_noop(name) && crate::packages::find_package(name).is_none() && !name.is_empty()) {
            self.note_slice(name);
        }
        if name == b"geometry" {
            return;
        } // set_margins diagnoses each ignored key.
        let mut at = 0;
        while let Some((key, _)) = figure::next_option(options, &mut at) {
            let supported = match name {
                b"inputenc" => key == b"utf8",
                b"fontenc" => key == b"T1",
                b"natbib" => matches!(key, b"numbers" | b"authoryear"),
                _ => false,
            };
            if !supported {
                self.note_slice(key);
            }
        }
    }

    fn note_slice(&mut self, text: &[u8]) {
        let at = (text.as_ptr() as usize).checked_sub(self.p as usize).filter(|at| *at <= self.len).unwrap_or(self.pos);
        self.note(at);
    }

    fn set_length(&mut self, d: &mut Doc) -> R<()> {
        let name = self.loose_argument();
        let value = self.loose_argument();
        let Some(v) = figure::length_pt(value) else {
            self.note_slice(value);
            return Ok(());
        };
        let v = self.pt_units(v);
        match name {
            b"\\columnsep" => {
                d.column_gap = v;
                self.fit_frame(d);
            }
            b"\\parindent" => d.parindent = v,
            b"\\parskip" => d.parskip = v,
            b"\\tabcolsep" => d.table_gap = v,
            b"\\headsep" => d.head_sep = v,
            b"\\footskip" => d.foot_skip = v,
            b"\\intextsep" => d.float_gap = v,
            b"\\unitlength" => d.picture_unit = v as f64 * self.doc.pt as f64 / 1000.0,
            _ => self.note_slice(name),
        }
        Ok(())
    }

    fn title_spacing(&mut self, d: &mut Doc) -> R<()> {
        self.skip_star();
        let level = self.group_text()?;
        self.group_text()?;
        let before = self.group_text()?;
        let after = self.group_text()?;
        let at = match level {
            b"\\section" => 0,
            b"\\subsection" => 1,
            b"\\subsubsection" => 2,
            _ => return Ok(()),
        };
        let first = |t: &'static [u8]| &t[..t.iter().position(|c| *c == b' ').unwrap_or(t.len())];
        if let (Some(b), Some(a)) = (figure::length_pt(first(before)), figure::length_pt(first(after))) {
            d.heading_gaps[at] = (self.pt_units(b), self.pt_units(a));
        }
        Ok(())
    }

    fn block_reset(&mut self) {
        self.items.clear();
        self.reparses = 0;
        self.expansions = 0;
        self.exp_bytes = 0;
    }

    fn skip_argument(&mut self) -> R<()> {
        self.skip_ws();
        if self.peek() == b'[' {
            self.bracketed()?;
            self.skip_ws();
        }
        if self.peek() == b'{' {
            self.balanced()?;
        }
        Ok(())
    }

    fn group_text(&mut self) -> R<&'static [u8]> {
        self.skip_ws();
        if self.peek() != b'{' {
            return self.fail(E_ARG);
        }
        self.balanced()
    }

    fn blank_ahead(&self) -> bool {
        let mut i = self.pos;
        let mut lines = 0;
        while i < self.len {
            match unsafe { *self.p.add(i) } {
                b'\n' => lines += 1,
                b' ' | b'\t' | b'\r' => {}
                _ => break,
            }
            i += 1;
        }
        lines >= 2 || i >= self.len
    }

    fn blocks(&mut self, d: &mut Doc, left: i32, narrow: i32, env: &[u8]) -> R<()> {
        self.enter()?;
        loop {
            self.block_reset();
            self.skip_ws();
            d.after_par = core::mem::take(&mut d.open_par);
            d.after_heading = core::mem::take(&mut d.open_heading);
            d.after_vspace = core::mem::take(&mut d.open_vspace);
            if self.at_end() {
                if env.is_empty() {
                    self.depth -= 1;
                    return Ok(());
                }
                return self.fail(E_END);
            }
            let width = d.text_w - narrow;
            if d.preamble && self.peek() != b'\\' {
                self.pos += 1;
                continue;
            }
            if self.peek() == b'$' && self.peek_at(1) == b'$' {
                self.pos += 2;
                let c = self.restyle(&d.ctx.clone(), ST_D);
                let b = self.display_until(&c, b"$$")?;
                self.place_display(d, b, left, width, None);
                self.after_display(d);
                continue;
            }
            if self.peek() == b'}' && d.open_groups > 0 {
                d.open_groups -= 1;
                self.pos += 1;
                if let Some((ctx, font, center, paths, extensions, urls)) = self.store.groups.pop() {
                    (d.ctx, d.body_font, d.center, d.graphic_paths, d.graphic_extensions, d.url_setup) = (ctx, font, center, paths, extensions, urls);
                }
                continue;
            }
            if self.peek() == b'{' && self.group_holds_blocks() {
                self.pos += 1;
                self.open_group(d);
                continue;
            }
            if self.peek() != b'\\' {
                let state = (self.p, self.len, self.user_src, self.depth, self.pos);
                if self.paragraph(d, left, width).is_err() {
                    self.recover(d, state, None)?;
                }
                continue;
            }
            let start = self.pos;
            self.pos += 1;
            let name = self.read_name();
            if self.control_command(name)? || self.page_setting(d, name)? {
                Self::keep_spacing(d);
                continue;
            }
            match name {
                b"item" | b"bibitem" => {
                    if !matches!(
                        env,
                        b"itemize" | b"enumerate" | b"description" | b"thebibliography" | b"itemize*" | b"enumerate*" | b"description*"
                    ) {
                        self.note(start);
                        self.skip_options();
                        continue;
                    }
                    self.pos = start;
                    self.depth -= 1;
                    return Ok(());
                }
                b"end" => {
                    let (s, e) = self.raw_group()?;
                    let found = self.slice(s, e);
                    if env.is_empty() && found == b"document" {
                        self.pos = self.len;
                        continue;
                    }
                    if found != env {
                        self.pos = start;
                        return self.fail(E_ENV);
                    }
                    self.pos = start;
                    self.depth -= 1;
                    return Ok(());
                }
                b"begin" => {
                    let state = (self.p, self.len, self.user_src, self.depth, start);
                    let held = self.pos;
                    if self.begin_block(d, left, narrow, start).is_err() {
                        self.pos = held;
                        let inner = self.loose_argument();
                        self.pos = start;
                        self.recover(d, state, Some(inner))?;
                    }
                }
                b"bibliographystyle" | b"addbibresource" | b"nocite" => {
                    self.skip_options();
                    let argument = self.loose_argument();
                    if name == b"addbibresource" && !self.assets.bibliography.has_resource(argument) {
                        self.note(start);
                    }
                }
                b"bibliography" | b"printbibliography" => {
                    if name == b"bibliography" {
                        let argument = self.loose_argument();
                        if !self.assets.bibliography.has_resource(argument) {
                            self.note(start);
                        }
                    } else {
                        self.skip_options();
                    }
                    if !self.assets.bibliography.is_active() {
                        self.note(start);
                    }
                    let text = self.assets.bibliography.document();
                    let text = self.source_copy(&text);
                    let source = self.push_src(text.as_ptr(), text.len());
                    let done = self.blocks(d, left, narrow, b"");
                    self.pop_src(source);
                    done?;
                }
                b"biburlsetup" => d.url_setup = true,
                b"graphicspath" | b"DeclareGraphicsExtensions" => self.graphic_setting(d, name)?,
                b"makeatletter" => self.at_letter = true,
                b"makeatother" => self.at_letter = false,
                b"documentclass" => {
                    self.skip_ws();
                    let options = if self.peek() == b'[' { self.loose_options() } else { b"" };
                    let class = self.loose_argument();
                    if !matches!(class, b"article" | b"report" | b"book" | b"standalone") {
                        self.note(start);
                    }
                    let mut at = 0;
                    while let Some((key, _)) = figure::next_option(options, &mut at) {
                        if !matches!(key, b"10pt" | b"11pt" | b"12pt" | b"a4paper" | b"letterpaper" | b"onecolumn" | b"twocolumn")
                            && !(class == b"standalone" && key == b"border")
                        {
                            self.note(start);
                        }
                    }
                    self.set_paper(d, options);
                    d.preamble = true;
                    if class == b"standalone" {
                        d.standalone = Some(standalone_border(options, self.doc.pt));
                    }
                }
                b"title" => {
                    self.skip_ws();
                    self.skip_options();
                    d.title = self.loose_argument();
                }
                b"author" => {
                    self.skip_ws();
                    self.skip_options();
                    d.author = self.loose_argument();
                }
                b"date" => {
                    self.skip_ws();
                    self.skip_options();
                    d.date = self.loose_argument();
                }
                b"maketitle" => self.title_block(d)?,
                b"section" => self.heading(d, 0, left, width)?,
                b"subsection" => self.heading(d, 1, left, width)?,
                b"subsubsection" => self.heading(d, 2, left, width)?,
                b"chapter" | b"part" => {
                    if !d.fresh {
                        self.new_page(d)?;
                    }
                    self.heading(d, 0, left, width)?;
                }
                b"paragraph" | b"subparagraph" => self.heading(d, 3, left, width)?,
                b"newenvironment" | b"renewenvironment" => self.define_environment()?,
                b"newtheorem" => self.define_theorem()?,
                b"newpage" | b"clearpage" | b"cleardoublepage" | b"pagebreak" => {
                    if !d.fresh {
                        self.new_page(d)?;
                    }
                }
                b"par" | b"raggedbottom" | b"flushbottom" | b"raggedright" | b"protect" => {}
                b"sloppy" => d.sloppy = true,
                b"fussy" => d.sloppy = false,
                b"centering" => d.center = true,
                b"hyphenation" => self.hyphenation_exceptions()?,
                b"caption" => self.caption(d, left, width)?,
                b"chemfig" => self.chemfig(d, left, width)?,
                b"usepackage" | b"RequirePackage" => self.package_line(d)?,
                b"setlength" => self.set_length(d)?,
                b"geometry" => {
                    let options = self.group_text()?;
                    self.set_margins(d, options);
                }
                b"titlespacing" => self.title_spacing(d)?,
                b"captionsetup" => {
                    self.skip_ws();
                    self.skip_options();
                    let options = self.group_text()?;
                    let mut at = 0;
                    while let Some((key, value)) = figure::next_option(options, &mut at) {
                        if key == b"skip" {
                            if let Some(v) = figure::length_pt(value) {
                                d.caption_gap = self.pt_units(v);
                            } else {
                                self.note_slice(key);
                            }
                        } else {
                            self.note_slice(key);
                        }
                    }
                }
                b"noindent" => d.noindent = true,
                b"smallskip" => d.skip += 300,
                b"medskip" => d.skip += 600,
                b"bigskip" => d.skip += 1200,
                b"vspace" => {
                    self.skip_star();
                    let c = d.ctx;
                    d.skip += self.length_arg(&c)?;
                    d.open_vspace = true;
                }
                b"[" => {
                    let c = self.restyle(&d.ctx.clone(), ST_D);
                    let b = self.display_until(&c, b"\\]")?;
                    self.place_display(d, b, left, width, None);
                    self.after_display(d);
                }
                b"resizebox" | b"scalebox" | b"rotatebox" => {
                    self.note(start);
                    self.skip_star();
                    self.skip_ws();
                    self.skip_options();
                    self.loose_argument();
                    if name == b"resizebox" {
                        self.loose_argument();
                    }
                    self.skip_ws();
                    if self.peek() == b'{' {
                        self.pos += 1;
                        self.open_group(d);
                    }
                }
                b"frame" => {
                    self.skip_overlay();
                    self.skip_ws();
                    self.skip_options();
                    self.skip_ws();
                    if !d.fresh {
                        self.new_page(d)?;
                    }
                    if self.peek() == b'{' {
                        self.pos += 1;
                        self.open_group(d);
                    }
                }
                b"tableofcontents" => {
                    self.unresolved = true;
                    self.store.wanted = true;
                    self.contents(d, left, width)?;
                }
                b"frametitle" | b"framesubtitle" => {
                    let text = self.loose_argument();
                    let text = self.source_copy(text);
                    self.block_reset();
                    let c = self.sized(d, font_size(b"large", self.doc.pt).unwrap_or(1200));
                    self.sub_text(d, text, &c, FONT_MAIN_BOLD)?;
                    self.flush_word(d, false);
                    self.set_lines(d, &c, left, width, 0, true, false);
                    d.skip += 600;
                }
                b"titlepage" => self.title_block(d)?,
                b"\\" => {
                    self.skip_star();
                    d.skip += self.bracket_length(&d.ctx.clone())?;
                }
                b"hyphenpenalty" | b"exhyphenpenalty" => {
                    self.skip_ws();
                    if self.peek() == b'=' {
                        self.pos += 1;
                    }
                    let at = self.pos;
                    self.skip_ws();
                    while self.peek().is_ascii_digit() || self.peek() == b'-' {
                        self.pos += 1;
                    }
                    if name == b"hyphenpenalty" {
                        d.hyphenate = figure::length_pt(self.slice(at, self.pos)).is_none_or(|v| v < 10000.0);
                    }
                }
                b"parindent" | b"parskip" | b"itemsep" | b"topsep" | b"parsep" | b"baselineskip" | b"tabcolsep" | b"arraystretch"
                | b"columnsep" => {
                    self.skip_ws();
                    if self.peek() == b'=' {
                        self.pos += 1;
                    }
                    self.skip_ws();
                    while !self.at_end() && !matches!(self.peek(), b'\n' | b' ' | b'\\' | b'%') {
                        self.pos += 1;
                    }
                }
                _ if is_definition(name) => {
                    let mut c = d.ctx;
                    let mut r = self.new_run(0, NONE, c);
                    let after = self.pos;
                    if self.core(name, &mut r, &mut c).is_err() {
                        self.err = 0;
                        self.pos = after;
                        self.note(start);
                        self.skip_arguments();
                    }
                }
                _ if ignored_setting(name) && name != b"label" => {
                    if matches!(name, b"usetikzlibrary" | b"pgfplotsset") {
                        let options = self.loose_argument();
                        if name == b"usetikzlibrary" {
                            for library in options.split(|c| *c == b',') {
                                if !matches!(
                                    library.trim_ascii(),
                                    b"calc"
                                        | b"arrows"
                                        | b"arrows.meta"
                                        | b"positioning"
                                        | b"shapes"
                                        | b"shapes.geometric"
                                        | b"patterns"
                                        | b"backgrounds"
                                        | b"fit"
                                ) {
                                    self.note_slice(library);
                                }
                            }
                        } else {
                            let mut at = 0;
                            while let Some((key, _)) = figure::next_option(options, &mut at) {
                                if key != b"compat" {
                                    self.note_slice(key);
                                }
                            }
                        }
                    } else {
                        self.note(start);
                        self.skip_argument()?;
                    }
                }
                b"label" => {
                    let key = self.loose_argument();
                    self.add_label(key);
                }
                b"today" | b"hline" | b"cline" => {
                    self.skip_arguments();
                }
                b"input" | b"include" => {
                    self.input_file(d, left, narrow, name == b"include")?;
                }
                b"let" => self.alias()?,
                _ if !d.preamble && lookup(name).is_some_and(|sy| sy.kind == K_TEXT || sy.kind == K_SYM) => {
                    self.pos = start;
                    let state = (self.p, self.len, self.user_src, self.depth, start);
                    if self.paragraph(d, left, width).is_err() {
                        self.recover(d, state, None)?;
                    }
                }
                _ if commands::silent_setting(name) => {
                    self.note(start);
                    self.skip_arguments();
                }
                b"tikzstyle" => self.preamble_style()?,
                b"tikzset" => {
                    let text = self.group_text()?;
                    self.store.settings.push(text);
                }
                _ if self.conditional(name)? => {}
                _ if d.preamble => {
                    self.note(start);
                    self.skip_arguments();
                }
                _ if d.steps && commands::step(name).is_some() => {
                    let state = (self.p, self.len, self.user_src, self.depth, start);
                    if self.algorithm_line(d, left, width, name).is_err() {
                        self.recover(d, state, None)?;
                    }
                }
                _ => {
                    if let Some(size) = font_size(name, self.doc.pt) {
                        d.ctx = self.sized(d, size);
                        continue;
                    }
                    if let Some(def) = self.find_def(name) {
                        if open_ended(def.body) {
                            let body = self.expansion(&def, false)?;
                            self.splice(&body[..body.len() - 1])?;
                            Self::keep_spacing(d);
                            continue;
                        }
                        if block_body(def.body) {
                            self.enter()?;
                            Self::keep_spacing(d);
                            let body = self.expansion(&def, false)?;
                            let body = self.source_copy(&body[..body.len() - 1]);
                            let src = self.push_src(body.as_ptr(), body.len());
                            self.blocks(d, left, narrow, b"")?;
                            self.pop_src(src);
                            self.depth -= 1;
                            continue;
                        }
                    }
                    self.pos = start;
                    let state = (self.p, self.len, self.user_src, self.depth, start);
                    if self.paragraph(d, left, width).is_err() {
                        self.recover(d, state, None)?;
                    }
                }
            }
        }
    }

    fn close_env(&mut self, name: &[u8]) -> R<()> {
        self.skip_ws();
        self.pos += b"\\end".len();
        let (s, e) = self.raw_group()?;
        if self.slice(s, e) != name {
            return self.fail(E_ENV);
        }
        Ok(())
    }

    fn begin_block(&mut self, d: &mut Doc, left: i32, narrow: i32, start: usize) -> R<()> {
        let settings = (d.graphic_paths, d.graphic_extensions, d.url_setup);
        let done = self.begin_block_content(d, left, narrow, start);
        (d.graphic_paths, d.graphic_extensions, d.url_setup) = settings;
        done
    }

    fn begin_block_content(&mut self, d: &mut Doc, left: i32, narrow: i32, start: usize) -> R<()> {
        let (s, e) = self.raw_group()?;
        let name = self.slice(s, e);
        let width = d.text_w - narrow;
        if d.preamble && name != b"document" {
            self.skip_environment(name);
            return Ok(());
        }
        match name {
            b"document" => d.preamble = false,
            b"itemize" | b"enumerate" | b"description" | b"thebibliography" | b"itemize*" | b"enumerate*" | b"description*" => {
                self.skip_ws();
                if self.peek() == b'[' {
                    d.bare_list = figure::blank_label(self.loose_options());
                }
                if name == b"thebibliography" {
                    d.url_setup |= self.counter_index(b"biburlbreakpenalty").is_some();
                    self.skip_ws();
                    if self.peek() == b'{' {
                        self.loose_argument();
                    }
                    self.block_reset();
                    let c = self.sized(d, font_size(b"Large", self.doc.pt).unwrap_or(1440));
                    for ch in b"References" {
                        self.put_char(d, &c, FONT_MAIN_BOLD, *ch as u32)?;
                    }
                    self.flush_word(d, false);
                    d.skip += 1200;
                    self.set_lines(d, &c, left, width, 0, true, false);
                    d.skip += 600;
                }
                self.list_block(d, left, narrow, name)?
            }
            b"displaymath" | b"math" => {
                let c = self.restyle(&d.ctx.clone(), ST_D);
                let close: &[u8] = if name == b"math" { b"\\end{math}" } else { b"\\end{displaymath}" };
                let b = self.display_until(&c, close)?;
                self.place_display(d, b, left, width, None);
                self.after_display(d);
            }
            b"CCSXML" => self.skip_environment(name),
            b"verbatim" | b"verbatim*" | b"Verbatim" | b"lstlisting" | b"minted" | b"alltt" | b"comment" => {
                if name == b"comment" {
                    self.skip_environment(name);
                } else {
                    self.verbatim_block(d, left, width, name)?;
                }
            }
            b"flushleft" | b"flushright" | b"titlepage" | b"spacing" | b"singlespace" | b"doublespace" | b"onehalfspace" | b"multicols"
            | b"multicols*" | b"columns" | b"column" | b"landscape" | b"small" | b"footnotesize" | b"scriptsize" | b"large" | b"Large"
            | b"sloppypar" | b"subequations" | b"adjustbox" | b"preview" | b"appendices" | b"keywords" | b"acks" | b"abstract*"
            | b"resizebox" | b"wrapfigure" | b"wraptable" | b"subfigure" | b"subtable" | b"mdframed" | b"tcolorbox" | b"block"
            | b"alertblock" | b"exampleblock" | b"frame" | b"sidewaystable" | b"sidewaysfigure" | b"threeparttable" | b"tablenotes"
            | b"textblock" | b"textblock*" | b"letter" | b"otherlanguage" => {
                if matches!(
                    name,
                    b"landscape"
                        | b"subequations"
                        | b"adjustbox"
                        | b"resizebox"
                        | b"wrapfigure"
                        | b"wraptable"
                        | b"subfigure"
                        | b"subtable"
                        | b"mdframed"
                        | b"tcolorbox"
                        | b"sidewaystable"
                        | b"sidewaysfigure"
                        | b"textblock"
                        | b"textblock*"
                        | b"letter"
                        | b"otherlanguage"
                ) {
                    self.note(start);
                }
                self.skip_arguments();
                if name == b"frame" && !d.fresh {
                    self.new_page(d)?;
                }
                self.blocks(d, left, narrow, name)?;
                self.close_env(name)?;
            }
            b"minipage" => self.minipage_row(d, left, width)?,
            b"proof" => {
                self.skip_options();
                d.lead = b"Proof.";
                self.blocks(d, left, narrow, name)?;
                self.close_env(name)?;
            }
            b"tabularx" | b"tabulary" | b"tabular*" | b"longtable" | b"longtable*" | b"xltabular" | b"supertabular" => {
                self.skip_options();
                if name != b"longtable" && name != b"longtable*" && name != b"supertabular" {
                    let text = self.loose_argument();
                    d.table_width = self.table_width(text, width);
                }
                self.table_block(d, left, width, name)?;
            }
            b"algorithmic" => {
                let outer = d.steps;
                d.steps = true;
                d.step_level = 0;
                self.skip_options();
                self.blocks(d, left, narrow, name)?;
                self.close_env(name)?;
                d.steps = outer;
            }
            b"picture" | b"pspicture" | b"forest" => self.legacy_picture(d, left, width, name)?,
            b"pgfpicture" | b"filecontents" | b"filecontents*" | b"asy" | b"gnuplot" => {
                self.note(start);
                self.skip_environment(name);
            }
            b"center" => {
                let outer = d.center;
                let gap = CENTER_GAP + if d.after_par { 0 } else { PARTOP_GAP };
                let give = CENTER_GIVE + if d.after_par { 0 } else { PARTOP_GIVE };
                let stretch = CENTER_STRETCH + if d.after_par { 0 } else { PARTOP_STRETCH };
                d.center = true;
                if !d.after_heading {
                    self.add_space(d, gap, stretch, give);
                }
                self.blocks(d, left, narrow, name)?;
                self.close_env(name)?;
                d.center = outer;
                self.add_space(d, gap, stretch, give);
                d.noindent = !self.blank_ahead();
            }
            b"quote" | b"quotation" => self.quote_block(d, left, narrow, name)?,
            b"figure" | b"figure*" | b"table" | b"table*" | b"algorithm" | b"algorithm*" => self.float_block(d, left, narrow, name)?,
            b"tabular" => self.table_block(d, left, width, name)?,
            b"tikzpicture" => self.picture(d, left, width)?,
            b"abstract" => {
                d.skip += 600;
                self.block_reset();
                let mut c = d.ctx;
                let mut font = FONT_MAIN_BOLD;
                for ch in b"Abstract" {
                    self.put_char(d, &c, font, *ch as u32)?;
                }
                font = 0;
                c.font = font;
                self.flush_word(d, false);
                self.set_lines(d, &c, left, width, 0, true, true);
                d.skip += 300;
                self.quote_block(d, left, narrow, name)?;
            }
            b"equation" | b"equation*" | b"align" | b"align*" | b"gather" | b"gather*" | b"aligned" | b"gathered" | b"split" => {
                self.pos = start + b"\\begin".len();
                let c = self.restyle(&d.ctx.clone(), ST_D);
                let b = if matches!(name, b"equation" | b"equation*") {
                    self.raw_group()?;
                    let close: &[u8] = if name == b"equation" { b"\\end{equation}" } else { b"\\end{equation*}" };
                    self.display_until(&c, close)?
                } else {
                    self.environment(&c)?.bx
                };
                let number = if name == b"equation" {
                    d.equation = self.increment_counter(b"equation")?.max(0) as u32;
                    self.set_marker_number(d.equation);
                    Some(d.equation)
                } else {
                    None
                };
                let body = self.slice(start, self.pos);
                self.scan_labels(body);
                self.place_display(d, b, left, width, number);
                self.after_display(d);
            }
            _ => {
                if let Some(row) = self.store.environments.iter().rev().find(|row| row.name == name).copied() {
                    return self.run_environment(d, left, narrow, row);
                }
                if let Some(i) = self.store.theorems.iter().position(|row| row.0 == name) {
                    return self.theorem_block(d, left, narrow, i, name);
                }
                self.note(start);
                self.skip_options();
                self.blocks(d, left, narrow, name)?;
                self.close_env(name)?;
            }
        }
        Ok(())
    }

    fn theorem_block(&mut self, d: &mut Doc, left: i32, narrow: i32, index: usize, name: &'static [u8]) -> R<()> {
        self.store.theorems[index].2 += 1;
        let (_, title, count) = self.store.theorems[index];
        self.skip_options();
        let mut lead = Vec::with_capacity(title.len() + 8);
        lead.extend_from_slice(title);
        lead.push(b' ');
        self.set_marker_number(count);
        let mut digits = count;
        let at = lead.len();
        loop {
            lead.insert(at, b'0' + (digits % 10) as u8);
            digits /= 10;
            if digits == 0 {
                break;
            }
        }
        lead.push(b'.');
        d.lead = self.pooled(&lead);
        let outer = d.body_font;
        d.body_font = FONT_MAIN_ITALIC;
        d.skip += 300;
        self.blocks(d, left, narrow, name)?;
        self.close_env(name)?;
        d.body_font = outer;
        d.skip += 300;
        Ok(())
    }

    #[cold]
    #[inline(never)]
    fn recover(&mut self, d: &mut Doc, state: (*const u8, usize, bool, u32, usize), inner: Option<&'static [u8]>) -> R<()> {
        if matches!(self.err, E_END | E_BRACE | E_ENV | E_DEPTH) {
            return Err(());
        }
        self.err = 0;
        (self.p, self.len, self.user_src, self.depth) = (state.0, state.1, state.2, state.3);
        self.pos = state.4;
        self.note(state.4);
        d.flowing = false;
        self.words.clear();
        self.items.clear();
        d.word = None;
        match inner {
            Some(name) => self.skip_environment(name),
            None => {
                self.pos += 1;
                while !self.at_end() && !self.blank_ahead() {
                    self.pos += 1;
                }
            }
        }
        Ok(())
    }

    fn quote_block(&mut self, d: &mut Doc, left: i32, narrow: i32, name: &[u8]) -> R<()> {
        d.skip += 300;
        self.blocks(d, left + QUOTE_INDENT, narrow + 2 * QUOTE_INDENT, name)?;
        self.close_env(name)?;
        d.skip += 300;
        Ok(())
    }

    fn list_block(&mut self, d: &mut Doc, left: i32, narrow: i32, name: &'static [u8]) -> R<()> {
        let bare = core::mem::take(&mut d.bare_list);
        self.enter()?;
        d.skip += 400;
        let mut count = 0;
        loop {
            self.skip_ws();
            if self.at_end() {
                return self.fail(E_END);
            }
            if self.peek() != b'\\' {
                self.blocks(d, left + LIST_INDENT, narrow + LIST_INDENT, name)?;
                continue;
            }
            let start = self.pos;
            self.pos += 1;
            match self.read_name() {
                word @ (b"item" | b"bibitem") => {
                    count += 1;
                    self.skip_ws();
                    let label = if self.peek() == b'[' { Some(self.bracketed()?) } else { None };
                    self.set_marker_number(count);
                    if word == b"bibitem" {
                        self.skip_ws();
                        let key = self.loose_argument();
                        let mut tagged = Vec::with_capacity(key.len() + 5);
                        tagged.extend_from_slice(b"cite:");
                        tagged.extend_from_slice(key);
                        self.add_label(&tagged);
                    }
                    d.label = if let Some(text) = label {
                        Some(Label::Text(text))
                    } else if name == b"thebibliography" {
                        Some(Label::Cite(count))
                    } else if matches!(name, b"enumerate" | b"enumerate*") {
                        Some(Label::Number(count))
                    } else if bare {
                        None
                    } else {
                        Some(Label::Bullet)
                    };
                    d.noindent = true;
                    self.blocks(d, left + LIST_INDENT, narrow + LIST_INDENT, name)?;
                    d.label = None;
                    d.skip += 200;
                }
                b"end" => {
                    self.pos = start;
                    self.close_env(name)?;
                    break;
                }
                _ => {
                    self.pos = start;
                    self.blocks(d, left + LIST_INDENT, narrow + LIST_INDENT, name)?;
                }
            }
        }
        d.skip += 200;
        d.noindent = !self.blank_ahead();
        self.depth -= 1;
        Ok(())
    }

    fn sized(&mut self, d: &Doc, size: u16) -> Ctx {
        let mut c = d.ctx;
        c.size = size;
        self.restyle(&c, ST_T)
    }

    fn sub_text(&mut self, d: &mut Doc, text: &'static [u8], c: &Ctx, font: u8) -> R<()> {
        let src = self.push_src(text.as_ptr(), text.len());
        let mut font = font;
        let settings = (d.graphic_paths, d.graphic_extensions, d.url_setup);
        let done = self.inline(d, c, &mut font, false);
        (d.graphic_paths, d.graphic_extensions, d.url_setup) = settings;
        self.pop_src(src);
        done
    }

    fn title_block(&mut self, d: &mut Doc) -> R<()> {
        let pt = self.doc.pt;
        let large = font_size(b"large", pt).unwrap_or(1200);
        let title = font_size(b"LARGE", pt).unwrap_or(1728);
        let rows: [(&'static [u8], u16, i32); 3] = [(d.title, title, 0), (d.author, large, 1500), (d.date, large, 1000)];
        let (left, top, width, height) = (d.left, d.top, d.text_w, d.text_h);
        d.left = d.frame_left;
        d.text_w = d.frame_w;
        let wide = d.frame_w;
        let single = d.columns == 1;
        let row_gap = if single { d.parskip } else { 0 };
        d.fresh = false;
        d.skip += TITLE_GAP - self.pt_units(10.0) + if single { self.pt_units(12.0) } else { 0 };
        for (text, size, gap) in rows {
            if text.is_empty() {
                continue;
            }
            self.block_reset();
            let c = self.sized(d, size);
            d.skip += if gap > 0 { gap + row_gap } else { 0 };
            self.sub_text(d, text, &c, 0)?;
            self.flush_word(d, false);
            self.set_lines(d, &c, 0, wide, 0, true, true);
        }
        d.skip += 1500
            + match (single, d.parskip > 0) {
                (true, true) => self.pt_units(10.0) - d.parskip * 3 / 4,
                (true, false) => self.pt_units(10.0),
                (false, false) => self.pt_units(10.5),
                (false, true) => 0,
            };
        d.noindent = true;
        d.left = left;
        d.text_w = width;
        if d.columns == 2 {
            let used = d.base + d.dep + d.skip;
            d.top = top + used;
            d.text_h = height - used;
            d.base = 0;
            d.dep = 0;
            d.skip = 0;
            d.fresh = true;
        }
        Ok(())
    }

    fn heading(&mut self, d: &mut Doc, level: usize, left: i32, width: i32) -> R<()> {
        let star = self.skip_star() || level == 3 || (level as i32 + 1) > self.counter_value(b"secnumdepth");
        let level = level.min(2);
        self.skip_ws();
        self.skip_options();
        self.skip_ws();
        if self.peek() != b'{' {
            return self.fail(E_ARG);
        }
        let (before, after) = d.heading_gaps[level];
        let pt = self.doc.pt;
        let size = font_size([b"Large" as &[u8], b"large", b"normalsize"][level], pt).unwrap_or(1000);
        let c = self.sized(d, size);
        if !star {
            self.increment_counter([b"section" as &[u8], b"subsection", b"subsubsection"][level])?;
            for (i, name) in [b"section" as &[u8], b"subsection", b"subsubsection"].iter().enumerate() {
                d.sections[i] = self.counter_value(name).max(0) as u32;
            }
            for i in 0..=level {
                if i > 0 {
                    self.put_char(d, &c, FONT_MAIN_BOLD, b'.' as u32)?;
                }
                self.put_number(d, &c, FONT_MAIN_BOLD, d.sections[i])?;
            }
            self.add_kern(d, sc(1000, c.scale));
            self.set_marker_section(level, d.sections);
        }
        self.pos += 1;
        let title = self.pos;
        let mut font = FONT_MAIN_BOLD;
        self.inline(d, &c, &mut font, true)?;
        self.flush_word(d, false);
        if !d.after_heading {
            self.add_space(d, before, HEADING_STRETCH, HEADING_GIVE);
        }
        if !d.fresh && d.base.saturating_add(d.skip).saturating_add(4 * d.baseline) > d.text_h {
            self.new_page(d)?;
        }
        self.set_lines(d, &c, left, width, 0, true, false);
        if self.store.wanted && !star {
            self.add_contents(d, level, title);
        }
        d.skip += after;
        self.give_space(d, HEADING_AFTER_STRETCH, 0);
        d.open_heading = true;
        d.noindent = true;
        Ok(())
    }

    fn paragraph(&mut self, d: &mut Doc, left: i32, width: i32) -> R<()> {
        let c = d.ctx;
        let mut font = d.body_font;
        if !d.lead.is_empty() {
            let lead = core::mem::take(&mut d.lead);
            for word in lead.split(|ch| *ch == b' ') {
                for ch in word {
                    self.put_char(d, &c, FONT_MAIN_BOLD, *ch as u32)?;
                }
                self.flush_word(d, false);
            }
        }
        let indent = if d.noindent || d.center { 0 } else { d.parindent };
        if d.parskip > 0 && !d.noindent && !d.fresh {
            d.skip += d.parskip;
        }
        d.noindent = false;
        if !d.fresh {
            self.give_space(d, PAR_STRETCH, 0);
        }
        d.flow = (left, width, indent);
        d.flowing = true;
        self.inline(d, &c, &mut font, false)?;
        d.flowing = false;
        self.flush_word(d, false);
        self.set_lines(d, &c, left, width, d.flow.2, d.center, d.center);
        d.open_par = !self.blank_behind();
        Ok(())
    }

    /// Expanding a macro places nothing, so what came before it still
    /// decides the spacing of whatever it expands to.
    fn keep_spacing(d: &mut Doc) {
        d.open_par = d.after_par;
        d.open_heading = d.after_heading;
        d.open_vspace = d.after_vspace;
    }

    fn blank_behind(&self) -> bool {
        let mut at = self.pos;
        let mut lines = 0;
        while at > 0 {
            at -= 1;
            match self.slice(at, at + 1).first() {
                Some(b'\n') => lines += 1,
                Some(b' ' | b'\t' | b'\r') => {}
                _ => break,
            }
        }
        lines >= 2
    }

    /// The \\baselineskip LaTeX pairs with each font size, in points.
    fn size_baseline(&self, size: u16, normal: i32) -> i32 {
        let pt = self.doc.pt;
        let points = size as i32 * pt;
        let rows: [(i32, f64); 12] = [
            (5000, 6.0),
            (6000, 7.0),
            (7000, 8.0),
            (8000, 9.5),
            (9000, 11.0),
            (10000, 12.0),
            (10950, 13.6),
            (12000, if pt == 12 { 14.5 } else { 14.0 }),
            (14400, 18.0),
            (17280, 22.0),
            (20740, 25.0),
            (24880, 30.0),
        ];
        let Some(&(_, skip)) = rows.iter().find(|row| (row.0 - points).abs() <= pt) else {
            return normal;
        };
        self.pt_units(skip)
    }

    fn add_kern(&mut self, d: &mut Doc, kern: i32) {
        self.break_pair();
        let mut t = d.word.take().unwrap_or_else(|| self.close(self.end(), 0, 0, 0));
        t.w += kern;
        d.word = Some(t);
    }

    fn add_box(&mut self, d: &mut Doc, b: Bx) {
        d.word = Some(self.join(d.word, b));
    }

    fn put_char(&mut self, d: &mut Doc, c: &Ctx, font: u8, cp: u32) -> R<()> {
        let ideograph = matches!(cp,0x2e80..=0x9fff|0xac00..=0xd7ff|0xf900..=0xfaff|0x20000..=0x3134f);
        if ideograph && d.word.is_some() {
            self.flush_word(d, false);
            d.glue_next = true;
        }
        let mut t = d.word.take().unwrap_or_else(|| self.close(self.end(), 0, 0, 0));
        self.text_glyph(c, font, cp, &mut t)?;
        t.e = self.end();
        d.word = Some(t);
        if ideograph {
            self.flush_word(d, false);
            d.glue_next = true;
        }
        Ok(())
    }

    fn put_number(&mut self, d: &mut Doc, c: &Ctx, font: u8, n: u32) -> R<()> {
        if n >= 10 {
            self.put_number(d, c, font, n / 10)?;
        }
        self.put_char(d, c, font, b'0' as u32 + n % 10)
    }

    /// The space factor left by a word, following TeX's \\sfcode rules.
    fn space_factor(&self, b: Bx) -> u16 {
        let mut factor = 1000;
        for it in &self.items[b.s as usize..b.e as usize] {
            if it.kind != IT_GLYPH {
                continue;
            }
            let code: u16 = match it.b {
                0 => {
                    factor = 1000;
                    continue;
                }
                0x2e | 0x3f | 0x21 => 3000,
                0x3a => 2000,
                0x3b => 1500,
                0x2c => 1250,
                0x29 | 0x27 | 0x5d | 0x22 | 0x2019 | 0x201d => continue,
                0x41..=0x5a => 999,
                _ => 1000,
            };
            factor = if code > 1000 && factor < 1000 { 1000 } else { code };
        }
        factor
    }

    fn flush_word(&mut self, d: &mut Doc, forced: bool) {
        if let Some(bx) = d.word.take() {
            let after = self.space_factor(bx);
            self.words.push(Word { bx, forced, gap: !d.glue_next, hyphen: None, penalty: 0, literal: d.literal_word, after, tied: d.tie_next, below: 0, space: (0, 0, 0, 0) });
            d.glue_next = false;
            d.tie_next = false;
            d.literal_word = false;
        } else if forced {
            if let Some(last) = self.words.last_mut() {
                last.forced = true;
            }
        }
    }

    /// Where the control word at the cursor ends, or nowhere for a control
    /// symbol, which keeps the space after it.
    fn control_word_end(&self) -> usize {
        let mut at = 1;
        if self.peek_at(at) == 0x1f {
            at += 1;
            while self.pos + at < self.len && self.peek_at(at) != 0x1f {
                at += 1;
            }
            return self.pos + at + usize::from(self.pos + at < self.len);
        }
        while self.peek_at(at).is_ascii_alphabetic() || (self.at_letter && self.peek_at(at) == b'@') {
            at += 1;
        }
        if at == 1 { usize::MAX } else { self.pos + at }
    }

    /// Sets the space after the last word in this font. A control space
    /// takes no account of the space factor.
    fn set_space(&mut self, font: u8, scale: u16, factor: Option<u16>) {
        let space = self.space_at(font, scale);
        if let Some(word) = self.words.last_mut() {
            word.space = space;
            word.after = factor.unwrap_or(word.after);
        }
    }

    fn inline(&mut self, d: &mut Doc, c: &Ctx, font: &mut u8, group: bool) -> R<()> {
        let settings = (d.graphic_paths, d.graphic_extensions, d.url_setup);
        let done = self.inline_content(d, c, font, group);
        if group { (d.graphic_paths, d.graphic_extensions, d.url_setup) = settings; }
        done
    }

    fn inline_content(&mut self, d: &mut Doc, c: &Ctx, font: &mut u8, group: bool) -> R<()> {
        let mut scoped = *c;
        let c = &mut scoped;
        loop {
            if self.at_end() {
                if group {
                    return self.fail(E_END);
                }
                return Ok(());
            }
            let ch = self.peek();
            match ch {
                b'}' => {
                    if !group {
                        if d.open_groups > 0 {
                            return Ok(());
                        }
                        return self.fail(E_BRACE);
                    }
                    self.pos += 1;
                    return Ok(());
                }
                b'{' => {
                    self.pos += 1;
                    self.enter()?;
                    let mut inner = *font;
                    self.inline(d, c, &mut inner, true)?;
                    self.depth -= 1;
                }
                b' ' | b'\t' | b'\r' | b'\n' => {
                    let breaks = !group && self.blank_ahead();
                    while matches!(self.peek(), b' ' | b'\t' | b'\r' | b'\n') {
                        self.pos += 1;
                    }
                    self.flush_word(d, false);
                    self.set_space(*font, c.scale, None);
                    d.glue_next = false;
                    if breaks {
                        return Ok(());
                    }
                }
                b'%' => {
                    while !self.at_end() && self.peek() != b'\n' {
                        self.pos += 1;
                    }
                    self.pos += 1;
                }
                b'~' => {
                    self.pos += 1;
                    if d.word.is_some() {
                        self.flush_word(d, false);
                        self.set_space(*font, c.scale, Some(1000));
                        d.tie_next = true;
                    } else {
                        self.add_kern(d, sc(SPACE, c.scale));
                    }
                }
                b'$' => {
                    if self.peek_at(1) == b'$' {
                        if group {
                            return self.fail(E_MISPLACED);
                        }
                        return Ok(());
                    }
                    self.pos += 1;
                    let b = self.list(*c, S_DOLLAR, NONE)?;
                    self.add_box(d, b);
                }
                b'-' => {
                    let run = if self.peek_at(1) != b'-' {
                        1
                    } else if self.peek_at(2) != b'-' {
                        2
                    } else {
                        3
                    };
                    self.pos += run;
                    self.put_char(d, c, *font, [0x2d, 0x2013, 0x2014][run - 1])?;
                }
                b'`' | b'\'' => {
                    let double = self.peek_at(1) == ch;
                    self.pos += if double { 2 } else { 1 };
                    let cp = match (ch, double) {
                        (b'`', false) => 0x2018,
                        (b'`', true) => 0x201c,
                        (_, false) => 0x2019,
                        _ => 0x201d,
                    };
                    self.put_char(d, c, *font, cp)?;
                }
                b'\\' => {
                    let (source, word_end) = (self.p, self.control_word_end());
                    if self.inline_command(d, c, font, group)? {
                        return Ok(());
                    }
                    if self.p == source && self.pos == word_end && !self.blank_ahead() {
                        self.skip_ws();
                    }
                }
                b'&' | b'^' | b'_' | b'#' => {
                    self.note(self.pos);
                    self.pos += 1;
                    self.put_char(d, c, *font, ch as u32)?;
                }
                _ if ch < 0x80 => {
                    self.pos += 1;
                    if ch < 32 || ch == 127 {
                        continue;
                    }
                    let mut t = d.word.take().unwrap_or_else(|| self.close(self.end(), 0, 0, 0));
                    self.text_glyph(c, *font, ch as u32, &mut t)?;
                    let mut next = self.peek();
                    while next.is_ascii_alphanumeric() {
                        self.pos += 1;
                        self.text_glyph(c, *font, next as u32, &mut t)?;
                        next = self.peek();
                    }
                    t.e = self.end();
                    d.word = Some(t);
                }
                _ => {
                    let (_, cp) = self.read_utf8()?;
                    self.put_char(d, c, *font, cp)?;
                }
            }
        }
    }

    fn styled_group(&mut self, d: &mut Doc, c: &Ctx, font: u8) -> R<()> {
        self.skip_ws();
        if self.peek() != b'{' {
            return self.fail(E_ARG);
        }
        self.pos += 1;
        self.enter()?;
        let mut inner = font;
        self.inline(d, c, &mut inner, true)?;
        self.depth -= 1;
        if inner == FONT_MAIN_ITALIC && !matches!(self.peek(), b'.' | b',') {
            self.italic_correction(d, c);
        }
        Ok(())
    }

    /// TeX's \/ after the last letter of a word.
    fn italic_correction(&mut self, d: &mut Doc, c: &Ctx) {
        let Some(word) = d.word else {
            return;
        };
        let Some(item) = self.items[..word.e as usize].last().filter(|it| it.kind == IT_GLYPH) else {
            return;
        };
        let slant = self.glyph_metric(item.a as usize).it as i32;
        self.add_kern(d, sc(slant, c.scale));
    }

    #[inline(never)]
    fn text_macro(&mut self, d: &mut Doc, c: &Ctx, font: &mut u8, def: &Def) -> R<()> {
        self.enter()?;
        let body = self.expansion(def, false)?;
        let body = self.source_copy(&body[..body.len() - 1]);
        let src = self.push_src(body.as_ptr(), body.len());
        self.inline(d, c, font, false)?;
        if !self.at_end() {
            if d.flowing {
                let (left, width, indent) = d.flow;
                self.flush_word(d, false);
                self.set_lines(d, c, left, width, indent, d.center, d.center);
                d.flow.2 = 0;
                d.flowing = false;
                self.blocks(d, left, d.text_w - width, b"")?;
                d.flowing = true;
            } else {
                self.note(self.pos);
            }
        }
        self.pop_src(src);
        self.depth -= 1;
        Ok(())
    }

    fn inline_command(&mut self, d: &mut Doc, c: &mut Ctx, font: &mut u8, group: bool) -> R<bool> {
        let tok = self.pos;
        self.pos += 1;
        let name = self.read_name();
        if self.control_command(name)? {
            return Ok(false);
        }
        if !group && (ends_paragraph(name) || (d.steps && commands::step(name).is_some())) {
            self.pos = tok;
            return Ok(true);
        }
        if !self.defs.is_empty() {
            if let Some(def) = self.find_def(name) {
                if !group && d.flowing && open_ended(def.body) {
                    self.pos = tok;
                    return Ok(true);
                }
                self.text_macro(d, c, font, &def)?;
                return Ok(false);
            }
        }
        match name {
            b"-" => {
                if d.word.is_some() {
                    self.flush_word(d, false);
                    let g = glyph_id(*font, b'-' as u32).or_else(|| glyph_id(0, b'-' as u32)).unwrap();
                    let metric = self.glyph_metric(g as usize);
                    let s = self.end();
                    self.push_glyph(g, 0, 0, c.grp);
                    let mark = Bx { s, e: self.end(), w: sc(metric.w as i32, c.scale), h: sc(metric.h as i32, c.scale), d: 0 };
                    if let Some(word) = self.words.last_mut() {
                        word.hyphen = Some(mark);
                    }
                    d.glue_next = true;
                }
                return Ok(false);
            }
            b"\\" => {
                self.skip_star();
                let gap = self.bracket_length(c)?;
                self.flush_word(d, true);
                match self.words.last_mut() {
                    Some(word) => word.below += gap,
                    None => d.skip += gap,
                }
                return Ok(false);
            }
            b"(" => {
                let b = self.display_until(c, b"\\)")?;
                self.add_box(d, b);
                return Ok(false);
            }
            b"emph" => {
                let next = if *font == FONT_MAIN_ITALIC { 0 } else { FONT_MAIN_ITALIC };
                self.styled_group(d, c, next)?;
                return Ok(false);
            }
            b"bfseries" | b"bf" => *font = FONT_MAIN_BOLD,
            b"itshape" | b"it" | b"em" => *font = FONT_MAIN_ITALIC,
            b"ttfamily" | b"tt" => *font = FONT_TYPEWRITER_REGULAR,
            b"sffamily" | b"sf" => *font = FONT_SANSSERIF_REGULAR,
            b"rmfamily" | b"rm" | b"normalfont" => *font = 0,
            b"label" => {
                let key = self.loose_argument();
                self.add_label(key);
            }
            b"today" => {}
            b"LaTeX" | b"TeX" => {
                for ch in name {
                    self.put_char(d, c, *font, *ch as u32)?;
                }
            }
            b" " | b"\n" => {
                self.flush_word(d, false);
                self.set_space(*font, c.scale, Some(1000));
                return Ok(false);
            }
            _ => {
                if let Some(size) = font_size(name, self.doc.pt) {
                    *c = self.restyle(&Ctx { size, ..*c }, ST_T);
                    return Ok(false);
                }
                let known = lookup(name);
                let ready = known.is_some_and(|sy| {
                    matches!(sy.kind, K_TEXT | K_MUKERN | K_EMKERN | K_HSPACE | K_NOOP) || (sy.kind == K_SYM && sy.flags & F_TEXT != 0)
                });
                if !ready && (self.conditional(name)? || self.text_command(d, c, font, name, tok)?) {
                    return Ok(false);
                }
                let Some(sy) = known else {
                    self.ignore_command(tok);
                    return Ok(false);
                };
                match sy.kind {
                    K_SYM if sy.flags & F_TEXT != 0 => self.put_char(d, c, *font, sy.cp)?,
                    K_TEXT => self.styled_group(d, c, sy.arg as u8)?,
                    K_MUKERN => self.add_kern(d, Self::mu(c, sy.arg as i16 as i32)),
                    K_EMKERN => self.add_kern(d, sc(sy.arg as i32, c.scale)),
                    K_HSPACE => {
                        self.skip_star();
                        let held = self.pos;
                        match self.length_arg(c) {
                            Ok(k) => self.add_kern(d, k),
                            Err(()) => {
                                self.err = 0;
                                self.pos = held;
                                self.loose_argument();
                            }
                        }
                    }
                    K_NOOP => {}
                    _ => {
                        self.note(tok);
                        self.pos = tok;
                        let b = self.arg(c)?;
                        self.add_box(d, b);
                    }
                }
            }
        }
        Ok(false)
    }

    fn display_until(&mut self, c: &Ctx, close: &[u8]) -> R<Bx> {
        let mut i = self.pos;
        loop {
            if i >= self.len {
                self.pos = self.len;
                return self.fail(E_END);
            }
            if self.slice(i, (i + close.len()).min(self.len)) == close {
                break;
            }
            i += if unsafe { *self.p.add(i) } == b'\\' { 2 } else { 1 };
        }
        let limit = self.len;
        let at = unsafe { (self.p as *mut u8).add(i) };
        let held = unsafe { *at };
        unsafe { *at = 0 };
        self.len = i;
        let b = self.list(*c, S_EOF, NONE);
        unsafe { *at = held };
        self.len = limit;
        let b = b?;
        self.pos = i + close.len();
        Ok(b)
    }

    fn after_display(&mut self, d: &mut Doc) {
        d.noindent = !self.blank_ahead();
    }

    fn place_display(&mut self, d: &mut Doc, b: Bx, left: i32, width: i32, number: Option<u32>) {
        // TeX chooses short display glue when the preceding line ends before
        // the centered formula (including two ems of clearance).
        let short = d.line_end + sc(2000, d.ctx.scale) <= left + (width - b.w) / 2;
        let (above, below) = if short {
            (
                0,
                self.pt_units(match self.doc.pt {
                    11 => 6.5,
                    12 => 7.0,
                    _ => 6.0,
                }),
            )
        } else {
            let skip = self.pt_units(self.doc.pt as f64);
            (skip, skip)
        };
        // A display at the start of a paragraph follows its empty first line.
        if d.fresh {
            self.place_line(d, 0, 0);
        }
        d.skip += above;
        if !short && !d.fresh {
            self.give_space(d, DISPLAY_STRETCH, DISPLAY_GIVE);
        }
        let h = if number.is_some() { b.h.max(sc(750, d.ctx.scale)) } else { b.h };
        let dep = if number.is_some() { b.d.max(sc(250, d.ctx.scale)) } else { b.d };
        let y = self.place_line(d, h, dep);
        self.commit(d, b.s, b.e, left + ((width - b.w) / 2).max(0), y);
        self.place_label(d, left, y);
        if let Some(n) = number {
            let c = d.ctx;
            let saved = d.word.take();
            let mark = self.end();
            let done = self
                .put_char(d, &c, 0, b'(' as u32)
                .and_then(|()| self.put_number(d, &c, 0, n))
                .and_then(|()| self.put_char(d, &c, 0, b')' as u32));
            if let (Ok(()), Some(t)) = (done, d.word.take()) {
                self.commit(d, t.s, t.e, left + width - t.w, y);
            }
            self.items.truncate(mark as usize);
            d.word = saved;
        }
        d.skip += below;
        let (stretch, give) = if short { (SHORT_DISPLAY_STRETCH, SHORT_DISPLAY_GIVE) } else { (DISPLAY_STRETCH, DISPLAY_GIVE) };
        self.give_space(d, stretch, give);
        d.line_end = 0;
    }

    fn place_line(&mut self, d: &mut Doc, h: i32, dep: i32) -> i32 {
        if !d.fresh {
            let extent = d.dep.saturating_add(h);
            let advance = if extent > d.baseline { extent.saturating_add(LINESKIP) } else { d.baseline };
            let y = d.base.saturating_add(advance).saturating_add(d.skip);
            let limit = d.text_h - d.note_h - d.bottom_h;
            let over = y.saturating_add((dep - d.topskip / 2).max(0)).saturating_sub(limit);
            if Self::line_stays(d, over) {
                d.base = y;
                d.dep = dep;
                d.skip = 0;
                d.page_over = d.page_over.max(over);
                d.placed_stretch = d.page_stretch;
                return y;
            }
            let _ = self.new_page(d);
        }
        d.line_penalties = (0, 0);
        d.fresh = false;
        d.skip = 0;
        d.base = h.max(d.topskip);
        d.dep = dep;
        d.first_line.get_or_insert(d.base);
        d.base
    }

    fn commit(&mut self, d: &Doc, s: u32, e: u32, x: i32, y: i32) {
        let dx = d.left + x;
        let dy = d.top + y;
        let Some(items) = self.items.get(s as usize..e as usize) else {
            return;
        };
        self.doc.items.extend(items.iter().map(|it| Item { x: it.x + dx, y: it.y + dy, ..*it }));
    }

    fn place_label(&mut self, d: &mut Doc, left: i32, y: i32) {
        let Some(label) = d.label.take() else {
            return;
        };
        let c = d.ctx;
        let saved = d.word.take();
        let mark = self.end();
        let made = match label {
            Label::Bullet => match lookup(b"bullet") {
                Some(s) => {
                    let a = self.sym_atom(&c, s);
                    d.word = Some(a.bx);
                    Ok(())
                }
                None => Ok(()),
            },
            Label::Number(n) => self.put_number(d, &c, 0, n).and_then(|()| self.put_char(d, &c, 0, b'.' as u32)),
            Label::Cite(n) => self
                .put_char(d, &c, 0, b'[' as u32)
                .and_then(|()| self.put_number(d, &c, 0, n))
                .and_then(|()| self.put_char(d, &c, 0, b']' as u32)),
            Label::Text(text) => self.sub_text(d, text, &c, 0),
        };
        if let (Ok(()), Some(t)) = (made, d.word.take()) {
            self.commit(d, t.s, t.e, left - t.w - LABEL_GAP, y);
        }
        self.items.truncate(mark as usize);
        d.word = saved;
    }

    /// TeX's \\addvspace, which keeps the larger of two skips unless a
    /// \\vspace stands between them, recording how far the page may shrink it.
    fn add_space(&mut self, d: &mut Doc, gap: i32, stretch: i32, give: i32) {
        let before = d.skip;
        d.skip = if d.after_vspace { d.skip + gap } else { d.skip.max(gap) };
        if d.skip > before && !d.fresh {
            self.give_space(d, stretch, give);
        }
    }

    fn give_space(&mut self, d: &mut Doc, stretch: i32, give: i32) {
        if d.text_h < float::WIDE / 2 {
            self.store.gives.push((self.doc.items.len() as u32, self.doc.art.len() as u32, give));
            d.page_shrink += give;
            d.page_stretch += stretch;
        }
    }

    /// TeX's page builder: ending the page after this line must cost no more
    /// than ending it before, where a break costs its badness and penalty.
    fn line_stays(d: &mut Doc, over: i32) -> bool {
        let (before, after) = core::mem::take(&mut d.line_penalties);
        if over > d.page_shrink {
            return false;
        }
        if over <= 0 && over.saturating_add(d.baseline) <= d.page_shrink {
            return true;
        }
        let short = (d.text_h - d.note_h - d.bottom_h).saturating_sub(d.base);
        let here = if over > 0 { page_cost(over, d.page_shrink, after) } else { page_cost(-over, d.page_stretch, after) };
        here <= page_cost(short, d.placed_stretch, before)
    }

    /// Takes a page that ran over back to its height, shrinking only the glue
    /// that allows it, in proportion, as TeX sets the page box.
    fn settle_page(&mut self, d: &mut Doc) {
        let gives = core::mem::take(&mut self.store.gives);
        d.page_stretch = 0;
        d.placed_stretch = 0;
        let (over, shrink) = (core::mem::take(&mut d.page_over) as i64, core::mem::take(&mut d.page_shrink) as i64);
        if over <= 0 || shrink <= 0 {
            return;
        }
        let mut given = 0i64;
        for (i, &(items, art, give)) in gives.iter().enumerate() {
            given += give as i64;
            let moved = (over * given / shrink) as i32;
            let next = gives.get(i + 1).map_or((self.doc.items.len() as u32, self.doc.art.len() as u32), |g| (g.0, g.1));
            // Recovering from an error can drop what a mark pointed at.
            for it in self.doc.items.get_mut(items as usize..next.0 as usize).unwrap_or_default() {
                it.y -= moved;
            }
            for a in self.doc.art.get_mut(art as usize..next.1 as usize).unwrap_or_default() {
                a.y -= moved;
            }
        }
    }

    fn new_page(&mut self, d: &mut Doc) -> R<()> {
        self.settle_page(d);
        self.finish_bottom(d);
        self.finish_notes(d);
        d.fresh = true;
        d.skip = 0;
        d.base = 0;
        d.dep = 0;
        d.float_counts = [0; 3];
        if d.columns == 2 && d.column == 0 {
            d.column = 1;
            d.left = d.frame_left + d.text_w + d.column_gap;
            self.place_deferred(d, false);
            return Ok(());
        }
        self.finish_page(d)?;
        d.page = self.increment_counter(b"page")?.max(0) as u32;
        d.column = 0;
        self.fit_frame(d);
        self.place_held(d, false);
        self.place_deferred(d, false);
        Ok(())
    }

    fn finish_page(&mut self, d: &mut Doc) -> R<()> {
        self.settle_page(d);
        d.page = self.counter_value(b"page").max(0) as u32;
        self.finish_bottom(d);
        self.finish_notes(d);
        self.page_furniture(d)?;
        self.doc.pages.push(self.doc.items.len() as u32);
        self.doc.art_pages.push(self.doc.art.len() as u32);
        Ok(())
    }

    /// Knuth and Plass: finds the breaks with the fewest demerits, keeping the
    /// best path for each fitness class so adjacent lines can be compared.
    fn all_breaks(&mut self, fit: &Fit, threshold: i64, rescue: bool) -> bool {
        let n = self.words.len();
        let words = &self.words[..n];
        let best = &mut self.store.break_best;
        best.clear();
        best.resize((n + 1) * 4, (NO_PATH, 0u32, 0u8));
        best[FIT_DECENT].0 = 0;
        for j in 1..=n {
            if j < n && (words[j].tied || words[j - 1].penalty >= 10000) {
                continue;
            }
            let penalty = if j < n { words[j - 1].penalty as i64 } else { 0 };
            let forced = j == n || words[j - 1].forced || penalty <= -10000;
            let discretionary = if j < n { words[j - 1].hyphen.map_or(0, |b| b.w) } else { 0 };
            let (mut natural, mut stretch, mut shrink) = (0i64, 0i64, 0i64);
            for i in (0..j).rev() {
                let word = &words[i];
                if i + 1 < j && word.forced {
                    break;
                }
                natural += word.bx.w as i64;
                if i + 1 < j && words[i + 1].gap {
                    let (w, more, less) = fit.glue(word);
                    natural += w as i64;
                    stretch += more as i64;
                    shrink += less as i64;
                }
                let avail = (fit.width - if i == 0 { fit.indent } else { 0 }) as i64;
                let line_width = natural + discretionary as i64;
                let excess = avail - line_width;
                let over = -excess - shrink;
                if over > 0 && i + 1 < j {
                    break;
                }
                if best[i * 4..i * 4 + 4].iter().all(|cell| cell.0 >= NO_PATH) {
                    continue;
                }
                let (bad, class) = if excess >= 0 {
                    if (forced && !fit.closed) || (fit.ragged && fit.fill == 0) {
                        (0, FIT_DECENT)
                    } else {
                        let bad = badness(excess, stretch + (fit.emergency + fit.fill) as i64);
                        (bad, if bad > 99 { FIT_VERY_LOOSE } else if bad > 12 { FIT_LOOSE } else { FIT_DECENT })
                    }
                } else {
                    let bad = if over > 0 { INF_BAD + 1 } else { badness(-excess, shrink) };
                    (bad, if bad > 12 { FIT_TIGHT } else { FIT_DECENT })
                };
                if bad > threshold && !rescue {
                    continue;
                }
                let line = if over > 0 {
                    OVERFULL + over
                } else {
                    let demerit = LINE_PENALTY + bad;
                    demerit * demerit + if discretionary > 0 { HYPHEN_PENALTY * HYPHEN_PENALTY } else { 0 }
                };
                let line = line + if penalty >= 0 { penalty * penalty } else if penalty > -10000 { -penalty * penalty } else { 0 };
                let doubled = discretionary > 0 && i > 0 && words[i - 1].hyphen.is_some();
                for from in 0..4 {
                    let previous = best[i * 4 + from].0;
                    if previous >= NO_PATH {
                        continue;
                    }
                    let clash = if from.abs_diff(class) > 1 { ADJ_DEMERITS } else { 0 };
                    let hyphens = if doubled { DOUBLE_HYPHEN_DEMERITS } else { 0 };
                    let total = previous.saturating_add(line).saturating_add(clash + hyphens);
                    if total < best[j * 4 + class].0 {
                        best[j * 4 + class] = (total, i as u32, from as u8);
                    }
                }
            }
        }
        let mut class = 0;
        for at in 1..4 {
            if best[n * 4 + at].0 < best[n * 4 + class].0 {
                class = at;
            }
        }
        if best[n * 4 + class].0 >= NO_PATH {
            return false;
        }
        self.line_costs.clear();
        self.line_costs.resize(n + 1, (0, 0));
        let mut j = n;
        while j > 0 {
            let (cost, from, previous) = best[j * 4 + class];
            self.line_costs[j] = (cost, from);
            j = from as usize;
            class = previous as usize;
        }
        true
    }

    fn set_lines(&mut self, d: &mut Doc, c: &Ctx, left: i32, width: i32, indent: i32, ragged: bool, center: bool) {
        let mut n = self.words.len();
        if n == 0 {
            return;
        }
        let mut fit = Fit {
            width,
            indent,
            ragged,
            space: self.space_at(FONT_MAIN_REGULAR, c.scale),
            emergency: 0,
            fill: sc(d.line_fill, c.scale),
            fixed: d.line_fill > 0,
            closed: d.line_fill > RAGGED_FILL,
            plain_space: sc(SPACE, c.scale),
            wide_space: sc(XSPACE, c.scale),
        };
        let tolerance = if d.sloppy { SLOPPY_TOLERANCE } else { TOLERANCE };
        if !self.all_breaks(&fit, PRETOLERANCE, false) {
            if !ragged && d.hyphenate && self.hyphenate_words() {
                n = self.words.len();
            }
            if !self.all_breaks(&fit, tolerance, false) {
                fit.emergency = if d.sloppy { sc(EMERGENCY_STRETCH, c.scale) } else { 0 };
                if fit.emergency == 0 || !self.all_breaks(&fit, tolerance, false) {
                    self.all_breaks(&fit, i64::MAX, true);
                }
            }
        }
        self.line_ends.clear();
        let mut j = n;
        while j > 0 {
            self.line_ends.push(j as u32);
            j = self.line_costs[j].1 as usize;
        }
        let body_baseline = d.baseline;
        let body_size = self.size_baseline(font_size(b"normalsize", self.doc.pt).unwrap_or(10000), d.baseline);
        if body_size == body_baseline {
            d.baseline = self.size_baseline(c.size, d.baseline);
        }
        let mut start = 0;
        let lines = self.line_ends.len();
        let (mut line, mut hyphened) = (0, false);
        for k in (0..lines).rev() {
            let end = self.line_ends[k] as usize;
            let (mut natural, mut h, mut dep) = (0, 0, 0);
            for w in &self.words[start..end] {
                natural += w.bx.w;
                h = h.max(w.bx.h);
                dep = dep.max(w.bx.d);
            }
            let (mut stretch, mut shrink) = (0, 0);
            for at in start + 1..end {
                if self.words[at].gap {
                    let (w, more, less) = fit.glue(&self.words[at - 1]);
                    natural += w;
                    stretch += more;
                    shrink += less;
                }
            }
            let hyphen = if end < n { self.words[end - 1].hyphen } else { None };
            natural += hyphen.map_or(0, |b| b.w);
            let first = if start == 0 { indent } else { 0 };
            let last = end == n || self.words[end - 1].forced;
            let excess = width - first - natural;
            let extra = if ragged || (last && excess > 0) { 0 } else { excess.max(-shrink) };
            let share = if extra > 0 { stretch } else { shrink };
            if !self.store.notes.is_empty() {
                let from = self.words[start].bx.s;
                let to = self.words[end - 1].bx.e;
                let _ = self.reserve_notes(d, from, to, h + dep);
            }
            let between = |line: usize, broken: bool| {
                (if line == 0 { CLUB_PENALTY } else { 0 }) + (if line + 2 == lines { WIDOW_PENALTY } else { 0 }) + (if broken { BROKEN_PENALTY } else { 0 })
            };
            let before = if line > 0 { between(line - 1, hyphened) } else if d.after_heading { NO_BREAK } else { 0 };
            hyphened = hyphen.is_some();
            d.line_penalties = (before, if line + 1 < lines { between(line, hyphened) } else { 0 });
            line += 1;
            let y = self.place_line(d, h, dep);
            d.skip += self.words[end - 1].below;
            let mut x = left + first;
            if center {
                x = left + (width - natural) / 2;
            }
            let (mut given, mut spent) = (0i64, 0);
            for at in start..end {
                let w = self.words[at];
                self.commit(d, w.bx.s, w.bx.e, x, y);
                x += w.bx.w;
                if at + 1 < end && self.words[at + 1].gap {
                    let (width, more, less) = fit.glue(&w);
                    x += width;
                    if share > 0 {
                        given += if extra > 0 { more } else { less } as i64;
                        let total = (extra as i64 * given / share as i64) as i32;
                        x += total - spent;
                        spent = total;
                    }
                }
            }
            if let Some(mark) = hyphen {
                self.commit(d, mark.s, mark.e, x, y);
            }
            if start == 0 {
                self.place_label(d, left, y);
            }
            d.line_end = if center { left + (width + natural) / 2 } else { left + first + natural + extra };
            start = end;
        }
        d.baseline = body_baseline;
        self.words.clear();
    }
}

impl Engine {
    fn input_file(&mut self, d: &mut Doc, left: i32, narrow: i32, include: bool) -> R<()> {
        self.skip_ws();
        let name = if self.peek() == b'{' {
            self.group_text()?
        } else {
            let start = self.pos;
            while !self.at_end() && !self.peek().is_ascii_whitespace() {
                self.pos += 1;
            }
            self.slice(start, self.pos)
        };
        let Some(index) = self.file_index(name, &[b".tex"]) else {
            self.note(self.pos);
            return Ok(());
        };
        let key = self.assets.files[index].0.clone();
        if self.assets.active.contains(&key) {
            return self.fail(E_DEPTH);
        }
        let content = self.assets.files[index].1.clone();
        let content = self.source_copy(&content);
        if include && !d.fresh {
            self.new_page(d)?;
        }
        self.assets.active.push(key);
        let src = self.push_src(content.as_ptr(), content.len());
        let done = self.blocks(d, left, narrow, b"");
        self.pop_src(src);
        self.assets.active.pop();
        done?;
        if include && !d.fresh {
            self.new_page(d)?;
        }
        Ok(())
    }
}
