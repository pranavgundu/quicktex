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

fn natural_start(widths: &[i32; MAX_COLUMNS], gap: i32, column: usize) -> i32 {
    widths[..column].iter().sum::<i32>() + 2 * gap * column as i32
}

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
        loop {
            let start = self.pos;
            if self.peek() != b'\\' {
                return RULE_NONE;
            }
            self.pos += 1;
            match self.read_name() {
                b"toprule" | b"bottomrule" => return RULE_HEAVY,
                b"midrule" => return RULE_LIGHT,
                b"hline" => return RULE_PLAIN,
                b"cline" | b"cmidrule" | b"specialrule" => {
                    self.skip_loose();
                    return RULE_LIGHT;
                }
                b"addlinespace" | b"endhead" | b"endfirsthead" | b"endfoot" | b"endlastfoot" | b"rowcolor" | b"noalign" | b"hhline"
                | b"rowcolors" | b"caption" | b"label" => {
                    self.skip_loose();
                    self.skip_ws();
                }
                _ => {
                    self.pos = start;
                    return RULE_NONE;
                }
            }
        }
    }

    fn table_spec(&mut self) -> R<([u8; MAX_COLUMNS], usize, u64, u64, bool)> {
        let spec = self.loose_argument();
        let mut aligns = [0u8; MAX_COLUMNS];
        let mut count = 0;
        let mut lines = 0u64;
        let mut flexible = 0u64;
        scan_spec(spec, &mut aligns, &mut count, &mut lines, &mut flexible, 0);
        if count == 0 {
            aligns[0] = b'l';
            count = 1;
        }
        let fill = spec.windows(5).any(|w| w == b"\\fill");
        Ok((aligns, count, lines, flexible, fill))
    }

    pub(super) fn table_width(&mut self, text: &[u8], width: i32) -> i32 {
        let text = figure::trim(text);
        for unit in [&b"\\textwidth"[..], b"\\linewidth", b"\\columnwidth", b"\\hsize"] {
            if let Some(scale) = text.strip_suffix(unit) {
                let scale = if scale.is_empty() { Some(1.0) } else { figure::length_pt(scale) };
                return scale.map_or(0, |v| (v * width as f64) as i32);
            }
        }
        figure::length_pt(text).map_or(0, |v| self.pt_units(v))
    }

    pub(super) fn table_block(&mut self, d: &mut Doc, left: i32, width: i32, name: &[u8]) -> R<()> {
        self.skip_ws();
        if self.peek() == b'[' {
            self.bracketed()?;
        }
        let (aligns, count, lines, flexible, fill) = self.table_spec()?;
        let target = core::mem::take(&mut d.table_width);
        let c = d.ctx;
        let mut cells = core::mem::take(&mut self.store.cells);
        let mut rows = core::mem::take(&mut self.store.rows);
        cells.clear();
        rows.clear();
        let done = self.table_rows(d, &c, count, &mut cells, &mut rows, name);
        let stretch = (target, flexible, fill);
        let placed = done.and_then(|()| self.table_place(d, &c, left, width, &aligns, count, lines, stretch, &cells, &rows));
        self.store.cells = cells;
        self.store.rows = rows;
        placed
    }

    fn table_rows(
        &mut self,
        d: &mut Doc,
        c: &Ctx,
        count: usize,
        cells: &mut Vec<(Bx, u16, u16)>,
        rows: &mut Vec<TableRow>,
        name: &[u8],
    ) -> R<()> {
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
                self.close_env(name)?;
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
                if column < count {
                    h = h.max(b.h);
                    dep = dep.max(b.d);
                    cells.push((b, row, column as u16));
                }
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
        stretch: (i32, u64, bool),
        cells: &[(Bx, u16, u16)],
        rows: &[TableRow],
    ) -> R<()> {
        let mut widths = [0i32; MAX_COLUMNS];
        for (b, _, column) in cells {
            let w = &mut widths[*column as usize];
            *w = (*w).max(b.w);
        }
        let gap = d.table_gap;
        let (target, flexible, fill) = stretch;
        let natural = widths[..count].iter().sum::<i32>() + 2 * gap * count as i32;
        let extra = (target - natural).max(0);
        let flex_count = (flexible & ((1u64 << count.min(63)) - 1)).count_ones() as i32;
        let mut starts = [0i32; MAX_COLUMNS + 1];
        let mut spread = 0;
        for i in 0..count {
            if flex_count > 0 && i < 63 && flexible & (1 << i) != 0 {
                widths[i] += extra / flex_count;
            }
            if fill && flex_count == 0 && count > 1 && i + 1 < count {
                spread = extra * (i as i32 + 1) / (count as i32 - 1);
            }
            starts[i + 1] = starts[i] + widths[i] + 2 * gap;
            if fill && flex_count == 0 && count > 1 && i + 1 < count {
                starts[i + 1] = natural_start(&widths, gap, i + 1) + spread;
            }
        }
        let total_w = if fill && flex_count == 0 && count > 1 { starts[count - 1] + widths[count - 1] + 2 * gap } else { starts[count] };
        let rule_of = |kind: u8| match kind {
            RULE_HEAVY => (sc(HEAVY_RULE, c.scale), sc(ABOVE_RULE, c.scale), sc(BELOW_RULE, c.scale)),
            RULE_LIGHT => (sc(LIGHT_RULE, c.scale), sc(ABOVE_RULE, c.scale), sc(BELOW_RULE, c.scale)),
            RULE_PLAIN => (PLAIN_RULE, 0, 0),
            _ => (0, 0, 0),
        };
        let mut y = 0;
        let mut at = 0;
        let mut first_base = None;
        let first_rule = self.end();
        for (index, row) in rows.iter().enumerate() {
            if row.rule != RULE_NONE {
                let (thick, above, below) = rule_of(row.rule);
                y += if index == 0 { 0 } else { above };
                self.push_rule(c, 0, y, total_w, thick);
                y += thick + below;
            }
            let base = y + row.h;
            if at < cells.len() && first_base.is_none() {
                first_base = Some(base);
            }
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
        self.place_label(d, left, line - y + first_base.unwrap_or(0));
        Ok(())
    }

    pub(super) fn caption(&mut self, d: &mut Doc, left: i32, width: i32) -> R<()> {
        if d.float == 0 {
            self.note(self.pos);
            self.skip_arguments();
            return Ok(());
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
        } else if d.float == 3 {
            d.algorithms += 1;
            (b"Algorithm", d.algorithms)
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
        f.float = match name.first() {
            Some(b'f') => 1,
            Some(b'a') => 3,
            _ => 2,
        };
        self.blocks(&mut f, 0, 0, name)?;
        self.close_env(name)?;
        d.figures = f.figures;
        d.tables = f.tables;
        d.algorithms = f.algorithms;
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

fn group_end(spec: &[u8], at: usize) -> usize {
    let mut depth = 0;
    let mut i = at;
    while i < spec.len() {
        match spec[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
        i += 1;
    }
    spec.len().saturating_sub(1)
}

fn scan_spec(spec: &[u8], aligns: &mut [u8; MAX_COLUMNS], count: &mut usize, lines: &mut u64, flexible: &mut u64, depth: u32) {
    let mut i = 0;
    while i < spec.len() {
        let ch = spec[i];
        match ch {
            b'l' | b'c' | b'r' | b'X' | b'S' | b's' | b'L' | b'C' | b'R' | b'p' | b'm' | b'b' | b'Y' | b'Z' => {
                if *count < MAX_COLUMNS {
                    if ch == b'X' && *count < 63 {
                        *flexible |= 1 << *count;
                    }
                    aligns[*count] = match ch {
                        b'c' | b'C' | b'S' | b's' => b'c',
                        b'r' | b'R' => b'r',
                        _ => b'l',
                    };
                    *count += 1;
                }
                if matches!(ch, b'p' | b'm' | b'b') && spec.get(i + 1) == Some(&b'{') {
                    i = group_end(spec, i + 1);
                }
            }
            b'|' => {
                if *count < 63 {
                    *lines |= 1 << *count;
                }
            }
            b'@' | b'!' | b'>' | b'<' => {
                if spec.get(i + 1) == Some(&b'{') {
                    i = group_end(spec, i + 1);
                }
            }
            b'*' if spec.get(i + 1) == Some(&b'{') && depth < 3 => {
                let end = group_end(spec, i + 1);
                let n = spec[i + 2..end]
                    .iter()
                    .filter(|c| c.is_ascii_digit())
                    .fold(0usize, |v, c| v * 10 + (*c - b'0') as usize)
                    .min(MAX_COLUMNS);
                if spec.get(end + 1) == Some(&b'{') {
                    let close = group_end(spec, end + 1);
                    for _ in 0..n {
                        scan_spec(&spec[end + 2..close], aligns, count, lines, flexible, depth + 1);
                    }
                    i = close;
                } else {
                    i = end;
                }
            }
            _ => {}
        }
        i += 1;
    }
}
