use super::fast_zip::Zip;
use super::*;
use crate::emit::brace_path;

// Type 3 fonts reuse the existing outlines and preserve their exact metrics.
// A ToUnicode map makes the same marks selectable and searchable.
const FONT_CHARS: usize = 255;

fn hex_code(out: &mut Vec<u8>, value: u32, digits: usize) {
    for i in (0..digits).rev() {
        out.push(b"0123456789ABCDEF"[((value >> (i * 4)) & 15) as usize]);
    }
}

const UNICODE: [u32; GLYPHS.len()] = {
    let mut out = [0xfffd; GLYPHS.len()];
    let mut i = 0;
    while i < GM_KEYS.len() {
        if GM_KEYS[i] != 0 {
            let id = GM_VALS[i] as usize;
            let cp = (GM_KEYS[i] - 1) % 0x200000;
            if cp < out[id] {
                out[id] = cp;
            }
        }
        i += 1;
    }
    out
};

#[derive(Default)]
pub(super) struct Scratch {
    offsets: Vec<usize>,
    forms: Vec<u32>,
    objects: Vec<(u32, usize)>,
    kids: Vec<usize>,
    body: Vec<u8>,
    form_spans: Vec<(u32, u32)>,
    form_bytes: Vec<u8>,
    zip: Zip,
}

const DIGITS: &[u8; 200] = b"00010203040506070809101112131415161718192021222324252627282930313233343536373839404142434445464748495051525354555657585960616263646566676869707172737475767778798081828384858687888990919293949596979899";

pub(super) fn int(out: &mut Vec<u8>, v: i32) {
    if v < 0 {
        out.push(b'-');
    }
    let mut n = v.unsigned_abs();
    if n < 10 {
        out.push(b'0' + n as u8);
        return;
    }
    if n < 100 {
        let pair = n as usize * 2;
        out.extend_from_slice(&DIGITS[pair..pair + 2]);
        return;
    }
    let mut digits = [0u8; 10];
    let mut i = digits.len();
    while n >= 100 {
        let pair = (n % 100) as usize * 2;
        n /= 100;
        i -= 2;
        digits[i] = DIGITS[pair];
        digits[i + 1] = DIGITS[pair + 1];
    }
    if n < 10 {
        i -= 1;
        digits[i] = b'0' + n as u8;
    } else {
        let pair = n as usize * 2;
        i -= 2;
        digits[i] = DIGITS[pair];
        digits[i + 1] = DIGITS[pair + 1];
    }
    out.extend_from_slice(&digits[i..]);
}

// Thousandths of a TeX point to thousandths of a PDF big point.
fn big_points(v: i32) -> i32 {
    (v as i64 * 800 / 803) as i32
}

fn milli(out: &mut Vec<u8>, v: i32) {
    if v < 0 {
        out.push(b'-');
    }
    let u = v.unsigned_abs() as i32;
    int(out, u / 1000);
    let f = u % 1000;
    if f != 0 {
        out.push(b'.');
        out.push(b'0' + (f / 100) as u8);
        out.push(b'0' + (f / 10 % 10) as u8);
        out.push(b'0' + (f % 10) as u8);
    }
}

fn rgb(out: &mut Vec<u8>, color: u32, op: &[u8]) {
    // Most drawing operations use these exact channel endpoints. Emit their
    // existing representation directly instead of formatting three numbers.
    let channels: Option<&[u8]> = match color & 0xffffff {
        0x000000 => Some(b"0 0 0 "),
        0xff0000 => Some(b"1 0 0 "),
        0x00ff00 => Some(b"0 1 0 "),
        0x0000ff => Some(b"0 0 1 "),
        0xffff00 => Some(b"1 1 0 "),
        0xff00ff => Some(b"1 0 1 "),
        0x00ffff => Some(b"0 1 1 "),
        0xffffff => Some(b"1 1 1 "),
        _ => None,
    };
    if let Some(channels) = channels {
        out.extend_from_slice(channels);
    } else {
        for shift in [16, 8, 0] {
            milli(out, ((color >> shift) & 255) as i32 * 1000 / 255);
            out.push(b' ');
        }
    }
    out.extend_from_slice(op);
    out.push(b'\n');
}

fn hex(out: &mut Vec<u8>, color: u32) {
    let digits = b"0123456789abcdef";
    out.push(b'#');
    for i in (0..6).rev() {
        out.push(digits[((color >> (i * 4)) & 15) as usize]);
    }
}

#[inline(always)]
fn ints(out: &mut Vec<u8>, values: &[i32], op: &[u8]) {
    for v in values {
        int(out, *v);
        out.push(b' ');
    }
    out.extend_from_slice(op);
    out.push(b'\n');
}

fn quad(out: &mut Vec<u8>, from: (i32, i32), control: (i32, i32), to: (i32, i32)) {
    let third = |a: i32, b: i32| a + (b - a) * 2 / 3;
    let c1 = (third(from.0, control.0), third(from.1, control.1));
    let c2 = (third(to.0, control.0), third(to.1, control.1));
    ints(out, &[c1.0, c1.1, c2.0, c2.1, to.0, to.1], b"c");
}

fn read_number(path: &[u8], i: &mut usize) -> Option<i32> {
    while *i < path.len() && matches!(path[*i], b' ' | b',') {
        *i += 1;
    }
    let negative = path.get(*i) == Some(&b'-');
    if negative {
        *i += 1;
    }
    let start = *i;
    let mut v = 0i32;
    while *i < path.len() && path[*i].is_ascii_digit() {
        v = v * 10 + (path[*i] - b'0') as i32;
        *i += 1;
    }
    if *i == start {
        return None;
    }
    Some(if negative { -v } else { v })
}

fn svg_path(out: &mut Vec<u8>, path: &[u8], origin: (i32, i32)) {
    let mut cur = origin;
    let mut start = origin;
    let mut control = origin;
    let mut command = 0u8;
    let mut i = 0;
    ints(out, &[cur.0, cur.1], b"m");
    while i < path.len() {
        let ch = path[i];
        if ch.is_ascii_alphabetic() {
            command = ch;
            i += 1;
            if ch == b'z' {
                out.extend_from_slice(b"h\n");
                cur = start;
                control = cur;
                continue;
            }
        } else if matches!(ch, b' ' | b',') {
            i += 1;
            continue;
        }
        let mut next = || read_number(path, &mut i).unwrap_or(0);
        match command {
            b'M' | b'm' => {
                let (x, y) = (next(), next());
                cur = if command == b'M' { (x, y) } else { (cur.0 + x, cur.1 + y) };
                start = cur;
                control = cur;
                ints(out, &[cur.0, cur.1], b"m");
                command = if command == b'M' { b'L' } else { b'l' };
            }
            b'l' => {
                let (x, y) = (next(), next());
                cur = (cur.0 + x, cur.1 + y);
                control = cur;
                ints(out, &[cur.0, cur.1], b"l");
            }
            b'h' => {
                cur.0 += next();
                control = cur;
                ints(out, &[cur.0, cur.1], b"l");
            }
            b'v' => {
                cur.1 += next();
                control = cur;
                ints(out, &[cur.0, cur.1], b"l");
            }
            b'q' => {
                let (ax, ay, x, y) = (next(), next(), next(), next());
                let c = (cur.0 + ax, cur.1 + ay);
                let to = (cur.0 + x, cur.1 + y);
                quad(out, cur, c, to);
                control = c;
                cur = to;
            }
            b'c' => {
                let (a, b, c, d, x, y) = (next(), next(), next(), next(), next(), next());
                ints(out, &[cur.0 + a, cur.1 + b, cur.0 + c, cur.1 + d, cur.0 + x, cur.1 + y], b"c");
                cur = (cur.0 + x, cur.1 + y);
                control = cur;
            }
            b't' => {
                let (x, y) = (next(), next());
                let c = (2 * cur.0 - control.0, 2 * cur.1 - control.1);
                let to = (cur.0 + x, cur.1 + y);
                quad(out, cur, c, to);
                control = c;
                cur = to;
            }
            _ => {
                i += 1;
            }
        }
    }
    out.extend_from_slice(b"f\n");
}

fn color_value(text: &[u8]) -> (i32, i32, i32) {
    let hex = |c: u8| (c as char).to_digit(16).unwrap_or(0) as i32;
    match text {
        [b'#', r, g, b] => (hex(*r) * 17 * 1000 / 255, hex(*g) * 17 * 1000 / 255, hex(*b) * 17 * 1000 / 255),
        [b'#', a, b, c, d, e, f] => {
            ((hex(*a) * 16 + hex(*b)) * 1000 / 255, (hex(*c) * 16 + hex(*d)) * 1000 / 255, (hex(*e) * 16 + hex(*f)) * 1000 / 255)
        }
        b"red" => (1000, 0, 0),
        b"green" => (0, 1000, 0),
        b"blue" => (0, 0, 1000),
        b"cyan" => (0, 1000, 1000),
        b"magenta" => (1000, 0, 1000),
        b"yellow" => (1000, 1000, 0),
        b"white" => (1000, 1000, 1000),
        b"gray" | b"grey" => (500, 500, 500),
        b"orange" => (1000, 500, 0),
        b"purple" => (500, 0, 500),
        _ => (0, 0, 0),
    }
}

const MIN_JOINED: usize = 128;
const MAX_JOINED: usize = 1024;

fn joins(a: &document::Art, b: &document::Art, segs: &[document::Seg]) -> bool {
    let stroked = |art: &document::Art| art.fill == document::NO_COLOR && art.stroke != document::NO_COLOR;
    stroked(a)
        && stroked(b)
        && (a.stroke, a.width, a.dash, a.round) == (b.stroke, b.width, b.dash, b.round)
        && segs.get(b.s as usize).is_some_and(|g| g.op == document::SEG_MOVE)
}

impl Engine {
    fn group_color(&self, id: u16) -> (i32, i32, i32) {
        let color = self.laid.groups[id as usize].color;
        if color == 0 {
            return (0, 0, 0);
        }
        let (o, l) = self.laid.colors[color as usize];
        color_value(&self.laid.color_bytes[o as usize..(o + l) as usize])
    }

    // Use final page glyph positions so wrapped links get one rectangle per line.
    fn page_annotations(&self, start: usize, end: usize) -> Vec<Vec<u8>> {
        if self.laid.links.is_empty() {
            return Vec::new();
        }
        let mut rectangles: Vec<(u32, i32, i32, i32, i32, i32)> = Vec::new();
        for item in &self.laid.items[start..end] {
            let group = self.laid.groups[item.g as usize];
            if group.link == 0 || item.kind != IT_GLYPH {
                continue;
            }
            let glyph = self.glyph_metric(item.a as usize);
            let left = item.x;
            let right = left + sc(glyph.w as i32, group.scale);
            let top = item.y - sc(glyph.h as i32, group.scale);
            let bottom = item.y + sc(glyph.d as i32, group.scale);
            if right <= left || bottom <= top {
                continue;
            }
            if let Some(last) = rectangles.last_mut() {
                if last.0 == group.link && last.5 == item.y && left >= last.1 && left - last.3 < 2000 {
                    last.2 = last.2.min(top);
                    last.3 = last.3.max(right);
                    last.4 = last.4.max(bottom);
                    continue;
                }
            }
            rectangles.push((group.link, left, top, right, bottom, item.y));
        }
        let mut annotations = Vec::new();
        for (link, left, top, right, bottom, _) in rectangles {
            let Some(target) = self.laid.links.get(link as usize - 1) else {
                continue;
            };
            let mut bytes = Vec::new();
            let out = &mut bytes;
            out.extend_from_slice(b"<< /Type /Annot /Subtype /Link /Border [0 0 0] /Rect [");
            for coordinate in
                [left * self.laid.pt, (self.laid.h - bottom) * self.laid.pt, right * self.laid.pt, (self.laid.h - top) * self.laid.pt]
            {
                milli(out, big_points(coordinate));
                out.push(b' ');
            }
            out.extend_from_slice(b"] /A << /S /URI /URI <");
            for &byte in target {
                hex_code(out, byte as u32, 2);
            }
            out.extend_from_slice(b"> >> >>");
            annotations.push(bytes);
        }
        annotations
    }

    fn paper_size(&self) -> (i32, i32) {
        match self.laid.paper {
            1 => (612000, 792000),
            2 => (595276, 841890),
            _ => (big_points(self.laid.w * self.laid.pt), big_points(self.laid.h * self.laid.pt)),
        }
    }

    fn page_stream(&self, b: &mut Vec<u8>, s: usize, e: usize, art: (usize, usize), forms: &[u32]) {
        b.clear();
        b.extend_from_slice(match self.laid.pt {
            11 => b"0.010958904 0 0 -0.010958904 0 ",
            12 => b"0.011955168 0 0 -0.011955168 0 ",
            _ => b"0.009962640 0 0 -0.009962640 0 ",
        });
        milli(b, self.paper_size().1);
        b.extend_from_slice(b" cm\n");
        let mut color = (0, 0, 0);
        for i in art.0..art.1 {
            let a = self.laid.art[i];
            self.art_stream(b, &a, forms);
        }
        self.item_stream(b, &self.laid.items[s..e], forms, &mut color);
    }

    fn art_stream(&self, b: &mut Vec<u8>, a: &document::Art, forms: &[u32]) {
        b.extend_from_slice(b"q\n");
        if a.text {
            milli(b, a.cos);
            b.push(b' ');
            milli(b, a.sin);
            b.push(b' ');
            milli(b, -a.sin);
            b.push(b' ');
            milli(b, a.cos);
            b.push(b' ');
            int(b, a.x);
            b.push(b' ');
            int(b, a.y);
            b.extend_from_slice(b" cm\n");
            let mut color = (0, 0, 0);
            self.item_stream(b, &self.laid.spun[a.s as usize..a.e as usize], forms, &mut color);
            b.extend_from_slice(b"Q\n");
            return;
        }
        let stroked = a.stroke != document::NO_COLOR;
        let filled = a.fill != document::NO_COLOR;
        if stroked {
            rgb(b, a.stroke, b"RG");
            int(b, a.width);
            b.extend_from_slice(b" w\n");
            if a.round {
                b.extend_from_slice(b"1 J 1 j\n");
            }
            if a.dash.0 > 0 {
                b.push(b'[');
                int(b, a.dash.0);
                b.push(b' ');
                int(b, a.dash.1);
                b.extend_from_slice(b"] 0 d\n");
            }
        }
        if filled {
            rgb(b, a.fill, b"rg");
        }
        let segs = &self.laid.segs[a.s as usize..a.e as usize];
        let mut i = 0;
        while i < segs.len() {
            let g = segs[i];
            let (x, y) = (g.x + a.x, g.y + a.y);
            match g.op {
                document::SEG_MOVE => ints(b, &[x, y], b"m"),
                document::SEG_LINE => ints(b, &[x, y], b"l"),
                document::SEG_CURVE if i + 2 < segs.len() => {
                    let (p, q) = (segs[i + 1], segs[i + 2]);
                    ints(b, &[x, y, p.x + a.x, p.y + a.y, q.x + a.x, q.y + a.y], b"c");
                    i += 2;
                }
                document::SEG_CLOSE => b.extend_from_slice(b"h\n"),
                _ => {}
            }
            i += 1;
        }
        b.extend_from_slice(match (stroked, filled) {
            (true, true) => b"B\nQ\n",
            (true, false) => b"S\nQ\n",
            (false, true) => b"f\nQ\n",
            _ => b"n\nQ\n",
        });
    }

    fn item_stream(&self, b: &mut Vec<u8>, items: &[Item], _forms: &[u32], color: &mut (i32, i32, i32)) {
        let mut at = 0;
        let mut text_open = false;
        let mut font = usize::MAX;
        let mut previous_scale = 0;
        let mut origin = (0, 0);
        while at < items.len() {
            let it = &items[at];
            let next = self.group_color(it.g);
            if next != *color {
                *color = next;
                milli(b, color.0);
                b.push(b' ');
                milli(b, color.1);
                b.push(b' ');
                milli(b, color.2);
                b.extend_from_slice(b" rg\n");
            }
            if it.kind != IT_GLYPH && text_open {
                b.extend_from_slice(b"ET\n");
                text_open = false;
            }
            match it.kind {
                IT_GLYPH => {
                    let scale = self.laid.groups[it.g as usize].scale as i32;
                    let id = it.a as usize;
                    if scale == 0 {
                        at += 1;
                        continue;
                    }
                    if !text_open {
                        b.extend_from_slice(b"BT ");
                        text_open = true;
                        font = usize::MAX;
                        previous_scale = 0;
                    }
                    let face = id / FONT_CHARS;
                    if font != face {
                        b.extend_from_slice(b"/F");
                        int(b, face as i32);
                        b.extend_from_slice(b" 1000 Tf ");
                        font = face;
                    }
                    let dx = (it.x as i64 - origin.0 as i64) * 1_000_000 / scale as i64;
                    let dy = (origin.1 as i64 - it.y as i64) * 1_000_000 / scale as i64;
                    if previous_scale == scale && dx.abs() <= i32::MAX as i64 && dy.abs() <= i32::MAX as i64 {
                        milli(b, dx as i32);
                        b.push(b' ');
                        milli(b, dy as i32);
                        b.extend_from_slice(b" Td <");
                    } else {
                        milli(b, scale);
                        b.extend_from_slice(b" 0 0 ");
                        milli(b, -scale);
                        b.push(b' ');
                        int(b, it.x);
                        b.push(b' ');
                        int(b, it.y);
                        b.extend_from_slice(b" Tm <");
                    }
                    previous_scale = scale;
                    origin = (it.x, it.y);
                    hex_code(b, (id % FONT_CHARS + 1) as u32, 2);
                    let mut expected = it.x + sc(self.glyph_metric(id).w as i32, scale as u16);
                    while at + 1 < items.len() {
                        let next = items[at + 1];
                        if next.kind != IT_GLYPH
                            || next.g != it.g
                            || next.y != it.y
                            || next.x != expected
                            || next.a as usize / FONT_CHARS != id / FONT_CHARS
                        {
                            break;
                        }
                        hex_code(b, (next.a as usize % FONT_CHARS + 1) as u32, 2);
                        expected += sc(self.glyph_metric(next.a as usize).w as i32, scale as u16);
                        at += 1;
                    }
                    b.extend_from_slice(b"> Tj\n");
                }
                IT_IMAGE => {
                    let im = &self.laid.images[it.a as usize];
                    b.extend_from_slice(b"q ");
                    ints(b, &[im.size.0, 0, 0, -im.size.1, it.x, it.y], b"cm");
                    b.extend_from_slice(b"/I");
                    int(b, it.a);
                    b.extend_from_slice(b" Do Q\n");
                }
                IT_RULE => ints(b, &[it.x, it.y, it.a, it.b], b"re f"),
                IT_SLASH => {
                    ints(b, &[it.x, it.y + it.b], b"m");
                    ints(b, &[it.x + it.a, it.y], b"l");
                    ints(b, &[it.x + it.a + 50, it.y], b"l");
                    ints(b, &[it.x + 50, it.y + it.b], b"l");
                    b.extend_from_slice(b"h f\n");
                }
                IT_BACKSLASH => {
                    ints(b, &[it.x, it.y], b"m");
                    ints(b, &[it.x + 50, it.y], b"l");
                    ints(b, &[it.x + 50 + it.a, it.y + it.b], b"l");
                    ints(b, &[it.x + it.a, it.y + it.b], b"l");
                    b.extend_from_slice(b"h f\n");
                }
                _ => {
                    let mut text = [0u8; 512];
                    let len = brace_path(it, &mut text);
                    let at = text[1..len].iter().position(|c| *c == b'q').map_or(len, |n| n + 1);
                    let mut i = 1;
                    let x = read_number(&text[..at], &mut i).unwrap_or(0);
                    let y = read_number(&text[..at], &mut i).unwrap_or(0);
                    svg_path(b, &text[at..len], (x, y));
                }
            }
            at += 1;
        }
        if text_open {
            b.extend_from_slice(b"ET\n");
        }
    }

    fn object(&mut self, offsets: &mut Vec<usize>, id: usize) {
        if offsets.len() <= id {
            offsets.resize(id + 1, 0);
        }
        offsets[id] = self.out.len();
        int(&mut self.out, id as i32);
        self.out.extend_from_slice(b" 0 obj\n");
    }

    fn stream(&mut self, head: &[u8], body: &[u8], zip: &mut Zip) {
        let compressed = if !head.windows(7).any(|w| w == b"/Filter") && body.len() >= 131072 { zip.compress(body) } else { &[] };
        let use_compressed = !compressed.is_empty() && compressed.len() < body.len();
        let body = if use_compressed { compressed } else { body };
        self.out.extend_from_slice(b"<< ");
        if use_compressed {
            self.out.extend_from_slice(b"/Filter /FlateDecode ");
        }
        self.out.extend_from_slice(head);
        self.out.extend_from_slice(b" /Length ");
        int(&mut self.out, body.len() as i32);
        self.out.extend_from_slice(b" >>\nstream\n");
        self.out.extend_from_slice(body);
        self.out.extend_from_slice(b"\nendstream\nendobj\n");
    }

    fn art_segments(&self, a: &document::Art, out: &mut Vec<u8>) {
        let segs = &self.laid.segs[a.s as usize..a.e as usize];
        let mut k = 0;
        while k < segs.len() {
            let g = segs[k];
            let (x, y) = (g.x + a.x, g.y + a.y);
            match g.op {
                document::SEG_MOVE | document::SEG_LINE => {
                    out.push(if g.op == document::SEG_MOVE { b'M' } else { b'L' });
                    int(out, x);
                    out.push(b' ');
                    int(out, y);
                }
                document::SEG_CURVE if k + 2 < segs.len() => {
                    let (p, q) = (segs[k + 1], segs[k + 2]);
                    out.push(b'C');
                    for (i, v) in [x, y, p.x + a.x, p.y + a.y, q.x + a.x, q.y + a.y].iter().enumerate() {
                        if i > 0 {
                            out.push(b' ');
                        }
                        int(out, *v);
                    }
                    k += 2;
                }
                document::SEG_CLOSE => out.push(b'Z'),
                _ => {}
            }
            k += 1;
        }
    }

    pub(super) fn art_svg(&mut self, from: usize, to: usize) {
        let mut out = core::mem::take(&mut self.art);
        out.clear();
        let mut i = from;
        while i < to {
            let a = self.laid.art[i];
            i += 1;
            if a.text {
                out.extend_from_slice(b"<g transform=\"matrix(");
                milli(&mut out, a.cos);
                out.push(b' ');
                milli(&mut out, a.sin);
                out.push(b' ');
                milli(&mut out, -a.sin);
                out.push(b' ');
                milli(&mut out, a.cos);
                out.push(b' ');
                int(&mut out, a.x);
                out.push(b' ');
                int(&mut out, a.y);
                out.extend_from_slice(b")\">");
                self.text_paths(&self.laid.spun[a.s as usize..a.e as usize], &mut out);
                out.extend_from_slice(b"</g>\n");
                continue;
            }
            out.extend_from_slice(b"<path d=\"");
            self.art_segments(&a, &mut out);
            let run = self.laid.art[i..to].iter().take(MIN_JOINED).take_while(|next| joins(&a, next, &self.laid.segs)).count();
            if run + 1 >= MIN_JOINED {
                let mut joined = 1;
                while joined < MAX_JOINED && i < to && joins(&a, &self.laid.art[i], &self.laid.segs) {
                    let next = self.laid.art[i];
                    self.art_segments(&next, &mut out);
                    i += 1;
                    joined += 1;
                }
            }
            out.extend_from_slice(b"\" fill=\"");
            if a.fill == document::NO_COLOR {
                out.extend_from_slice(b"none");
            } else {
                hex(&mut out, a.fill);
            }
            out.push(b'"');
            if a.stroke != document::NO_COLOR {
                out.extend_from_slice(b" stroke=\"");
                hex(&mut out, a.stroke);
                out.extend_from_slice(b"\" stroke-width=\"");
                int(&mut out, a.width);
                out.push(b'"');
                if a.dash.0 > 0 {
                    out.extend_from_slice(b" stroke-dasharray=\"");
                    int(&mut out, a.dash.0);
                    out.push(b' ');
                    int(&mut out, a.dash.1);
                    out.push(b'"');
                }
                if a.round {
                    out.extend_from_slice(b" stroke-linecap=\"round\" stroke-linejoin=\"round\"");
                }
            }
            out.extend_from_slice(b"/>\n");
        }
        self.art = out;
    }

    pub fn document_pdf(&mut self) -> i32 {
        const CATALOG: usize = 1;
        const PAGES: usize = 2;
        const RESOURCES: usize = 3;
        let mut scratch = core::mem::take(&mut self.scratch);
        let Scratch { offsets, forms, objects, kids, body, form_spans, form_bytes, zip } = &mut scratch;
        form_spans.resize(self.glyph_count(), (0, 0));
        offsets.clear();
        forms.clear();
        forms.resize(self.glyph_count(), 0);
        objects.clear();
        kids.clear();
        let mut next = 4;
        self.out.clear();
        self.out.extend_from_slice(b"%PDF-1.4\n");

        for i in 0..self.laid.items.len() + self.laid.spun.len() {
            let it = if i < self.laid.items.len() { self.laid.items[i] } else { self.laid.spun[i - self.laid.items.len()] };
            if it.kind != IT_GLYPH || forms[it.a as usize] != 0 {
                continue;
            }
            let id = it.a as usize;
            forms[id] = objects.len() as u32 + 1;
            if form_spans[id].1 == 0 {
                let g = self.glyph_metric(id);
                let at = form_bytes.len();
                int(form_bytes, g.w as i32);
                form_bytes.extend_from_slice(b" 0 d0\n");
                svg_path(form_bytes, self.glyph_path(id), (g.sx as i32, g.sy as i32));
                form_spans[id] = (at as u32, form_bytes.len() as u32);
            }
            let (from, to) = form_spans[id];
            self.object(offsets, next);
            self.stream(b"", &form_bytes[from as usize..to as usize], zip);
            objects.push((forms[id], next));
            next += 1;
        }

        let mut start = 0;
        let mut art_start = 0;
        for p in 0..self.laid.pages.len() {
            let end = self.laid.pages[p] as usize;
            let art_end = self.laid.art_pages[p] as usize;
            self.page_stream(body, start, end, (art_start, art_end), forms);
            let page_start = start;
            start = end;
            art_start = art_end;
            let content = next;
            self.object(offsets, content);
            self.stream(b"", body, zip);
            next += 1;
            let annotations = self.page_annotations(page_start, end);
            let first_annotation = next;
            for annotation in &annotations {
                self.object(offsets, next);
                self.out.extend_from_slice(annotation);
                self.out.extend_from_slice(b"\nendobj\n");
                next += 1;
            }
            self.object(offsets, next);
            let (w, h) = self.paper_size();
            self.out.extend_from_slice(b"<< /Type /Page /Parent 2 0 R /Resources 3 0 R /MediaBox [0 0 ");
            milli(&mut self.out, w);
            self.out.push(b' ');
            milli(&mut self.out, h);
            self.out.extend_from_slice(b"] /Contents ");
            int(&mut self.out, content as i32);
            self.out.extend_from_slice(b" 0 R");
            if !annotations.is_empty() {
                self.out.extend_from_slice(b" /Annots [");
                for id in first_annotation..next {
                    int(&mut self.out, id as i32);
                    self.out.extend_from_slice(b" 0 R ");
                }
                self.out.push(b']');
            }
            self.out.extend_from_slice(b" >>\nendobj\n");
            kids.push(next);
            next += 1;
        }

        let mut fonts = Vec::new();
        for first in (0..self.glyph_count()).step_by(FONT_CHARS) {
            let end = (first + FONT_CHARS).min(self.glyph_count());
            if !forms[first..end].iter().any(|id| *id != 0) {
                continue;
            }
            let cmap = next;
            next += 1;
            body.clear();
            body.extend_from_slice(b"/CIDInit /ProcSet findresource begin 12 dict begin begincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /Quicktex def /CMapType 2 def\n1 begincodespacerange <01> <FF> endcodespacerange\n");
            let used: Vec<_> = (first..end).filter(|i| forms[*i] != 0).collect();
            for chunk in used.chunks(100) {
                int(body, chunk.len() as i32);
                body.extend_from_slice(b" beginbfchar\n");
                for id in chunk {
                    body.push(b'<');
                    hex_code(body, (*id - first + 1) as u32, 2);
                    body.extend_from_slice(b"> <");
                    let cp = if *id < GLYPHS.len() { UNICODE[*id] } else { self.fonts.unicode[*id - GLYPHS.len()] };
                    if cp > 0xffff {
                        hex_code(body, 0xd800 + ((cp - 0x10000) >> 10), 4);
                        hex_code(body, 0xdc00 + ((cp - 0x10000) & 1023), 4);
                    } else {
                        hex_code(body, cp, 4);
                    }
                    body.extend_from_slice(b">\n");
                }
                body.extend_from_slice(b"endbfchar\n");
            }
            body.extend_from_slice(b"endcmap CMapName currentdict /CMap defineresource pop end end\n");
            self.object(offsets, cmap);
            self.stream(b"", body, zip);
            self.object(offsets, next);
            fonts.push((first / FONT_CHARS, next));
            next += 1;
            let (mut x0, mut y0, mut x1, mut y1) = (0, 0, 0, 0);
            for id in &used {
                let g = self.glyph_metric(*id);
                x0 = x0.min(g.sx as i32);
                y0 = y0.min(-g.h as i32);
                x1 = x1.max(g.w as i32 + g.it as i32);
                y1 = y1.max(g.d as i32);
            }
            self.out.extend_from_slice(b"<< /Type /Font /Subtype /Type3 /FontBBox [");
            for value in [x0, y0, x1, y1] {
                int(&mut self.out, value);
                self.out.push(b' ');
            }
            let low = *used.first().unwrap();
            let high = *used.last().unwrap() + 1;
            self.out.extend_from_slice(b"] /FontMatrix [0.001 0 0 -0.001 0 0] /Resources << >> /FirstChar ");
            int(&mut self.out, (low - first + 1) as i32);
            self.out.extend_from_slice(b" /LastChar ");
            int(&mut self.out, (high - first) as i32);
            self.out.extend_from_slice(b" /Widths [");
            for id in low..high {
                let w = self.glyph_metric(id).w;
                int(&mut self.out, w as i32);
                self.out.push(b' ');
            }
            self.out.extend_from_slice(b"] /Encoding << /Type /Encoding /Differences [");
            let mut previous = usize::MAX;
            for id in &used {
                let code = id - first + 1;
                if previous == usize::MAX || code != previous + 1 {
                    self.out.push(b' ');
                    int(&mut self.out, code as i32);
                }
                self.out.extend_from_slice(b" /g");
                int(&mut self.out, *id as i32);
                previous = code;
            }
            self.out.extend_from_slice(b"] >> /CharProcs <<");
            for id in first..end {
                if forms[id] == 0 {
                    continue;
                }
                self.out.extend_from_slice(b" /g");
                int(&mut self.out, id as i32);
                self.out.push(b' ');
                int(&mut self.out, objects[(forms[id] - 1) as usize].1 as i32);
                self.out.extend_from_slice(b" 0 R");
            }
            self.out.extend_from_slice(b" >> /ToUnicode ");
            int(&mut self.out, cmap as i32);
            self.out.extend_from_slice(b" 0 R >>\nendobj\n");
        }
        let mut images = Vec::new();
        for i in 0..self.laid.images.len() {
            let im = self.laid.images[i].clone();
            let alpha = if !im.alpha.is_empty() {
                let id = next;
                next += 1;
                let head = alloc::format!(
                    "/Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceGray /BitsPerComponent 8",
                    im.pixels.0,
                    im.pixels.1
                );
                self.object(offsets, id);
                self.stream(head.as_bytes(), &im.alpha, zip);
                Some(id)
            } else {
                None
            };
            let mut head = alloc::format!(
                "/Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8",
                im.pixels.0,
                im.pixels.1
            );
            if im.jpeg {
                head.push_str(" /Filter /DCTDecode");
            }
            if let Some(id) = alpha {
                head.push_str(&alloc::format!(" /SMask {id} 0 R"));
            }
            self.object(offsets, next);
            self.stream(head.as_bytes(), &im.data, zip);
            images.push(next);
            next += 1;
        }
        self.object(offsets, RESOURCES);
        self.out.extend_from_slice(b"<< /Font <<");
        for (name, id) in fonts {
            self.out.extend_from_slice(b" /F");
            int(&mut self.out, name as i32);
            self.out.push(b' ');
            int(&mut self.out, id as i32);
            self.out.extend_from_slice(b" 0 R");
        }
        self.out.extend_from_slice(b" >> /XObject <<");
        for (i, id) in images.iter().enumerate() {
            self.out.extend_from_slice(b" /I");
            int(&mut self.out, i as i32);
            self.out.push(b' ');
            int(&mut self.out, *id as i32);
            self.out.extend_from_slice(b" 0 R");
        }
        self.out.extend_from_slice(b" >> >>\nendobj\n");

        self.object(offsets, PAGES);
        self.out.extend_from_slice(b"<< /Type /Pages /Count ");
        int(&mut self.out, kids.len() as i32);
        self.out.extend_from_slice(b" /Kids [");
        for id in kids.iter() {
            int(&mut self.out, *id as i32);
            self.out.extend_from_slice(b" 0 R ");
        }
        self.out.extend_from_slice(b"] >>\nendobj\n");

        self.object(offsets, CATALOG);
        self.out.extend_from_slice(b"<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        let xref = self.out.len();
        self.out.extend_from_slice(b"xref\n0 ");
        int(&mut self.out, offsets.len() as i32);
        self.out.extend_from_slice(b"\n0000000000 65535 f \n");
        for offset in &offsets[1..] {
            let mut digits = [b'0'; 10];
            let mut n = *offset;
            let mut i = digits.len();
            while n > 0 {
                i -= 1;
                digits[i] = b'0' + (n % 10) as u8;
                n /= 10;
            }
            self.out.extend_from_slice(&digits);
            self.out.extend_from_slice(b" 00000 n \n");
        }
        self.out.extend_from_slice(b"trailer\n<< /Size ");
        int(&mut self.out, offsets.len() as i32);
        self.out.extend_from_slice(b" /Root 1 0 R >>\nstartxref\n");
        int(&mut self.out, xref as i32);
        self.out.extend_from_slice(b"\n%%EOF\n");
        self.scratch = scratch;
        self.out.len() as i32
    }
}
