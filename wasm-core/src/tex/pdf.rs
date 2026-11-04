use super::*;
use crate::emit::brace_path;

const FORM: &[u8] = b"/Type /XObject /Subtype /Form /BBox [-30000 -30000 30000 30000]";

#[derive(Default)]
pub(super) struct Scratch {
    offsets: Vec<usize>,
    forms: Vec<u32>,
    objects: Vec<(u32, usize)>,
    kids: Vec<usize>,
    body: Vec<u8>,
}

fn int(out: &mut Vec<u8>, v: i32) {
    if v < 0 {
        out.push(b'-');
    }
    let mut digits = [0u8; 10];
    let mut n = v.unsigned_abs();
    let mut i = digits.len();
    loop {
        i -= 1;
        digits[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    out.extend_from_slice(&digits[i..]);
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
    for shift in [16, 8, 0] {
        milli(out, ((color >> shift) & 255) as i32 * 1000 / 255);
        out.push(b' ');
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
        [b'#', a, b, c, d, e, f] => (
            (hex(*a) * 16 + hex(*b)) * 1000 / 255,
            (hex(*c) * 16 + hex(*d)) * 1000 / 255,
            (hex(*e) * 16 + hex(*f)) * 1000 / 255,
        ),
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

impl Engine {
    fn group_color(&self, id: u16) -> (i32, i32, i32) {
        let color = self.laid.groups[id as usize].color;
        if color == 0 {
            return (0, 0, 0);
        }
        let (o, l) = self.laid.colors[color as usize];
        color_value(&self.laid.color_bytes[o as usize..(o + l) as usize])
    }

    fn page_stream(&self, b: &mut Vec<u8>, s: usize, e: usize, art: (usize, usize), forms: &[u32]) {
        b.clear();
        milli(b, self.laid.pt);
        b.extend_from_slice(b" 0 0 ");
        milli(b, -self.laid.pt);
        b.extend_from_slice(b" 0 ");
        milli(b, self.laid.h * self.laid.pt);
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

    fn item_stream(&self, b: &mut Vec<u8>, items: &[Item], forms: &[u32], color: &mut (i32, i32, i32)) {
        for it in items {
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
            match it.kind {
                IT_GLYPH => {
                    let scale = self.laid.groups[it.g as usize].scale as i32;
                    b.extend_from_slice(b"q ");
                    milli(b, scale);
                    b.extend_from_slice(b" 0 0 ");
                    milli(b, scale);
                    b.push(b' ');
                    int(b, it.x);
                    b.push(b' ');
                    int(b, it.y);
                    b.extend_from_slice(b" cm /G");
                    int(b, forms[it.a as usize] as i32);
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

    fn stream(&mut self, head: &[u8], body: &[u8]) {
        self.out.extend_from_slice(b"<< ");
        self.out.extend_from_slice(head);
        self.out.extend_from_slice(b" /Length ");
        int(&mut self.out, body.len() as i32);
        self.out.extend_from_slice(b" >>\nstream\n");
        self.out.extend_from_slice(body);
        self.out.extend_from_slice(b"\nendstream\nendobj\n");
    }

    pub(super) fn art_svg(&mut self, from: usize, to: usize) {
        let mut out = core::mem::take(&mut self.art);
        out.clear();
        for i in from..to {
            let a = self.laid.art[i];
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
                out.extend_from_slice(b"</g>");
                continue;
            }
            out.extend_from_slice(b"<path d=\"");
            let segs = &self.laid.segs[a.s as usize..a.e as usize];
            let mut k = 0;
            while k < segs.len() {
                let g = segs[k];
                let (x, y) = (g.x + a.x, g.y + a.y);
                match g.op {
                    document::SEG_MOVE | document::SEG_LINE => {
                        out.push(if g.op == document::SEG_MOVE { b'M' } else { b'L' });
                        int(&mut out, x);
                        out.push(b' ');
                        int(&mut out, y);
                    }
                    document::SEG_CURVE if k + 2 < segs.len() => {
                        let (p, q) = (segs[k + 1], segs[k + 2]);
                        out.push(b'C');
                        for (i, v) in [x, y, p.x + a.x, p.y + a.y, q.x + a.x, q.y + a.y].iter().enumerate() {
                            if i > 0 {
                                out.push(b' ');
                            }
                            int(&mut out, *v);
                        }
                        k += 2;
                    }
                    document::SEG_CLOSE => out.push(b'Z'),
                    _ => {}
                }
                k += 1;
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
            out.extend_from_slice(b"/>");
        }
        self.art = out;
    }

    pub fn document_pdf(&mut self) -> i32 {
        const CATALOG: usize = 1;
        const PAGES: usize = 2;
        const RESOURCES: usize = 3;
        let mut scratch = core::mem::take(&mut self.scratch);
        let Scratch { offsets, forms, objects, kids, body } = &mut scratch;
        offsets.clear();
        forms.clear();
        forms.resize(GLYPHS.len(), 0);
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
            let g = &GLYPHS[id];
            body.clear();
            svg_path(body, &PATHS[g.off as usize..g.off as usize + g.len as usize], (g.sx as i32, g.sy as i32));
            self.object(offsets, next);
            self.stream(FORM, body);
            objects.push((forms[id], next));
            next += 1;
        }

        let mut start = 0;
        let mut art_start = 0;
        for p in 0..self.laid.pages.len() {
            let end = self.laid.pages[p] as usize;
            let art_end = self.laid.art_pages[p] as usize;
            self.page_stream(body, start, end, (art_start, art_end), forms);
            start = end;
            art_start = art_end;
            let content = next;
            self.object(offsets, content);
            self.stream(b"", body);
            next += 1;
            self.object(offsets, next);
            let (w, h) = (self.laid.w * self.laid.pt, self.laid.h * self.laid.pt);
            self.out.extend_from_slice(b"<< /Type /Page /Parent 2 0 R /Resources 3 0 R /MediaBox [0 0 ");
            milli(&mut self.out, w);
            self.out.push(b' ');
            milli(&mut self.out, h);
            self.out.extend_from_slice(b"] /Contents ");
            int(&mut self.out, content as i32);
            self.out.extend_from_slice(b" 0 R >>\nendobj\n");
            kids.push(next);
            next += 1;
        }

        self.object(offsets, RESOURCES);
        self.out.extend_from_slice(b"<< /XObject <<");
        for (name, id) in objects.iter() {
            self.out.extend_from_slice(b" /G");
            int(&mut self.out, *name as i32);
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
