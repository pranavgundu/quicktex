use super::*;

mod figure;
#[cfg(test)]
mod tests;

mod float;
mod plot;

const CENTER_GAP: i32 = 800;
const TITLE_GAP: i32 = 2800;
const SPACE: i32 = 333;
const STRETCH: i32 = 166;
const SHRINK: i32 = 111;
const LINESKIP: i32 = 100;
const TOPSKIP: i32 = 1000;
const PARINDENT: i32 = 1500;
const LIST_INDENT: i32 = 2500;
const LABEL_GAP: i32 = 500;
const QUOTE_INDENT: i32 = 2000;
const DISPLAY_SKIP: i32 = 1000;
const OVERFULL: i64 = 1 << 40;

#[derive(Clone, Copy)]
pub(super) struct Word {
    bx: Bx,
    forced: bool,
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

#[derive(Default)]
pub(super) struct Store {
    pic: figure::Pic,
    held: Vec<float::Held>,
    held_items: Vec<Item>,
    held_art: Vec<Art>,
    tail_items: Vec<Item>,
    tail_art: Vec<Art>,
    cells: Vec<(Bx, u16, u16)>,
    rows: Vec<float::TableRow>,
}

#[derive(Default)]
pub(super) struct Laid {
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
}

#[derive(Clone, Copy)]
enum Label {
    Bullet,
    Number(u32),
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
    skip: i32,
    page: u32,
    noindent: bool,
    center: bool,
    sections: [u32; 3],
    equation: u32,
    title: &'static [u8],
    author: &'static [u8],
    date: &'static [u8],
    word: Option<Bx>,
    label: Option<Label>,
    ctx: Ctx,
    columns: u8,
    column: u8,
    column_gap: i32,
    baseline: i32,
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

fn ignored_setting(name: &[u8]) -> bool {
    matches!(
        name,
        b"pagestyle" | b"thispagestyle" | b"pagenumbering" | b"label" | b"bibliographystyle" | b"usetikzlibrary" | b"pgfplotsset" | b"graphicspath"
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
        b"section"
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
        self.reset(len);
        self.words.clear();
        self.doc.items.clear();
        self.doc.pages.clear();
        self.doc.art.clear();
        self.doc.segs.clear();
        self.doc.spun.clear();
        self.doc.art_pages.clear();
        self.store.held.clear();
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
            skip: 0,
            page: 1,
            noindent: false,
            center: false,
            sections: [0; 3],
            equation: 0,
            title: b"",
            author: b"",
            date: b"",
            word: None,
            label: None,
            ctx,
            columns: 1,
            column: 0,
            column_gap: 0,
            baseline: 1200,
            frame_left: 0,
            frame_top: 0,
            frame_w: 0,
            frame_h: 0,
            parskip: 0,
            parindent: PARINDENT,
            table_gap: 0,
            float_gap: 0,
            caption_gap: 0,
            heading_gaps: [(1500, 900), (1200, 600), (1000, 400)],
            figures: 0,
            tables: 0,
            float: 0,
        };
        self.set_paper(&mut d, b"");
        let done = self.blocks(&mut d, 0, 0, b"");
        match done.and_then(|()| {
            self.finish_page(&mut d)?;
            while !self.store.held.is_empty() {
                d.page += 1;
                d.column = 0;
                self.fit_frame(&mut d);
                self.place_held(&mut d);
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
        let (page_w, page_h) = if a4 { (595, 842) } else { (612, 792) };
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
        self.doc.w = unit(page_w);
        self.doc.h = unit(page_h);
        d.columns = if two { 2 } else { 1 };
        d.frame_w = unit(if two { 470 } else { text_w });
        d.frame_left = (self.doc.w - d.frame_w) / 2;
        d.frame_h = unit(frame_h);
        d.frame_top = unit(frame_top);
        d.column_gap = unit(10);
        d.baseline = match pt {
            11 => 1236,
            12 => 1208,
            _ => 1200,
        };
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
            let Some(v) = figure::length_pt(value) else {
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
        let options = if self.peek() == b'[' { self.bracketed()? } else { b"" };
        let names = self.group_text()?;
        for part in names.split(|b| *b == b',') {
            let name = part.trim_ascii();
            match name {
                b"" | b"tikz" | b"pgfplots" => {}
                b"geometry" => self.set_margins(d, options),
                b"parskip" => {
                    d.parskip = 618;
                    d.parindent = 0;
                }
                _ if crate::packages::is_noop(name) => {}
                _ => match crate::packages::find_package(name) {
                    Some(i) => self.load_package(i)?,
                    None => return self.fail_at(name, E_PACKAGE),
                },
            }
        }
        Ok(())
    }

    fn set_length(&mut self, d: &mut Doc) -> R<()> {
        let name = self.group_text()?;
        let value = self.group_text()?;
        let Some(v) = figure::length_pt(value) else {
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
            b"\\intextsep" => d.float_gap = v,
            _ => {}
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
            if self.at_end() {
                if env.is_empty() {
                    self.depth -= 1;
                    return Ok(());
                }
                return self.fail(E_END);
            }
            let width = d.text_w - narrow;
            if self.peek() == b'$' && self.peek_at(1) == b'$' {
                self.pos += 2;
                let c = self.restyle(&d.ctx.clone(), ST_D);
                let b = self.display_until(&c, b"$$")?;
                self.place_display(d, b, left, width, None);
                self.after_display(d);
                continue;
            }
            if self.peek() != b'\\' {
                self.paragraph(d, left, width)?;
                continue;
            }
            let start = self.pos;
            self.pos += 1;
            let name = self.read_name();
            match name {
                b"documentclass" => {
                    self.skip_ws();
                    let options = if self.peek() == b'[' { self.bracketed()? } else { b"" };
                    self.group_text()?;
                    self.set_paper(d, options);
                }
                b"title" => d.title = self.group_text()?,
                b"author" => d.author = self.group_text()?,
                b"date" => d.date = self.group_text()?,
                b"maketitle" => self.title_block(d)?,
                b"section" => self.heading(d, 0, left, width)?,
                b"subsection" => self.heading(d, 1, left, width)?,
                b"subsubsection" => self.heading(d, 2, left, width)?,
                b"newpage" | b"clearpage" => {
                    if !d.fresh {
                        self.new_page(d)?;
                    }
                }
                b"par" | b"raggedbottom" | b"flushbottom" | b"sloppy" | b"raggedright" | b"protect" => {}
                b"centering" => d.center = true,
                b"caption" => self.caption(d, left, width)?,
                b"usepackage" | b"RequirePackage" => self.package_line(d)?,
                b"setlength" => self.set_length(d)?,
                b"setcounter" => {
                    self.skip_argument()?;
                    self.skip_argument()?;
                }
                b"geometry" => {
                    let options = self.group_text()?;
                    self.set_margins(d, options);
                }
                b"titlespacing" => self.title_spacing(d)?,
                b"captionsetup" => {
                    let options = self.group_text()?;
                    let mut at = 0;
                    while let Some((key, value)) = figure::next_option(options, &mut at) {
                        if key == b"skip" {
                            if let Some(v) = figure::length_pt(value) {
                                d.caption_gap = self.pt_units(v);
                            }
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
                }
                b"[" => {
                    let c = self.restyle(&d.ctx.clone(), ST_D);
                    let b = self.display_until(&c, b"\\]")?;
                    self.place_display(d, b, left, width, None);
                    self.after_display(d);
                }
                b"item" => {
                    if env != b"itemize" && env != b"enumerate" {
                        return self.fail(E_MISPLACED);
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
                b"begin" => self.begin_block(d, left, narrow, start)?,
                _ if is_definition(name) => {
                    let mut c = d.ctx;
                    let mut r = self.new_run(0, NONE, c);
                    self.core(name, &mut r, &mut c)?;
                }
                _ if ignored_setting(name) => self.skip_argument()?,
                b"tableofcontents" | b"today" => {}
                _ => {
                    if let Some(size) = font_size(name, self.doc.pt) {
                        d.ctx = self.sized(d, size);
                        continue;
                    }
                    self.pos = start;
                    self.paragraph(d, left, width)?;
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
        let (s, e) = self.raw_group()?;
        let name = self.slice(s, e);
        let width = d.text_w - narrow;
        match name {
            b"document" => {}
            b"itemize" | b"enumerate" => self.list_block(d, left, narrow, name)?,
            b"center" => {
                let outer = d.center;
                d.center = true;
                d.skip = d.skip.max(CENTER_GAP);
                self.blocks(d, left, narrow, name)?;
                self.close_env(name)?;
                d.center = outer;
                d.skip = d.skip.max(CENTER_GAP);
            }
            b"quote" | b"quotation" => self.quote_block(d, left, narrow, name)?,
            b"figure" | b"figure*" | b"table" | b"table*" => self.float_block(d, left, narrow, name)?,
            b"tabular" => self.table_block(d, left, width)?,
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
                let b = self.environment(&c)?.bx;
                let number = if name == b"equation" {
                    d.equation += 1;
                    Some(d.equation)
                } else {
                    None
                };
                self.place_display(d, b, left, width, number);
                self.after_display(d);
            }
            _ => {
                self.pos = s;
                return self.fail(E_ENV);
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
        self.enter()?;
        d.skip += 400;
        let mut count = 0;
        loop {
            self.skip_ws();
            if self.at_end() {
                return self.fail(E_END);
            }
            if self.peek() != b'\\' {
                return self.fail(E_MISPLACED);
            }
            let start = self.pos;
            self.pos += 1;
            match self.read_name() {
                b"item" => {
                    count += 1;
                    self.skip_ws();
                    d.label = Some(if self.peek() == b'[' {
                        Label::Text(self.bracketed()?)
                    } else if name == b"enumerate" {
                        Label::Number(count)
                    } else {
                        Label::Bullet
                    });
                    d.noindent = true;
                    self.blocks(d, left + LIST_INDENT, narrow + LIST_INDENT, name)?;
                    d.skip += 200;
                }
                b"end" => {
                    self.pos = start;
                    self.close_env(name)?;
                    break;
                }
                _ => {
                    self.pos = start;
                    return self.fail(E_MISPLACED);
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
        self.inline(d, c, &mut font, false)?;
        self.pop_src(src);
        Ok(())
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
        d.skip += TITLE_GAP + if single { self.pt_units(10.0) } else { 0 };
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
                (true, true) => -d.parskip * 3 / 4,
                (false, false) => self.pt_units(10.5),
                _ => 0,
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
        let star = self.skip_star();
        self.skip_ws();
        if self.peek() != b'{' {
            return self.fail(E_ARG);
        }
        let (before, after) = d.heading_gaps[level];
        let pt = self.doc.pt;
        let size = font_size([b"Large" as &[u8], b"large", b"normalsize"][level], pt).unwrap_or(1000);
        let c = self.sized(d, size);
        if !star {
            d.sections[level] += 1;
            d.sections[level + 1..].fill(0);
            for i in 0..=level {
                if i > 0 {
                    self.put_char(d, &c, FONT_MAIN_BOLD, b'.' as u32)?;
                }
                self.put_number(d, &c, FONT_MAIN_BOLD, d.sections[i])?;
            }
            self.add_kern(d, sc(1000, c.scale));
        }
        self.pos += 1;
        let mut font = FONT_MAIN_BOLD;
        self.inline(d, &c, &mut font, true)?;
        self.flush_word(d, false);
        d.skip += before;
        if !d.fresh && d.base.saturating_add(d.skip).saturating_add(4 * d.baseline) > d.text_h {
            self.new_page(d)?;
        }
        self.set_lines(d, &c, left, width, 0, true, false);
        d.skip += after;
        d.noindent = true;
        Ok(())
    }

    fn paragraph(&mut self, d: &mut Doc, left: i32, width: i32) -> R<()> {
        let c = d.ctx;
        let mut font = 0;
        self.inline(d, &c, &mut font, false)?;
        self.flush_word(d, false);
        let indent = if d.noindent || d.center { 0 } else { d.parindent };
        if d.parskip > 0 && !d.noindent && !d.fresh {
            d.skip += d.parskip;
        }
        d.noindent = false;
        self.set_lines(d, &c, left, width, indent, d.center, d.center);
        Ok(())
    }

    fn add_kern(&mut self, d: &mut Doc, kern: i32) {
        let mut t = d.word.take().unwrap_or_else(|| self.close(self.end(), 0, 0, 0));
        t.w += kern;
        d.word = Some(t);
    }

    fn add_box(&mut self, d: &mut Doc, b: Bx) {
        d.word = Some(self.join(d.word, b));
    }

    fn put_char(&mut self, d: &mut Doc, c: &Ctx, font: u8, cp: u32) -> R<()> {
        let mut t = d.word.take().unwrap_or_else(|| self.close(self.end(), 0, 0, 0));
        self.text_glyph(c, font, cp, &mut t)?;
        t.e = self.end();
        d.word = Some(t);
        Ok(())
    }

    fn put_number(&mut self, d: &mut Doc, c: &Ctx, font: u8, n: u32) -> R<()> {
        if n >= 10 {
            self.put_number(d, c, font, n / 10)?;
        }
        self.put_char(d, c, font, b'0' as u32 + n % 10)
    }

    fn flush_word(&mut self, d: &mut Doc, forced: bool) {
        if let Some(bx) = d.word.take() {
            self.words.push(Word { bx, forced });
        } else if forced {
            if let Some(last) = self.words.last_mut() {
                last.forced = true;
            }
        }
    }

    fn inline(&mut self, d: &mut Doc, c: &Ctx, font: &mut u8, group: bool) -> R<()> {
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
                    self.add_kern(d, sc(SPACE, c.scale));
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
                    if self.inline_command(d, c, font, group)? {
                        return Ok(());
                    }
                }
                b'&' | b'^' | b'_' | b'#' => return self.fail(E_MISPLACED),
                _ if ch < 0x80 => {
                    if ch < 32 || ch == 127 {
                        return self.fail(E_CHAR);
                    }
                    self.pos += 1;
                    self.put_char(d, c, *font, ch as u32)?;
                }
                _ => {
                    let tok = self.pos;
                    let (_, cp) = self.read_utf8()?;
                    if glyph_id(*font, cp).or_else(|| glyph_id(FONT_MAIN_REGULAR, cp)).is_none() {
                        self.pos = tok;
                        return self.fail(E_CHAR);
                    }
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
        Ok(())
    }

    fn inline_command(&mut self, d: &mut Doc, c: &mut Ctx, font: &mut u8, group: bool) -> R<bool> {
        let tok = self.pos;
        self.pos += 1;
        let name = self.read_name();
        if !group && ends_paragraph(name) {
            self.pos = tok;
            return Ok(true);
        }
        if !self.defs.is_empty() {
            if let Some(def) = self.find_def(name) {
                let b = self.text_def(&def, c, *font)?;
                self.add_box(d, b);
                return Ok(false);
            }
        }
        match name {
            b"\\" => {
                self.skip_star();
                self.bracket_length(c)?;
                self.flush_word(d, true);
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
            b"label" => self.skip_argument()?,
            b"today" | b"centering" | b"protect" => {}
            b"LaTeX" | b"TeX" => {
                for ch in name {
                    self.put_char(d, c, *font, *ch as u32)?;
                }
            }
            _ => {
                if let Some(size) = font_size(name, self.doc.pt) {
                    *c = self.restyle(&Ctx { size, ..*c }, ST_T);
                    return Ok(false);
                }
                let Some(sy) = lookup(name) else {
                    self.pos = tok;
                    return self.fail(E_CMD);
                };
                match sy.kind {
                    K_SYM if sy.flags & F_TEXT != 0 => self.put_char(d, c, *font, sy.cp)?,
                    K_TEXT => self.styled_group(d, c, sy.arg as u8)?,
                    K_MUKERN => self.add_kern(d, Self::mu(c, sy.arg as i16 as i32)),
                    K_EMKERN => self.add_kern(d, sc(sy.arg as i32, c.scale)),
                    K_HSPACE => {
                        let k = self.length_arg(c)?;
                        self.add_kern(d, k);
                    }
                    K_NOOP => {}
                    _ => {
                        self.pos = tok;
                        return self.fail(E_CMD);
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
        d.skip += DISPLAY_SKIP;
        let y = self.place_line(d, b.h, b.d);
        self.commit(d, b.s, b.e, left + ((width - b.w) / 2).max(0), y);
        self.place_label(d, left, y);
        if let Some(n) = number {
            let c = d.ctx;
            let saved = d.word.take();
            let mark = self.end();
            let done = self.put_char(d, &c, 0, b'(' as u32).and_then(|()| self.put_number(d, &c, 0, n)).and_then(|()| self.put_char(d, &c, 0, b')' as u32));
            if let (Ok(()), Some(t)) = (done, d.word.take()) {
                self.commit(d, t.s, t.e, left + width - t.w, y);
            }
            self.items.truncate(mark as usize);
            d.word = saved;
        }
        d.skip += DISPLAY_SKIP;
    }

    fn place_line(&mut self, d: &mut Doc, h: i32, dep: i32) -> i32 {
        if !d.fresh {
            let y = d.base.saturating_add(d.baseline.max(d.dep.saturating_add(h).saturating_add(LINESKIP))).saturating_add(d.skip);
            if y.saturating_add(dep) <= d.text_h {
                d.base = y;
                d.dep = dep;
                d.skip = 0;
                return y;
            }
            let _ = self.new_page(d);
        }
        d.fresh = false;
        d.skip = 0;
        d.base = h.max(TOPSKIP);
        d.dep = dep;
        d.base
    }

    fn commit(&mut self, d: &Doc, s: u32, e: u32, x: i32, y: i32) {
        let dx = d.left + x;
        let dy = d.top + y;
        for i in s as usize..e as usize {
            let mut it = self.items[i];
            it.x += dx;
            it.y += dy;
            self.doc.items.push(it);
        }
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
            Label::Text(text) => self.sub_text(d, text, &c, 0),
        };
        if let (Ok(()), Some(t)) = (made, d.word.take()) {
            self.commit(d, t.s, t.e, left - t.w - LABEL_GAP, y);
        }
        self.items.truncate(mark as usize);
        d.word = saved;
    }

    fn new_page(&mut self, d: &mut Doc) -> R<()> {
        d.fresh = true;
        d.skip = 0;
        d.base = 0;
        d.dep = 0;
        if d.columns == 2 && d.column == 0 {
            d.column = 1;
            d.left = d.frame_left + d.text_w + d.column_gap;
            return Ok(());
        }
        self.finish_page(d)?;
        d.page += 1;
        d.column = 0;
        self.fit_frame(d);
        self.place_held(d);
        Ok(())
    }

    fn finish_page(&mut self, d: &mut Doc) -> R<()> {
        let c = d.ctx;
        let saved = d.word.take();
        let mark = self.end();
        let (left, top) = (d.left, d.top);
        d.left = d.frame_left;
        d.top = d.frame_top;
        self.put_number(d, &c, 0, d.page)?;
        if let Some(t) = d.word.take() {
            self.commit(d, t.s, t.e, (d.frame_w - t.w) / 2, d.frame_h + 3000);
        }
        d.left = left;
        d.top = top;
        self.items.truncate(mark as usize);
        d.word = saved;
        self.doc.pages.push(self.doc.items.len() as u32);
        self.doc.art_pages.push(self.doc.art.len() as u32);
        Ok(())
    }

    fn set_lines(&mut self, d: &mut Doc, c: &Ctx, left: i32, width: i32, indent: i32, ragged: bool, center: bool) {
        let n = self.words.len();
        if n == 0 {
            return;
        }
        let space = sc(SPACE, c.scale);
        let stretch = sc(STRETCH, c.scale);
        let shrink = sc(SHRINK, c.scale);
        self.line_costs.clear();
        self.line_costs.push((0, 0));
        for j in 1..=n {
            let mut best = (i64::MAX, 0u32);
            let mut natural = 0;
            let mut gaps = 0;
            let last = j == n || self.words[j - 1].forced;
            for i in (0..j).rev() {
                if i + 1 < j && self.words[i].forced {
                    break;
                }
                natural += self.words[i].bx.w;
                if i + 1 < j {
                    natural += space;
                    gaps += 1;
                }
                let avail = width - if i == 0 { indent } else { 0 };
                let over = natural - gaps * shrink - avail;
                if over > 0 && i + 1 < j {
                    break;
                }
                let slack = (avail - natural) as i64;
                let cost = if over > 0 {
                    OVERFULL + over as i64
                } else if last {
                    0
                } else if ragged {
                    slack * slack / 1000
                } else if gaps == 0 {
                    1_000_000
                } else {
                    let room = gaps as i64 * if slack >= 0 { stretch as i64 } else { shrink as i64 };
                    let ratio = slack.abs() * 1000 / room.max(1);
                    let badness = (ratio * ratio / 1000 * ratio / 10_000_000).min(10_000);
                    (10 + badness) * (10 + badness)
                };
                let total = self.line_costs[i].0.saturating_add(cost);
                if total < best.0 {
                    best = (total, i as u32);
                }
            }
            self.line_costs.push(best);
        }
        self.line_ends.clear();
        let mut j = n;
        while j > 0 {
            self.line_ends.push(j as u32);
            j = self.line_costs[j].1 as usize;
        }
        let mut start = 0;
        for k in (0..self.line_ends.len()).rev() {
            let end = self.line_ends[k] as usize;
            let (mut natural, mut h, mut dep) = (0, 0, 0);
            for w in &self.words[start..end] {
                natural += w.bx.w;
                h = h.max(w.bx.h);
                dep = dep.max(w.bx.d);
            }
            let gaps = (end - start - 1) as i32;
            natural += gaps * space;
            let first = if start == 0 { indent } else { 0 };
            let last = end == n || self.words[end - 1].forced;
            let extra = if ragged || last || gaps == 0 { 0 } else { (width - first - natural).max(-gaps * shrink) };
            let y = self.place_line(d, h, dep);
            let mut x = left + first;
            if center {
                x = left + (width - natural) / 2;
            }
            for (i, at) in (start..end).enumerate() {
                let w = self.words[at];
                self.commit(d, w.bx.s, w.bx.e, x, y);
                x += w.bx.w + space;
                if gaps > 0 {
                    let i = i as i32;
                    x += extra * (i + 1) / gaps - extra * i / gaps;
                }
            }
            if start == 0 {
                self.place_label(d, left, y);
            }
            start = end;
        }
        self.words.clear();
    }
}
