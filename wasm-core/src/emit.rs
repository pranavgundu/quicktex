use crate::tex::*;
use alloc::vec::Vec;

static DIGITS: [u8; 200] = {
    let mut t = [0u8; 200];
    let mut i = 0;
    while i < 100 {
        t[i * 2] = b'0' + (i / 10) as u8;
        t[i * 2 + 1] = b'0' + (i % 10) as u8;
        i += 1;
    }
    t
};

#[inline(always)]
unsafe fn put(p: &mut *mut u8, b: &[u8]) {
    unsafe {
        core::ptr::copy_nonoverlapping(b.as_ptr(), *p, b.len());
        *p = p.add(b.len());
    }
}

#[inline(always)]
unsafe fn byte(p: &mut *mut u8, b: u8) {
    unsafe {
        **p = b;
        *p = p.add(1);
    }
}

#[inline(always)]
unsafe fn num(p: &mut *mut u8, v: i32) {
    unsafe {
        if v < 0 {
            byte(p, b'-');
        }
        let mut u = v.unsigned_abs();
        if u < 10 {
            byte(p, b'0' + u as u8);
            return;
        }
        if u < 100 {
            core::ptr::copy_nonoverlapping(DIGITS.as_ptr().add(u as usize * 2), *p, 2);
            *p = p.add(2);
            return;
        }
        if u < 1000 {
            byte(p, b'0' + (u / 100) as u8);
            core::ptr::copy_nonoverlapping(DIGITS.as_ptr().add((u % 100) as usize * 2), *p, 2);
            *p = p.add(2);
            return;
        }
        if u < 10000 {
            let hi = u / 100;
            let lo = u % 100;
            core::ptr::copy_nonoverlapping(DIGITS.as_ptr().add(hi as usize * 2), *p, 2);
            core::ptr::copy_nonoverlapping(DIGITS.as_ptr().add(lo as usize * 2), p.add(2), 2);
            *p = p.add(4);
            return;
        }
        let mut buf = [0u8; 10];
        let mut i = 10;
        while u > 0 {
            i -= 1;
            buf[i] = b'0' + (u % 10) as u8;
            u /= 10;
        }
        put(p, &buf[i..]);
    }
}

#[inline(always)]
unsafe fn pair(p: &mut *mut u8, x: i32, y: i32) {
    unsafe {
        num(p, x);
        if y >= 0 {
            byte(p, b' ');
        }
        num(p, y);
    }
}

unsafe fn milli(p: &mut *mut u8, v: i32) {
    unsafe {
        if v < 0 {
            byte(p, b'-');
        }
        let v = v.unsigned_abs() as i64;
        num(p, (v / 1000) as i32);
        let f = (v % 1000) as i32;
        if f != 0 {
            byte(p, b'.');
            byte(p, b'0' + (f / 100) as u8);
            byte(p, b'0' + (f / 10 % 10) as u8);
            byte(p, b'0' + (f % 10) as u8);
        }
    }
}

unsafe fn em(p: &mut *mut u8, v: i32) {
    unsafe {
        milli(p, v);
        put(p, b"em");
    }
}

unsafe fn quad(p: &mut *mut u8, ax: i32, ay: i32, bx: i32, by: i32) {
    unsafe {
        byte(p, b'q');
        pair(p, ax, ay);
        byte(p, b' ');
        pair(p, bx, by);
    }
}

unsafe fn across(p: &mut *mut u8, dx: i32) {
    unsafe {
        byte(p, b'h');
        num(p, dx);
    }
}

unsafe fn brace(p: &mut *mut u8, it: &Item) {
    unsafe {
        let w = it.a.max(120);
        let h = it.b;
        let under = it.kind == IT_UNDERBRACE;
        let t = (h / 8).max(20);
        let r = (h / 2).max(30).min(w / 4);
        let hh = h / 2;
        let s = if under { -1 } else { 1 };
        let (x0, y0) = if under { (it.x + w, it.y) } else { (it.x, it.y + h) };
        let span = w / 2 - 2 * r;
        let rest = h - hh;
        let thin = h - hh - t / 2;
        let wide = r + t / 2;
        byte(p, b'M');
        pair(p, x0, y0);
        quad(p, 0, -s * hh, s * r, -s * hh);
        across(p, s * span);
        quad(p, s * r, 0, s * r, -s * rest);
        quad(p, 0, s * rest, s * r, s * rest);
        across(p, s * span);
        quad(p, s * r, 0, s * r, s * hh);
        across(p, -s * t);
        quad(p, 0, -s * (hh - t), -s * (r - t), -s * (hh - t));
        across(p, -s * span);
        quad(p, -s * wide, 0, -s * wide, -s * thin);
        quad(p, 0, s * thin, -s * wide, s * thin);
        across(p, -s * span);
        quad(p, -s * (r - t), 0, -s * (r - t), s * (hh - t));
        byte(p, b'z');
    }
}

pub(crate) fn brace_path(it: &Item, out: &mut [u8; 512]) -> usize {
    unsafe {
        let base = out.as_mut_ptr();
        let mut q = base;
        brace(&mut q, it);
        q.offset_from(base) as usize
    }
}

const LINE_BAND: i32 = 1000;

impl Engine {
    pub(crate) fn emit(&mut self, root: Bx) {
        self.emit_as(root, false);
    }

    pub(crate) fn emit_as(&mut self, root: Bx, banded: bool) {
        let n = self.items.len();
        let ng = self.groups.len();
        let mut cap = 512 + ng * 96 + self.art.len();
        if banded {
            cap += n * 128;
        }
        self.counts.clear();
        self.counts.resize(ng + 1, 0);
        for it in &self.items {
            self.counts[it.g as usize + 1] += 1;
            cap += match it.kind {
                IT_GLYPH => self.glyph_path(it.a as usize).len() + 40,
                IT_RULE => 72,
                IT_IMAGE => self.laid.images[it.a as usize].url.len() + 200,
                _ => 512,
            };
        }
        self.out.clear();
        self.out.reserve(cap);
        for i in 1..=ng {
            self.counts[i] += self.counts[i - 1];
        }
        let single = !banded && self.counts[1] as usize == n;
        if !single {
            self.order.clear();
            self.order.resize(n, 0);
            for i in 0..n {
                let g = self.items[i].g as usize;
                let k = self.counts[g];
                self.order[k as usize] = i as u32;
                self.counts[g] = k + 1;
            }
        }
        unsafe {
            let base = self.out.as_mut_ptr();
            let mut q = base;
            let p = &mut q;
            let h = root.h.max(0);
            let d = root.d.max(0);
            let w = root.w.max(0);
            put(p, b"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 ");
            num(p, -h);
            byte(p, b' ');
            num(p, w);
            byte(p, b' ');
            num(p, h + d);
            put(p, b"\" width=\"");
            em(p, w);
            put(p, b"\" height=\"");
            em(p, h + d);
            put(p, b"\" fill=\"currentColor\" style=\"vertical-align:");
            em(p, -d);
            put(p, b";overflow:visible\">");
            if banded {
                byte(p, b'\n');
            }
            if !self.art.is_empty() {
                put(p, &self.art);
                self.art.clear();
            }
            if banded {
                let mut i = 0;
                while i < n {
                    let first = self.items[self.order[i] as usize];
                    let band = first.y / LINE_BAND;
                    let g = self.groups[first.g as usize];
                    self.open_path(p, g);
                    while i < n {
                        let it = *self.items.get_unchecked(*self.order.get_unchecked(i) as usize);
                        if it.g != first.g || it.y / LINE_BAND != band {
                            break;
                        }
                        self.item(p, &it, g.scale);
                        i += 1;
                    }
                    put(p, b"\"/>\n");
                }
            } else if single {
                if n > 0 {
                    let g = self.groups[self.items[0].g as usize];
                    self.open_path(p, g);
                    for i in 0..n {
                        let it = *self.items.get_unchecked(i);
                        self.item(p, &it, g.scale);
                    }
                    put(p, b"\"/>");
                }
            } else {
                let mut i = 0;
                while i < n {
                    let gi = self.items[self.order[i] as usize].g;
                    let g = self.groups[gi as usize];
                    self.open_path(p, g);
                    while i < n {
                        let it = *self.items.get_unchecked(*self.order.get_unchecked(i) as usize);
                        if it.g != gi {
                            break;
                        }
                        self.item(p, &it, g.scale);
                        i += 1;
                    }
                    put(p, b"\"/>");
                }
            }
            put(p, b"</svg>");
            let len = q.offset_from(base) as usize;
            self.out.set_len(len);
        }
    }

    pub(crate) fn text_paths(&self, items: &[Item], out: &mut Vec<u8>) {
        let mut cap = 64;
        for it in items {
            cap += 96
                + if it.kind == IT_GLYPH {
                    self.glyph_path(it.a as usize).len() + 40
                } else if it.kind == IT_IMAGE {
                    self.laid.images[it.a as usize].url.len() + 200
                } else {
                    512
                };
        }
        out.reserve(cap);
        unsafe {
            let base = out.as_mut_ptr().add(out.len());
            let mut q = base;
            let p = &mut q;
            let mut i = 0;
            while i < items.len() {
                let gi = items[i].g;
                let g = self.groups[gi as usize];
                self.open_path(p, g);
                while i < items.len() && items[i].g == gi {
                    self.item(p, &items[i], g.scale);
                    i += 1;
                }
                put(p, b"\"/>");
            }
            let added = q.offset_from(base) as usize;
            out.set_len(out.len() + added);
        }
    }

    #[inline(always)]
    unsafe fn open_path(&self, p: &mut *mut u8, g: Grp) {
        unsafe {
            put(p, b"<path");
            if g.color != 0 {
                let (o, l) = self.colors[g.color as usize];
                put(p, b" fill=\"");
                put(p, &self.color_bytes[o as usize..(o + l) as usize]);
                byte(p, b'"');
            }
            if g.scale != UNIT {
                put(p, b" transform=\"scale(");
                milli(p, g.scale as i32);
                put(p, b")\"");
            }
            put(p, b" d=\"");
        }
    }

    #[inline(always)]
    unsafe fn item(&self, p: &mut *mut u8, it: &Item, scale: u16) {
        unsafe {
            match it.kind {
                IT_GLYPH => {
                    let g = self.glyph_metric(it.a as usize);
                    let (mut x, mut y) = (it.x, it.y);
                    if scale != UNIT {
                        let inv = (1000i64 << 16) / scale as i64;
                        x = ((x as i64 * inv) >> 16) as i32;
                        y = ((y as i64 * inv) >> 16) as i32;
                    }
                    byte(p, b'M');
                    pair(p, x + g.sx as i32, y + g.sy as i32);
                    let path = self.glyph_path(it.a as usize);
                    put(p, path);
                }
                IT_IMAGE => {
                    let image = &self.laid.images[it.a as usize];
                    put(p, b"\"/>\n<image x=\"");
                    num(p, it.x);
                    put(p, b"\" y=\"");
                    num(p, it.y - image.size.1);
                    put(p, b"\" width=\"");
                    num(p, image.size.0);
                    put(p, b"\" height=\"");
                    num(p, image.size.1);
                    put(p, b"\" href=\"");
                    put(p, &image.url);
                    put(p, b"\" preserveAspectRatio=\"none\"/>\n");
                    self.open_path(p, self.groups[it.g as usize]);
                }
                IT_RULE => {
                    byte(p, b'M');
                    pair(p, it.x, it.y);
                    byte(p, b'h');
                    num(p, it.a);
                    byte(p, b'v');
                    num(p, it.b);
                    byte(p, b'h');
                    num(p, -it.a);
                    byte(p, b'z');
                }
                IT_SLASH => {
                    byte(p, b'M');
                    pair(p, it.x, it.y + it.b);
                    byte(p, b'l');
                    pair(p, it.a, -it.b);
                    put(p, b"h50l");
                    pair(p, -it.a, it.b);
                    byte(p, b'z');
                }
                IT_BACKSLASH => {
                    byte(p, b'M');
                    pair(p, it.x, it.y);
                    put(p, b"h50l");
                    pair(p, it.a, it.b);
                    put(p, b"h-50z");
                }
                _ => brace(p, it),
            }
        }
    }
}
