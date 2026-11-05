//! Small, explicit drawing frontends. Unsupported statements retain warnings;
//! these are deliberately not emulations of the packages' macro languages.
use super::*;
use alloc::{format, string::String};

struct Cursor<'a> {
    text: &'a [u8],
    at: usize,
}
impl<'a> Cursor<'a> {
    fn ws(&mut self) {
        while self.at < self.text.len() && self.text[self.at].is_ascii_whitespace() {
            self.at += 1;
        }
    }
    fn group(&mut self, open: u8, close: u8) -> Option<&'a [u8]> {
        self.ws();
        if self.text.get(self.at) != Some(&open) {
            return None;
        }
        self.at += 1;
        let start = self.at;
        let mut depth = 1;
        while self.at < self.text.len() {
            let c = self.text[self.at];
            self.at += 1;
            if c == b'\\' {
                self.at = (self.at + 1).min(self.text.len());
                continue;
            }
            if c == open {
                depth += 1;
            }
            if c == close {
                depth -= 1;
                if depth == 0 {
                    return Some(&self.text[start..self.at - 1]);
                }
            }
        }
        None
    }
    fn pair(&mut self) -> Option<(f64, f64)> {
        let t = self.group(b'(', b')')?;
        let at = t.iter().position(|c| *c == b',')?;
        Some((number(&t[..at])?, number(&t[at + 1..])?))
    }
    fn name(&mut self) -> &'a [u8] {
        let start = self.at;
        while self.at < self.text.len() && self.text[self.at].is_ascii_alphabetic() {
            self.at += 1;
        }
        &self.text[start..self.at]
    }
}
fn number(t: &[u8]) -> Option<f64> {
    figure::evaluate(t, 0.0, 0.0).map(|n| n.0)
}
fn text(t: &[u8]) -> &str {
    core::str::from_utf8(t).unwrap_or("")
}
fn point(p: (f64, f64)) -> String {
    format!("({}pt,{}pt)", p.0, p.1)
}
fn segment(out: &mut String, a: (f64, f64), b: (f64, f64), style: &str) {
    out.push_str(&format!("\\draw[{style}] {} -- {};", point(a), point(b)));
}
fn label(out: &mut String, p: (f64, f64), t: &[u8]) {
    out.push_str(&format!("\\node at {} {{{}}};", point(p), text(t)));
}

fn picture_content(t: &[u8], origin: (f64, f64), unit: f64, out: &mut String, depth: usize) -> bool {
    if depth > 32 {
        return false;
    }
    let mut c = Cursor { text: t, at: 0 };
    let mut clean = true;
    while c.at < t.len() {
        c.ws();
        if c.at == t.len() {
            break;
        }
        if t[c.at] != b'\\' {
            let start = c.at;
            while c.at < t.len() && t[c.at] != b'\\' {
                c.at += 1;
            }
            label(out, origin, &t[start..c.at]);
            continue;
        }
        c.at += 1;
        let name = c.name();
        match name {
            b"put" => {
                if let (Some((x, y)), Some(body)) = (c.pair(), c.group(b'{', b'}')) {
                    clean &= picture_content(body, (origin.0 + x * unit, origin.1 + y * unit), unit, out, depth + 1);
                } else {
                    return false;
                }
            }
            b"line" | b"vector" => {
                if let (Some((dx, dy)), Some(len)) = (c.pair(), c.group(b'{', b'}').and_then(number)) {
                    let (dx, dy) = if dx == 0.0 { (0.0, len * dy.signum()) } else { (len * dx.signum(), len * dy / dx.abs()) };
                    segment(out, origin, (origin.0 + dx * unit, origin.1 + dy * unit), if name == b"vector" { "->" } else { "" });
                } else {
                    return false;
                }
            }
            b"circle" => {
                let fill = if t.get(c.at) == Some(&b'*') {
                    c.at += 1;
                    true
                } else {
                    false
                };
                if let Some(d) = c.group(b'{', b'}').and_then(number) {
                    out.push_str(&format!(
                        "\\{} {} circle[radius={}pt];",
                        if fill { "fill" } else { "draw" },
                        point(origin),
                        d * unit / 2.0
                    ));
                } else {
                    return false;
                }
            }
            b"qbezier" => {
                c.group(b'[', b']');
                if let (Some(a), Some(b), Some(z)) = (c.pair(), c.pair(), c.pair()) {
                    let p = |v: (f64, f64)| (origin.0 + v.0 * unit, origin.1 + v.1 * unit);
                    let a = p(a);
                    let b = p(b);
                    let z = p(z);
                    out.push_str(&format!(
                        "\\draw {} .. controls {} and {} .. {};",
                        point(a),
                        point((a.0 + 2.0 * (b.0 - a.0) / 3.0, a.1 + 2.0 * (b.1 - a.1) / 3.0)),
                        point((z.0 + 2.0 * (b.0 - z.0) / 3.0, z.1 + 2.0 * (b.1 - z.1) / 3.0)),
                        point(z)
                    ));
                } else {
                    return false;
                }
            }
            b"makebox" | b"framebox" => {
                c.pair();
                c.group(b'[', b']');
                if let Some(body) = c.group(b'{', b'}') {
                    label(out, origin, body);
                } else {
                    return false;
                }
            }
            b"linethickness" => {
                c.group(b'{', b'}');
            }
            b"thinlines" | b"thicklines" => {}
            b"psline" | b"pspolygon" | b"psframe" => {
                if c.group(b'[', b']').is_some_and(|v| !v.is_empty()) {
                    clean = false;
                }
                let mut pairs = Vec::new();
                while let Some(p) = c.pair() {
                    pairs.push((p.0 * unit, p.1 * unit));
                }
                if pairs.len() < 2 {
                    return false;
                }
                if name == b"psframe" {
                    out.push_str(&format!("\\draw {} rectangle {};", point(pairs[0]), point(pairs[1])));
                } else {
                    for p in pairs.windows(2) {
                        segment(out, p[0], p[1], "");
                    }
                    if name == b"pspolygon" {
                        segment(out, *pairs.last().unwrap(), pairs[0], "");
                    }
                }
            }
            b"pscircle" => {
                if c.group(b'[', b']').is_some_and(|v| !v.is_empty()) {
                    clean = false;
                }
                if let (Some(p), Some(r)) = (c.pair(), c.group(b'{', b'}').and_then(number)) {
                    out.push_str(&format!("\\draw {} circle[radius={}pt];", point((p.0 * unit, p.1 * unit)), r * unit));
                } else {
                    return false;
                }
            }
            b"rput" => {
                c.group(b'[', b']');
                if let (Some(p), Some(body)) = (c.pair(), c.group(b'{', b'}')) {
                    label(out, (p.0 * unit, p.1 * unit), body);
                } else {
                    return false;
                }
            }
            _ => {
                clean = false;
                c.group(b'[', b']');
                c.group(b'{', b'}');
            }
        }
    }
    clean
}

fn tree(c: &mut Cursor<'_>, depth: usize, next: &mut f64, out: &mut String) -> Option<(f64, f64)> {
    if depth > 64 {
        return None;
    }
    c.ws();
    if c.text.get(c.at) != Some(&b'[') {
        return None;
    }
    c.at += 1;
    let start = c.at;
    let mut braces = 0;
    while c.at < c.text.len() {
        let ch = c.text[c.at];
        if braces == 0 && matches!(ch, b'[' | b']' | b',') {
            break;
        }
        if ch == b'{' {
            braces += 1;
        }
        if ch == b'}' {
            braces -= 1;
        }
        c.at += 1;
    }
    let title = &c.text[start..c.at];
    if c.text.get(c.at) == Some(&b',') {
        while c.at < c.text.len() && !matches!(c.text[c.at], b'[' | b']') {
            if c.text[c.at] == b'{' {
                c.group(b'{', b'}')?;
            } else {
                c.at += 1;
            }
        }
    }
    let mut children = Vec::new();
    c.ws();
    while c.text.get(c.at) == Some(&b'[') {
        children.push(tree(c, depth + 1, next, out)?);
        c.ws();
    }
    if c.text.get(c.at) != Some(&b']') {
        return None;
    }
    c.at += 1;
    let x = if children.is_empty() {
        let x = *next;
        *next += 55.0;
        x
    } else {
        (children.first()?.0 + children.last()?.0) / 2.0
    };
    let p = (x, -(depth as f64) * 45.0);
    for child in children {
        let dx = child.0 - p.0;
        let dy = child.1 - p.1;
        let length = figure::sqrt(dx * dx + dy * dy).max(1.0);
        let gap = 8.0 / length;
        segment(out, (p.0 + dx * gap, p.1 + dy * gap), (child.0 - dx * gap, child.1 - dy * gap), "");
    }
    label(out, p, title.trim_ascii());
    Some(p)
}

fn molecule(c: &mut Cursor<'_>, mut p: (f64, f64), mut angle: f64, out: &mut String, depth: usize) -> bool {
    if depth > 32 {
        return false;
    }
    let mut clean = true;
    while c.at < c.text.len() {
        c.ws();
        let Some(&ch) = c.text.get(c.at) else {
            break;
        };
        match ch {
            b')' => {
                c.at += 1;
                return clean;
            }
            b'(' => {
                c.at += 1;
                clean &= molecule(c, p, angle, out, depth + 1);
            }
            b'-' | b'=' | b'~' | b'>' | b'<' => {
                c.at += 1;
                if let Some(opts) = c.group(b'[', b']') {
                    let first = opts.split(|c| *c == b',').next().unwrap_or(opts).trim_ascii();
                    if let Some(v) = first.strip_prefix(b"::").and_then(number) {
                        angle += v;
                    } else if let Some(v) = first.strip_prefix(b":").and_then(number) {
                        angle = v;
                    } else if let Some(v) = number(first) {
                        angle = v * 45.0;
                    } else {
                        clean = false;
                    }
                }
                let q = (p.0 + 35.0 * figure::cos_deg(angle), p.1 + 35.0 * figure::sin_deg(angle));
                let count = if ch == b'=' {
                    2
                } else if ch == b'~' {
                    3
                } else {
                    1
                };
                let dx = figure::sin_deg(angle) * 2.0;
                let dy = -figure::cos_deg(angle) * 2.0;
                for i in 0..count {
                    let off = i as f64 - (count - 1) as f64 / 2.0;
                    let trim = (figure::cos_deg(angle) * 9.0, figure::sin_deg(angle) * 9.0);
                    segment(
                        out,
                        (p.0 + trim.0 + dx * off, p.1 + trim.1 + dy * off),
                        (q.0 - trim.0 + dx * off, q.1 - trim.1 + dy * off),
                        if ch == b'>' { "->" } else { "" },
                    );
                }
                p = q;
            }
            b'*' => {
                clean = false;
                c.at += 1;
            }
            _ => {
                let start = c.at;
                while c.at < c.text.len() && !matches!(c.text[c.at], b'-' | b'=' | b'~' | b'>' | b'<' | b'(' | b')' | b'*') {
                    c.at += 1;
                }
                let atom = text(&c.text[start..c.at]);
                if !atom.trim().is_empty() {
                    label(out, p, format!("$\\mathrm{{{atom}}}$").as_bytes());
                }
            }
        }
    }
    clean
}

impl Engine {
    fn translated_picture(&mut self, d: &mut Doc, left: i32, width: i32, mut source: String) -> R<()> {
        source.push_str("\\end{tikzpicture}");
        let kept = self.source_copy(source.as_bytes());
        let src = self.push_src(kept.as_ptr(), kept.len());
        let done = self.picture(d, left, width);
        self.pop_src(src);
        done
    }
    pub(super) fn legacy_picture(&mut self, d: &mut Doc, left: i32, width: i32, name: &[u8]) -> R<()> {
        let start = self.pos;
        self.skip_environment(name);
        let end = self.pos;
        let closing = format!("\\end{{{}}}", text(name));
        let body = self.slice(start, end);
        let Some(body) = body.strip_suffix(closing.as_bytes()) else {
            return self.fail(E_END);
        };
        let mut out = String::new();
        let clean = if name == b"forest" {
            let mut c = Cursor { text: body, at: 0 };
            while c.at < body.len() && body[c.at] != b'[' {
                if body[c.at] == b'{' {
                    c.group(b'{', b'}');
                } else {
                    c.at += 1;
                }
            }
            tree(&mut c, 0, &mut 0.0, &mut out).is_some()
        } else {
            let mut c = Cursor { text: body, at: 0 };
            c.pair();
            if name == b"pspicture" {
                c.pair();
            }
            picture_content(&body[c.at..], (0.0, 0.0), if name == b"picture" { d.picture_unit } else { figure::CM }, &mut out, 0)
        };
        if !clean {
            self.note(start);
        }
        self.translated_picture(d, left, width, out)
    }
    pub(super) fn chemfig(&mut self, d: &mut Doc, left: i32, width: i32) -> R<()> {
        let start = self.pos;
        self.skip_options();
        let body = self.group_text()?;
        let mut out = String::new();
        let mut cursor = Cursor { text: body, at: 0 };
        if !molecule(&mut cursor, (0.0, 0.0), 0.0, &mut out, 0) {
            self.note(start);
        }
        self.translated_picture(d, left, width, out)
    }
}
