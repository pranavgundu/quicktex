use super::*;

const WIDE: i32 = i32::MAX / 4;
const HEAVY_RULE: i32 = 80;
const LIGHT_RULE: i32 = 50;
const ABOVE_RULE: i32 = 170;
const BELOW_RULE: i32 = 280;
const PLAIN_RULE: i32 = 40;
const MAX_COLUMNS: usize = 32;
const WIDE_FLOAT_GAP: f64 = 20.0;

#[derive(Clone, Copy)]
pub(in crate::tex) struct Held {
    items: (u32, u32),
    art: (u32, u32),
    h: i32,
}

#[derive(Clone, Copy, Default)]
pub(in crate::tex) struct TableRow {
    h: i32,
    d: i32,
    rule: u8,
}

const RULE_NONE: u8 = 0;
const RULE_HEAVY: u8 = 1;
const RULE_LIGHT: u8 = 2;
const RULE_PLAIN: u8 = 3;

fn cell_end(engine: &Engine) -> usize {
    let mut i = engine.pos;
    let mut depth = 0;
    let mut math = false;
    while i < engine.len {
        match unsafe { *engine.p.add(i) } {
            b'\\' => {
                let next = if i + 1 < engine.len { unsafe { *engine.p.add(i + 1) } } else { 0 };
                if depth == 0 && !math && (next == b'\\' || engine.slice(i, (i + 4).min(engine.len)) == b"\\end") {
                    return i;
                }
                i += 2;
                continue;
            }
            b'{' => depth += 1,
            b'}' => depth -= 1,
            b'$' => math = !math,
            b'&' if depth == 0 && !math => return i,
            b'%' => {
                while i < engine.len && unsafe { *engine.p.add(i) } != b'\n' {
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    i
}

impl Engine {
    pub(super) fn line_box(&mut self, d: &mut Doc, c: &Ctx) -> Bx {
        self.flush_word(d, false);
        let space = sc(SPACE, c.scale);
        let mut out = self.close(self.end(), 0, 0, 0);
        let mut first = true;
        for i in 0..self.words.len() {
            let b = self.words[i].bx;
            if first {
                out.s = b.s;
                first = false;
            } else {
                out.w += space;
            }
            self.shift(b.s, b.e, out.w, 0);
            out.w += b.w;
            out.h = out.h.max(b.h);
            out.d = out.d.max(b.d);
            out.e = b.e;
        }
        self.words.clear();
        out
    }

    fn line_until(&mut self, d: &mut Doc, c: &Ctx, stop: usize) -> R<Bx> {
        let limit = self.len;
        let at = unsafe { (self.p as *mut u8).add(stop) };
        let held = unsafe { *at };
        unsafe { *at = 0 };
        self.len = stop;
        let mut font = 0;
        let done = self.inline(d, c, &mut font, false);
        let reached = self.pos >= stop;
        unsafe { *at = held };
        self.len = limit;
        done?;
        if !reached {
            return self.fail(E_MISPLACED);
        }
        self.pos = stop;
        Ok(self.line_box(d, c))
    }

    fn table_rule(&mut self) -> u8 {
        let start = self.pos;
        if self.peek() == b'\\' {
            self.pos += 1;
            let rule = match self.read_name() {
                b"toprule" | b"bottomrule" => RULE_HEAVY,
                b"midrule" => RULE_LIGHT,
                b"hline" => RULE_PLAIN,
                _ => RULE_NONE,
            };
            if rule != RULE_NONE {
                return rule;
            }
            self.pos = start;
        }
        RULE_NONE
    }

    fn table_spec(&mut self) -> R<([u8; MAX_COLUMNS], usize, u64)> {
        let spec = self.group_text()?;
        let mut aligns = [0u8; MAX_COLUMNS];
        let mut count = 0;
        let mut lines = 0u64;
        let mut i = 0;
        while i < spec.len() {
            match spec[i] {
                b'l' | b'c' | b'r' => {
                    if count == MAX_COLUMNS {
                        return self.fail(E_ENV);
                    }
                    aligns[count] = spec[i];
                    count += 1;
                }
                b'|' => lines |= 1 << count,
                b' ' => {}
                _ => return self.fail(E_ENV),
            }
            i += 1;
        }
        if count == 0 {
            return self.fail(E_ENV);
        }
        Ok((aligns, count, lines))
    }

    pub(super) fn table_block(&mut self, d: &mut Doc, left: i32, width: i32) -> R<()> {
        self.skip_ws();
        if self.peek() == b'[' {
            self.bracketed()?;
        }
        let (aligns, count, lines) = self.table_spec()?;
        let c = d.ctx;
        let mut cells = core::mem::take(&mut self.store.cells);
        let mut rows = core::mem::take(&mut self.store.rows);
        cells.clear();
        rows.clear();
        let done = self.table_rows(d, &c, count, &mut cells, &mut rows);
        let placed = done.and_then(|()| self.table_place(d, &c, left, width, &aligns, count, lines, &cells, &rows));
        self.store.cells = cells;
        self.store.rows = rows;
        placed
    }

    fn table_rows(&mut self, d: &mut Doc, c: &Ctx, count: usize, cells: &mut Vec<(Bx, u16, u16)>, rows: &mut Vec<TableRow>) -> R<()> {
        let mut rule = RULE_NONE;
        loop {
            self.skip_ws();
            let found = self.table_rule();
            if found != RULE_NONE {
                rule = found;
                continue;
            }
            if self.at_end() {
                return self.fail(E_END);
            }
            if self.slice(self.pos, (self.pos + 4).min(self.len)) == b"\\end" {
                self.close_env(b"tabular")?;
                rows.push(TableRow { h: 0, d: 0, rule });
                return Ok(());
            }
            let row = rows.len() as u16;
            let mut column = 0;
            let (mut h, mut dep) = (sc(840, c.scale), sc(360, c.scale));
            loop {
                let stop = cell_end(self);
                if stop >= self.len {
                    self.pos = self.len;
                    return self.fail(E_END);
                }
                let b = self.line_until(d, c, stop)?;
                if column >= count {
                    return self.fail(E_MISPLACED);
                }
                h = h.max(b.h);
                dep = dep.max(b.d);
                cells.push((b, row, column as u16));
                column += 1;
                if self.peek() == b'&' {
                    self.pos += 1;
                    continue;
                }
                break;
            }
            if self.peek() == b'\\' && self.peek_at(1) == b'\\' {
                self.pos += 2;
                self.bracket_length(c)?;
            }
            rows.push(TableRow { h, d: dep, rule });
            rule = RULE_NONE;
        }
    }

    fn table_place(
        &mut self,
        d: &mut Doc,
        c: &Ctx,
        left: i32,
        width: i32,
        aligns: &[u8; MAX_COLUMNS],
        count: usize,
        lines: u64,
        cells: &[(Bx, u16, u16)],
        rows: &[TableRow],
    ) -> R<()> {
        let mut widths = [0i32; MAX_COLUMNS];
        for (b, _, column) in cells {
            let w = &mut widths[*column as usize];
            *w = (*w).max(b.w);
        }
        let gap = d.table_gap;
        let mut starts = [0i32; MAX_COLUMNS + 1];
        for i in 0..count {
            starts[i + 1] = starts[i] + widths[i] + 2 * gap;
        }
        let total_w = starts[count];
        let rule_of = |kind: u8| match kind {
            RULE_HEAVY => (sc(HEAVY_RULE, c.scale), sc(ABOVE_RULE, c.scale), sc(BELOW_RULE, c.scale)),
            RULE_LIGHT => (sc(LIGHT_RULE, c.scale), sc(ABOVE_RULE, c.scale), sc(BELOW_RULE, c.scale)),
            RULE_PLAIN => (PLAIN_RULE, 0, 0),
            _ => (0, 0, 0),
        };
        let mut y = 0;
        let mut at = 0;
        let first_rule = self.end();
        for (index, row) in rows.iter().enumerate() {
            if row.rule != RULE_NONE {
                let (thick, above, below) = rule_of(row.rule);
                y += if index == 0 { 0 } else { above };
                self.push_rule(c, 0, y, total_w, thick);
                y += thick + below;
            }
            let base = y + row.h;
            while at < cells.len() && cells[at].1 as usize == index {
                let (b, _, column) = cells[at];
                let i = column as usize;
                let room = widths[i] - b.w;
                let x = starts[i]
                    + gap
                    + match aligns[i] {
                        b'c' => room / 2,
                        b'r' => room,
                        _ => 0,
                    };
                self.shift(b.s, b.e, x, base);
                at += 1;
            }
            y = base + row.d;
        }
        for i in 0..=count {
            if lines & (1 << i) != 0 {
                self.push_rule(c, starts[i], 0, PLAIN_RULE, y);
            }
        }
        let s = cells.first().map_or(first_rule, |cell| cell.0.s.min(first_rule));
        let e = self.end();
        self.shift(s, e, 0, -y);
        let b = Bx { s, e, w: total_w, h: y, d: 0 };
        let line = self.place_line(d, b.h, 0);
        let x = left + if d.center { ((width - b.w) / 2).max(0) } else { 0 };
        self.commit(d, b.s, b.e, x, line);
        Ok(())
    }

    pub(super) fn caption(&mut self, d: &mut Doc, left: i32, width: i32) -> R<()> {
        if d.float == 0 {
            return self.fail(E_MISPLACED);
        }
        self.skip_ws();
        if self.peek() == b'[' {
            self.bracketed()?;
            self.skip_ws();
        }
        if self.peek() != b'{' {
            return self.fail(E_ARG);
        }
        let c = self.sized(d, UNIT);
        let (word, number): (&[u8], u32) = if d.float == 1 {
            d.figures += 1;
            (b"Figure", d.figures)
        } else {
            d.tables += 1;
            (b"Table", d.tables)
        };
        for ch in word {
            self.put_char(d, &c, 0, *ch as u32)?;
        }
        self.flush_word(d, false);
        self.put_number(d, &c, 0, number)?;
        self.put_char(d, &c, 0, b':' as u32)?;
        self.flush_word(d, false);
        self.pos += 1;
        let mut font = 0;
        self.inline(d, &c, &mut font, true)?;
        self.flush_word(d, false);
        let space = sc(SPACE, c.scale);
        let natural = self.words.iter().map(|w| w.bx.w + space).sum::<i32>() - space;
        let short = natural <= width;
        d.skip += d.caption_gap;
        self.set_lines(d, &c, left, width, 0, short, short);
        Ok(())
    }

    fn stash(&mut self, items: usize, art: usize, h: i32) -> Held {
        let i0 = self.store.held_items.len() as u32;
        let a0 = self.store.held_art.len() as u32;
        self.store.held_items.extend_from_slice(&self.doc.items[items..]);
        self.store.held_art.extend_from_slice(&self.doc.art[art..]);
        self.doc.items.truncate(items);
        self.doc.art.truncate(art);
        Held { items: (i0, self.store.held_items.len() as u32), art: (a0, self.store.held_art.len() as u32), h }
    }

    fn release(&mut self, held: Held, x: i32, y: i32) {
        for i in held.items.0..held.items.1 {
            let mut it = self.store.held_items[i as usize];
            it.x += x;
            it.y += y;
            self.doc.items.push(it);
        }
        for i in held.art.0..held.art.1 {
            let mut a = self.store.held_art[i as usize];
            a.x += x;
            a.y += y;
            self.doc.art.push(a);
        }
    }

    pub(super) fn place_held(&mut self, d: &mut Doc) {
        let mut used = 0;
        for i in 0..self.store.held.len() {
            let held = self.store.held[i];
            self.release(held, d.frame_left, d.frame_top + used);
            used += held.h + self.pt_units(WIDE_FLOAT_GAP);
        }
        self.store.held.clear();
        self.store.held_items.clear();
        self.store.held_art.clear();
        d.top = d.frame_top + used;
        d.text_h = d.frame_h - used;
    }

    pub(super) fn float_block(&mut self, d: &mut Doc, left: i32, narrow: i32, name: &'static [u8]) -> R<()> {
        self.skip_ws();
        if self.peek() == b'[' {
            self.bracketed()?;
        }
        let wide = name.ends_with(b"*") && d.columns == 2;
        let width = if wide { d.frame_w } else { d.text_w - narrow };
        let items = self.doc.items.len();
        let art = self.doc.art.len();
        let mut f = d.clone();
        f.left = 0;
        f.top = 0;
        f.text_w = width;
        f.text_h = WIDE;
        f.base = 0;
        f.dep = 0;
        f.skip = 0;
        f.fresh = true;
        f.noindent = true;
        f.center = false;
        f.columns = 1;
        f.column = 0;
        f.word = None;
        f.label = None;
        f.float = if name.starts_with(b"figure") { 1 } else { 2 };
        self.blocks(&mut f, 0, 0, name)?;
        self.close_env(name)?;
        d.figures = f.figures;
        d.tables = f.tables;
        d.equation = f.equation;
        let h = f.base + f.dep;
        if wide {
            let held = self.stash(items, art, h);
            self.store.held.push(held);
            return Ok(());
        }
        let gap = d.float_gap;
        let (mut items, mut art) = (items, art);
        let mut y = if d.fresh { 0 } else { d.base + d.dep + d.skip + gap };
        if !d.fresh && y + h > d.text_h {
            let mut tail_items = core::mem::take(&mut self.store.tail_items);
            let mut tail_art = core::mem::take(&mut self.store.tail_art);
            tail_items.clear();
            tail_art.clear();
            tail_items.extend_from_slice(&self.doc.items[items..]);
            tail_art.extend_from_slice(&self.doc.art[art..]);
            self.doc.items.truncate(items);
            self.doc.art.truncate(art);
            let broke = self.new_page(d);
            items = self.doc.items.len();
            art = self.doc.art.len();
            self.doc.items.extend_from_slice(&tail_items);
            self.doc.art.extend_from_slice(&tail_art);
            self.store.tail_items = tail_items;
            self.store.tail_art = tail_art;
            broke?;
            y = 0;
        }
        let (dx, dy) = (d.left + left, d.top + y);
        for it in &mut self.doc.items[items..] {
            it.x += dx;
            it.y += dy;
        }
        for a in &mut self.doc.art[art..] {
            a.x += dx;
            a.y += dy;
        }
        d.base = y + h;
        d.dep = 0;
        d.skip = gap;
        d.fresh = false;
        d.noindent = false;
        Ok(())
    }
}
