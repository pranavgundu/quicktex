use super::plot::{AxisMap, Plot};
use super::*;
#[cfg(not(test))]
use core::arch::wasm32::{f64x2_extract_lane, f64x2_splat, f64x2_sqrt};
use core::cell::Cell;

pub(super) const CM: f64 = 28.452756;
const PI: f64 = core::f64::consts::PI;
const LN2: f64 = core::f64::consts::LN_2;
const NODE_SEP: f64 = 3.3333;
const TO_REACH: f64 = 0.3915;
const ARC_HANDLE: f64 = 0.5523;
const MAX_STYLE_DEPTH: u32 = 8;
const MAX_CALC_DEPTH: u32 = 32;
const MAX_SWEEP: f64 = 1440.0;
const OPTION_BUDGET: u32 = 20000;
const MAX_UNITS: f64 = 1e8;

fn ceil_steps(v: f64, step: f64) -> f64 {
    let q = v / step;
    let whole = q as i64 as f64;
    if q > whole + 1e-9 { whole + 1.0 } else { whole }
}

fn to_units(v: f64) -> i32 {
    v.clamp(-MAX_UNITS, MAX_UNITS) as i32
}

#[cfg(not(test))]
pub(super) fn sqrt(v: f64) -> f64 {
    f64x2_extract_lane::<0>(f64x2_sqrt(f64x2_splat(v)))
}

#[cfg(test)]
pub(super) fn sqrt(v: f64) -> f64 {
    v.sqrt()
}

pub(super) fn floor(v: f64) -> f64 {
    let t = v as i64 as f64;
    if t > v { t - 1.0 } else { t }
}

fn sine(x: f64) -> f64 {
    let turns = floor(x / (2.0 * PI) + 0.5);
    let mut r = x - turns * 2.0 * PI;
    if r > PI / 2.0 {
        r = PI - r;
    } else if r < -PI / 2.0 {
        r = -PI - r;
    }
    let s = r * r;
    r * (1.0 - s / 6.0 * (1.0 - s / 20.0 * (1.0 - s / 42.0 * (1.0 - s / 72.0 * (1.0 - s / 110.0 * (1.0 - s / 156.0))))))
}

pub(super) fn sin_deg(a: f64) -> f64 {
    sine(a * PI / 180.0)
}

pub(super) fn cos_deg(a: f64) -> f64 {
    sine((a + 90.0) * PI / 180.0)
}

fn exp(x: f64) -> f64 {
    if x > 700.0 {
        return f64::INFINITY;
    }
    if x < -700.0 {
        return 0.0;
    }
    let k = floor(x / LN2 + 0.5);
    let r = x - k * LN2;
    let mut term = 1.0;
    let mut sum = 1.0;
    let mut i = 1.0;
    while i < 16.0 {
        term *= r / i;
        sum += term;
        i += 1.0;
    }
    sum * f64::from_bits(((1023 + k as i64) as u64) << 52)
}

fn ln(x: f64) -> f64 {
    if x <= 0.0 {
        return f64::NAN;
    }
    let bits = x.to_bits();
    let mut e = ((bits >> 52) & 0x7ff) as i64 - 1023;
    let mut m = f64::from_bits((bits & 0x000f_ffff_ffff_ffff) | (1023u64 << 52));
    if m > core::f64::consts::SQRT_2 {
        m /= 2.0;
        e += 1;
    }
    let t = (m - 1.0) / (m + 1.0);
    let s = t * t;
    let mut sum = 0.0;
    let mut power = t;
    let mut i = 1.0;
    while i < 24.0 {
        sum += power / i;
        power *= s;
        i += 2.0;
    }
    2.0 * sum + e as f64 * LN2
}

fn power(a: f64, b: f64) -> f64 {
    if b == 0.0 {
        return 1.0;
    }
    if a == 0.0 {
        return 0.0;
    }
    if a < 0.0 {
        let whole = floor(b) == b;
        let v = exp(b * ln(-a));
        return if !whole {
            f64::NAN
        } else if (b as i64) & 1 != 0 {
            -v
        } else {
            v
        };
    }
    exp(b * ln(a))
}

fn truncate(v: f64) -> f64 {
    if v < 0.0 { -floor(-v) } else { floor(v) }
}

fn arctan(x: f64) -> f64 {
    if x < 0.0 {
        return -arctan(-x);
    }
    if x > 1.0 {
        return PI / 2.0 - arctan(1.0 / x);
    }
    if x > 0.4142135623730951 {
        return PI / 4.0 + arctan((x - 1.0) / (x + 1.0));
    }
    let (square, mut term, mut sum) = (x * x, x, x);
    for n in 1..16 {
        term *= -square;
        sum += term / (2 * n + 1) as f64;
    }
    sum
}

fn angle_of(y: f64, x: f64) -> f64 {
    let turn = if x > 0.0 {
        arctan(y / x)
    } else if x < 0.0 {
        arctan(y / x) + if y < 0.0 { -PI } else { PI }
    } else if y < 0.0 {
        -PI / 2.0
    } else if y > 0.0 {
        PI / 2.0
    } else {
        0.0
    };
    turn * 180.0 / PI
}

fn abs(v: f64) -> f64 {
    if v < 0.0 { -v } else { v }
}

struct Calc<'a> {
    s: &'a [u8],
    i: usize,
    x: f64,
    line: f64,
    unit: bool,
    depth: u32,
}

impl Calc<'_> {
    fn peek(&self) -> u8 {
        if self.i < self.s.len() { self.s[self.i] } else { 0 }
    }

    fn gap(&mut self) {
        while matches!(self.peek(), b' ' | b'\n' | b'\t' | b'\r') {
            self.i += 1;
        }
    }

    fn sum(&mut self) -> Option<f64> {
        self.depth += 1;
        if self.depth > MAX_CALC_DEPTH {
            return None;
        }
        let mut v = self.term()?;
        loop {
            self.gap();
            match self.peek() {
                b'+' => {
                    self.i += 1;
                    v += self.term()?;
                }
                b'-' => {
                    self.i += 1;
                    v -= self.term()?;
                }
                _ => {
                    self.depth -= 1;
                    return Some(v);
                }
            }
        }
    }

    fn term(&mut self) -> Option<f64> {
        let mut v = self.unary()?;
        loop {
            self.gap();
            match self.peek() {
                b'*' => {
                    self.i += 1;
                    v *= self.unary()?;
                }
                b'/' => {
                    self.i += 1;
                    v /= self.unary()?;
                }
                _ => return Some(v),
            }
        }
    }

    fn unary(&mut self) -> Option<f64> {
        self.gap();
        match self.peek() {
            b'-' => {
                self.i += 1;
                self.depth += 1;
                if self.depth > MAX_CALC_DEPTH {
                    return None;
                }
                let v = self.unary()?;
                self.depth -= 1;
                Some(-v)
            }
            b'+' => {
                self.i += 1;
                self.unary()
            }
            _ => {
                let v = self.atom()?;
                self.gap();
                if self.peek() == b'^' {
                    self.i += 1;
                    let e = self.unary()?;
                    return Some(power(v, e));
                }
                Some(v)
            }
        }
    }

    fn word(&mut self) -> &[u8] {
        let s = self.i;
        while self.peek().is_ascii_alphabetic() {
            self.i += 1;
        }
        &self.s[s..self.i]
    }

    fn number(&mut self) -> Option<f64> {
        let mut v = 0.0;
        let mut seen = false;
        while self.peek().is_ascii_digit() {
            v = v * 10.0 + (self.peek() - b'0') as f64;
            self.i += 1;
            seen = true;
        }
        if self.peek() == b'.' {
            self.i += 1;
            let mut place = 0.1;
            while self.peek().is_ascii_digit() {
                v += (self.peek() - b'0') as f64 * place;
                place /= 10.0;
                self.i += 1;
                seen = true;
            }
        }
        if !seen {
            return None;
        }
        if matches!(self.peek(), b'e' | b'E') {
            let mut j = self.i + 1;
            let negative = j < self.s.len() && self.s[j] == b'-';
            if j < self.s.len() && matches!(self.s[j], b'-' | b'+') {
                j += 1;
            }
            if j < self.s.len() && self.s[j].is_ascii_digit() {
                let mut e = 0;
                while j < self.s.len() && self.s[j].is_ascii_digit() {
                    e = (e * 10 + (self.s[j] - b'0') as i32).min(300);
                    j += 1;
                }
                self.i = j;
                v *= power(10.0, if negative { -e as f64 } else { e as f64 });
            }
        }
        Some(v)
    }

    fn atom(&mut self) -> Option<f64> {
        self.gap();
        let ch = self.peek();
        if ch == b'(' || ch == b'{' {
            let close = if ch == b'(' { b')' } else { b'}' };
            self.i += 1;
            let v = self.sum()?;
            self.gap();
            if self.peek() != close {
                return None;
            }
            self.i += 1;
            return Some(v);
        }
        if ch == b'\\' {
            self.i += 1;
            return match self.word() {
                b"linewidth" | b"textwidth" | b"columnwidth" => {
                    self.unit = true;
                    Some(self.line)
                }
                b"pi" => Some(PI),
                _ => None,
            };
        }
        if ch.is_ascii_digit() || ch == b'.' {
            let v = self.number()?;
            let at = self.i;
            self.gap();
            if self.peek() == b'\\' {
                return Some(v * self.atom()?);
            }
            let scale = match self.word() {
                b"pt" => 1.0,
                b"bp" => 1.00375,
                b"cm" => CM,
                b"mm" => CM / 10.0,
                b"in" => 72.27,
                b"em" => 10.0,
                b"ex" => 4.3,
                b"x" => return Some(v * self.x),
                _ => {
                    self.i = at;
                    return Some(v);
                }
            };
            self.unit = true;
            return Some(v * scale);
        }
        if !ch.is_ascii_alphabetic() {
            return None;
        }
        let word = {
            let s = self.i;
            self.word();
            s..self.i
        };
        let mut word = word;
        if self.peek() == b'2' && &self.s[word.clone()] == b"atan" {
            self.i += 1;
            word.end = self.i;
        }
        let name = &self.s[word];
        if self.peek() != b'(' {
            return match name {
                b"x" => Some(self.x),
                b"pi" => Some(PI),
                b"e" => Some(core::f64::consts::E),
                _ => None,
            };
        }
        self.i += 1;
        let a = self.sum()?;
        self.gap();
        let b = if self.peek() == b',' {
            self.i += 1;
            self.sum()?
        } else {
            0.0
        };
        self.gap();
        if self.peek() != b')' {
            return None;
        }
        self.i += 1;
        Some(match name {
            b"sin" => sin_deg(a),
            b"cos" => cos_deg(a),
            b"tan" => sin_deg(a) / cos_deg(a),
            b"exp" => exp(a),
            b"ln" => ln(a),
            b"log" => ln(a) / core::f64::consts::LN_10,
            b"sqrt" => sqrt(a),
            b"abs" => abs(a),
            b"deg" => a * 180.0 / PI,
            b"rad" => a * PI / 180.0,
            b"floor" => floor(a),
            b"ceil" => -floor(-a),
            b"round" => floor(a + 0.5),
            b"min" => a.min(b),
            b"max" => a.max(b),
            b"pow" => power(a, b),
            b"mod" => a - b * truncate(a / b),
            b"Mod" => a - b * floor(a / b),
            b"int" => truncate(a),
            b"frac" => a - truncate(a),
            b"neg" => -a,
            b"sign" => {
                if a > 0.0 {
                    1.0
                } else if a < 0.0 {
                    -1.0
                } else {
                    0.0
                }
            }
            b"atan" => arctan(a) * 180.0 / PI,
            b"atan2" => angle_of(a, b),
            b"asin" => angle_of(a, sqrt(1.0 - a * a)),
            b"acos" => angle_of(sqrt(1.0 - a * a), a),
            b"sec" => 1.0 / cos_deg(a),
            b"cosec" => 1.0 / sin_deg(a),
            b"cot" => cos_deg(a) / sin_deg(a),
            b"veclen" => sqrt(a * a + b * b),
            _ => return None,
        })
    }
}

pub(super) fn evaluate(s: &[u8], x: f64, line: f64) -> Option<(f64, bool)> {
    let mut calc = Calc { s, i: 0, x, line, unit: false, depth: 0 };
    let v = calc.sum()?;
    calc.gap();
    if calc.i != s.len() {
        return None;
    }
    Some((v, calc.unit))
}

pub(super) fn blank_label(options: &'static [u8]) -> bool {
    let mut at = 0;
    while let Some((key, value)) = next_option(options, &mut at) {
        if key == b"label" {
            return matches!(trim(value), b"" | b"{}");
        }
    }
    false
}

pub(super) fn length_pt(s: &[u8]) -> Option<f64> {
    evaluate(trim(s), 0.0, 0.0).map(|v| v.0)
}

pub(super) fn trim(s: &[u8]) -> &[u8] {
    let mut a = 0;
    let mut b = s.len();
    while a < b && matches!(s[a], b' ' | b'\n' | b'\t' | b'\r') {
        a += 1;
    }
    while b > a && matches!(s[b - 1], b' ' | b'\n' | b'\t' | b'\r') {
        b -= 1;
    }
    &s[a..b]
}

pub(super) fn unbrace(s: &'static [u8]) -> &'static [u8] {
    let t = trim(s);
    if t.len() >= 2 && t[0] == b'{' && t[t.len() - 1] == b'}' { trim(&t[1..t.len() - 1]) } else { t }
}

pub(super) fn next_option(text: &'static [u8], at: &mut usize) -> Option<(&'static [u8], &'static [u8])> {
    loop {
        if *at >= text.len() {
            return None;
        }
        while *at < text.len() && text[*at].is_ascii_whitespace() {
            *at += 1;
        }
        if text.get(*at) == Some(&b'%') {
            while *at < text.len() && text[*at] != b'\n' {
                *at += 1;
            }
            continue;
        }
        let start = *at;
        let mut depth = 0;
        let mut equals = None;
        while *at < text.len() {
            match text[*at] {
                b'{' | b'[' | b'(' => depth += 1,
                b'}' | b']' | b')' => depth -= 1,
                b'=' if depth == 0 && equals.is_none() => equals = Some(*at),
                b',' if depth == 0 => break,
                _ => {}
            }
            *at += 1;
        }
        let end = *at;
        *at += 1;
        let (key, value) = match equals {
            Some(e) => (trim(&text[start..e]), unbrace(&text[e + 1..end])),
            None => (trim(&text[start..end]), &text[0..0]),
        };
        if !key.is_empty() {
            return Some((key, value));
        }
    }
}

fn length_of(v: (f64, f64)) -> f64 {
    sqrt(v.0 * v.0 + v.1 * v.1)
}

fn round_corners(path: &mut Vec<(u8, f64, f64)>, radius: f64) {
    if path.iter().any(|seg| seg.0 == SEG_CURVE) {
        return;
    }
    let old = core::mem::take(path);
    let mut start = 0;
    while start < old.len() {
        let mut end = start + 1;
        while end < old.len() && old[end].0 != SEG_MOVE {
            end += 1;
        }
        let closed = old[end - 1].0 == SEG_CLOSE;
        let points: Vec<(f64, f64)> = old[start..end - closed as usize].iter().map(|seg| (seg.1, seg.2)).collect();
        round_subpath(path, &points, closed, radius);
        start = end;
    }
}

fn round_subpath(path: &mut Vec<(u8, f64, f64)>, points: &[(f64, f64)], closed: bool, radius: f64) {
    let n = points.len();
    if n < 3 {
        for (i, p) in points.iter().enumerate() {
            path.push((if i == 0 { SEG_MOVE } else { SEG_LINE }, p.0, p.1));
        }
        return;
    }
    let toward = |from: (f64, f64), to: (f64, f64)| {
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        let length = length_of((dx, dy));
        let step = radius.min(length / 2.0) / length.max(1e-9);
        (from.0 + dx * step, from.1 + dy * step)
    };
    let corner = |path: &mut Vec<(u8, f64, f64)>, at: (f64, f64), enter: (f64, f64), leave: (f64, f64)| {
        path.push((SEG_LINE, enter.0, enter.1));
        path.push((SEG_CURVE, enter.0 + (at.0 - enter.0) * ARC_HANDLE, enter.1 + (at.1 - enter.1) * ARC_HANDLE));
        path.push((SEG_CURVE, leave.0 + (at.0 - leave.0) * ARC_HANDLE, leave.1 + (at.1 - leave.1) * ARC_HANDLE));
        path.push((SEG_CURVE, leave.0, leave.1));
    };
    if !closed {
        path.push((SEG_MOVE, points[0].0, points[0].1));
        for i in 1..n - 1 {
            corner(path, points[i], toward(points[i], points[i - 1]), toward(points[i], points[i + 1]));
        }
        path.push((SEG_LINE, points[n - 1].0, points[n - 1].1));
        return;
    }
    let begin = toward(points[0], points[1]);
    path.push((SEG_MOVE, begin.0, begin.1));
    for i in 1..=n {
        let at = points[i % n];
        corner(path, at, toward(at, points[i - 1]), toward(at, points[(i + 1) % n]));
    }
    path.push((SEG_CLOSE, begin.0, begin.1));
}

#[derive(Clone, Copy)]
pub(super) struct Mat {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
}

impl Mat {
    const IDENTITY: Mat = Mat { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 };

    fn at(&self, p: (f64, f64)) -> (f64, f64) {
        (self.a * p.0 + self.c * p.1 + self.e, self.b * p.0 + self.d * p.1 + self.f)
    }

    fn vector(&self, p: (f64, f64)) -> (f64, f64) {
        (self.a * p.0 + self.c * p.1, self.b * p.0 + self.d * p.1)
    }

    fn undo(&self, p: (f64, f64)) -> (f64, f64) {
        let det = self.a * self.d - self.b * self.c;
        if det == 0.0 {
            return (0.0, 0.0);
        }
        ((self.d * p.0 - self.c * p.1) / det, (self.a * p.1 - self.b * p.0) / det)
    }

    fn then(&self, t: &Mat) -> Mat {
        Mat {
            a: self.a * t.a + self.c * t.b,
            b: self.b * t.a + self.d * t.b,
            c: self.a * t.c + self.c * t.d,
            d: self.b * t.c + self.d * t.d,
            e: self.a * t.e + self.c * t.f + self.e,
            f: self.b * t.e + self.d * t.f + self.f,
        }
    }

    fn shifted(&self, x: f64, y: f64) -> Mat {
        self.then(&Mat { e: x, f: y, ..Mat::IDENTITY })
    }

    fn scaled(&self, x: f64, y: f64) -> Mat {
        self.then(&Mat { a: x, d: y, ..Mat::IDENTITY })
    }

    fn rotated(&self, angle: f64) -> Mat {
        let (s, c) = (sin_deg(angle), cos_deg(angle));
        self.then(&Mat { a: c, b: s, c: -s, d: c, e: 0.0, f: 0.0 })
    }
}

const ALIGN_NONE: u8 = 0;
const ALIGN_LEFT: u8 = 1;
const ALIGN_CENTER: u8 = 2;
const ALIGN_RIGHT: u8 = 3;
const ALIGN_FLUSH: u8 = 4;

const TIP_NONE: u8 = 0;
const TIP_DEFAULT: u8 = 1;
const TIP_OPEN: u8 = 2;
const TIP_STEALTH: u8 = 3;

#[derive(Clone, Copy)]
pub(super) struct Tip {
    kind: u8,
    length: f64,
}

#[derive(Clone, Copy)]
pub(super) struct Pen {
    pub(super) width: f64,
    pub(super) stroke: u32,
    pub(super) fill: u32,
    pub(super) dash: (f64, f64),
    pub(super) round: bool,
    pub(super) drawn: bool,
    pub(super) solid: bool,
    pub(super) start: Tip,
    pub(super) end: Tip,
    pub(super) arrow: Tip,
    pub(super) corner: f64,
}

#[derive(Clone, Copy)]
pub(super) struct NodeLook {
    anchor: (i8, i8),
    gap: f64,
    sep: f64,
    turn: f64,
    size: u16,
    border: bool,
    color: u32,
    round: bool,
    place: f64,
    least: (f64, f64),
    wrap: f64,
    align: u8,
    font: u8,
}

#[derive(Clone, Copy)]
pub(super) struct Look {
    pub(super) axes: [(f64, f64); 3],
    pub(super) step: (f64, f64),
    pub(super) m: Mat,
    pub(super) pen: Pen,
    pub(super) node: NodeLook,
}

#[derive(Clone, Copy, Default)]
pub(super) struct Extent {
    w: f64,
    h: f64,
    round: bool,
}

#[derive(Clone, Copy)]
struct Named {
    name: &'static [u8],
    x: f64,
    y: f64,
    extent: Extent,
}

fn border(from: (f64, f64), e: Extent, toward: (f64, f64)) -> (f64, f64) {
    let (dx, dy) = (toward.0 - from.0, toward.1 - from.1);
    let size = sqrt(dx * dx + dy * dy);
    if e.w <= 0.0 || size == 0.0 {
        return from;
    }
    let reach = if e.round {
        e.w / size
    } else {
        let across = if dx == 0.0 { f64::INFINITY } else { e.w / abs(dx) };
        let down = if dy == 0.0 { f64::INFINITY } else { e.h / abs(dy) };
        across.min(down)
    };
    (from.0 + dx * reach, from.1 + dy * reach)
}

#[derive(Default)]
pub(in crate::tex) struct Pic {
    coords: Vec<Named>,
    extent: Cell<Extent>,
    styles: Vec<(&'static [u8], &'static [u8])>,
    every_node: &'static [u8],
    pub(super) saved: Vec<(&'static [u8], Option<Def>)>,
    pub(super) path: Vec<(u8, f64, f64)>,
    aside: Vec<(u8, f64, f64)>,
    art: Vec<Art>,
    bounds: (f64, f64, f64, f64),
    drawn: bool,
    budget: u32,
    pub(super) line: f64,
    pub(super) axis: Option<AxisMap>,
    pub(super) points: Vec<(f64, f64)>,
    pub(super) plots: Vec<Plot>,
    pub(super) entries: Vec<&'static [u8]>,
}

impl Pic {
    pub(super) fn grow(&mut self, x: f64, y: f64) {
        if !self.drawn {
            self.drawn = true;
            self.bounds = (x, y, x, y);
            return;
        }
        let b = &mut self.bounds;
        b.0 = b.0.min(x);
        b.1 = b.1.min(y);
        b.2 = b.2.max(x);
        b.3 = b.3.max(y);
    }
}

pub(super) fn color_by_name(name: &[u8]) -> Option<u32> {
    Some(match name {
        b"black" => 0x000000,
        b"white" => 0xffffff,
        b"red" => 0xff0000,
        b"green" => 0x00ff00,
        b"blue" => 0x0000ff,
        b"cyan" => 0x00ffff,
        b"magenta" => 0xff00ff,
        b"yellow" => 0xffff00,
        b"gray" | b"grey" => 0x808080,
        b"darkgray" => 0x404040,
        b"lightgray" => 0xbfbfbf,
        b"brown" => 0xbf8040,
        b"lime" => 0xbfff00,
        b"olive" => 0x808000,
        b"orange" => 0xff8000,
        b"pink" => 0xffbfbf,
        b"purple" => 0xbf0040,
        b"teal" => 0x008080,
        b"violet" => 0x800080,
        _ => return None,
    })
}

fn parse_color(text: &[u8], named: &dyn Fn(&[u8]) -> Option<u32>) -> Option<u32> {
    if text == b"none" {
        return Some(NO_COLOR);
    }
    let mut parts = text.split(|c| *c == b'!');
    let mut color = named(parts.next()?)?;
    while let Some(amount) = parts.next() {
        let (percent, _) = evaluate(amount, 0.0, 0.0)?;
        let other = match parts.next() {
            Some(name) => named(name)?,
            None => 0xffffff,
        };
        color = mix(color, other, percent.clamp(0.0, 100.0));
    }
    Some(color)
}

#[cfg(test)]
fn color_value(text: &[u8]) -> Option<u32> {
    parse_color(text, &color_by_name)
}

fn mix(a: u32, b: u32, percent: f64) -> u32 {
    let lane = |shift: u32| {
        let x = ((a >> shift) & 255) as f64;
        let y = ((b >> shift) & 255) as f64;
        ((x * percent + y * (100.0 - percent)) / 100.0 + 0.5) as u32 & 255
    };
    (lane(16) << 16) | (lane(8) << 8) | lane(0)
}

fn tip_named(text: &'static [u8]) -> Option<Tip> {
    let t = unbrace(text);
    let (name, options) = match t.iter().position(|c| *c == b'[') {
        Some(i) if t.last() == Some(&b']') => (&t[..i], &t[i + 1..t.len() - 1]),
        Some(_) => return None,
        None => (t, &t[0..0]),
    };
    let kind = match name {
        b"" => TIP_NONE,
        b">" | b"<" => TIP_DEFAULT,
        b"to" | b"To" => TIP_OPEN,
        b"Stealth" | b"stealth" | b"latex" | b"Latex" | b"LaTeX" | b"Triangle" => TIP_STEALTH,
        _ => return None,
    };
    let mut length = 0.0;
    let mut at = 0;
    while let Some((key, value)) = next_option(options, &mut at) {
        if key == b"length" {
            length = length_pt(value)?;
        }
    }
    Some(Tip { kind, length })
}

fn tips(text: &'static [u8]) -> Option<(Tip, Tip)> {
    let mut depth = 0;
    for (i, c) in text.iter().enumerate() {
        match c {
            b'{' | b'[' => depth += 1,
            b'}' | b']' => depth -= 1,
            b'-' if depth == 0 => return Some((tip_named(&text[..i])?, tip_named(&text[i + 1..])?)),
            _ => {}
        }
    }
    None
}

fn placement(key: &[u8]) -> Option<(i8, i8)> {
    Some(match key {
        b"above" => (0, -1),
        b"below" => (0, 1),
        b"left" => (1, 0),
        b"right" => (-1, 0),
        b"above left" => (1, -1),
        b"above right" => (-1, -1),
        b"below left" => (1, 1),
        b"below right" => (-1, 1),
        _ => return None,
    })
}

fn anchor_value(name: &[u8]) -> Option<(i8, i8)> {
    Some(match name {
        b"center" => (0, 0),
        b"north" => (0, 1),
        b"south" | b"base" | b"mid" => (0, -1),
        b"east" => (1, 0),
        b"west" => (-1, 0),
        b"north east" => (1, 1),
        b"north west" => (-1, 1),
        b"south east" => (1, -1),
        b"south west" => (-1, -1),
        _ => return None,
    })
}

pub(super) fn first_look() -> Look {
    let none = Tip { kind: TIP_NONE, length: 0.0 };
    Look {
        axes: [(CM, 0.0), (0.0, CM), (-0.385 * CM, -0.385 * CM)],
        step: (CM, CM),
        m: Mat::IDENTITY,
        pen: Pen {
            width: 0.4,
            stroke: 0,
            fill: NO_COLOR,
            dash: (0.0, 0.0),
            round: false,
            drawn: false,
            solid: false,
            start: none,
            end: none,
            arrow: Tip { kind: TIP_OPEN, length: 0.0 },
            corner: 0.0,
        },
        node: NodeLook {
            anchor: (0, 0),
            gap: 0.0,
            sep: NODE_SEP,
            turn: 0.0,
            size: 0,
            border: false,
            color: 0,
            round: false,
            place: 1.0,
            least: (0.0, 0.0),
            wrap: 0.0,
            align: ALIGN_LEFT,
            font: 0,
        },
    }
}

impl Engine {
    pub(super) fn fail_at<T>(&mut self, at: &[u8], code: u8) -> R<T> {
        let offset = (at.as_ptr() as usize).wrapping_sub(self.p as usize);
        if offset <= self.len {
            self.pos = offset;
        }
        self.fail(code)
    }

    pub(super) fn skip_gap(&mut self) {
        loop {
            self.skip_ws();
            if self.peek() != b'%' || self.at_end() {
                return;
            }
            while !self.at_end() && self.peek() != b'\n' {
                self.pos += 1;
            }
        }
    }

    pub(super) fn square_options(&mut self) -> R<&'static [u8]> {
        self.skip_gap();
        if self.peek() != b'[' {
            return Ok(&[]);
        }
        self.nested(b'[', b']')
    }

    fn nested(&mut self, open: u8, close: u8) -> R<&'static [u8]> {
        self.pos += 1;
        let start = self.pos;
        let mut depth = 0;
        let mut braces = 0;
        loop {
            if self.at_end() {
                return self.fail(E_END);
            }
            let ch = self.peek();
            if ch == b'\\' {
                self.pos += 2;
                continue;
            }
            if ch == close && depth == 0 && braces == 0 {
                let end = self.pos;
                self.pos += 1;
                return Ok(self.slice(start, end));
            }
            if ch == open {
                depth += 1;
            } else if ch == close {
                depth -= 1;
            } else if ch == b'{' {
                braces += 1;
            } else if ch == b'}' {
                braces -= 1;
            }
            self.pos += 1;
        }
    }

    pub(super) fn options(&mut self, pic: &mut Pic, look: &mut Look, text: &'static [u8], node: bool, depth: u32) -> R<()> {
        if depth == 0 {
            pic.budget = OPTION_BUDGET;
        }
        if depth > MAX_STYLE_DEPTH || pic.budget == 0 {
            return self.fail_at(text, E_OPTION);
        }
        let text = self.expand_text(text);
        let mut at = 0;
        while let Some((key, value)) = next_option(text, &mut at) {
            pic.budget = pic.budget.saturating_sub(1);
            let held = self.pos;
            if self.option(pic, look, key, value, node, depth).is_err() {
                if !matches!(self.err, E_OPTION | E_ARG | E_UNIT | E_COLOR) {
                    return Err(());
                }
                self.err = 0;
                self.note(held);
                self.pos = held;
            }
        }
        Ok(())
    }

    fn value(&mut self, value: &'static [u8]) -> R<f64> {
        match evaluate(value, 0.0, 0.0) {
            Some((v, _)) if !v.is_nan() => Ok(v),
            _ => self.fail_at(value, E_OPTION),
        }
    }

    fn set_color(&mut self, look: &mut Look, value: &'static [u8], node: bool) -> R<()> {
        let Some(color) = self.color_value(value) else {
            return self.fail_at(value, E_OPTION);
        };
        look.pen.stroke = color;
        if node {
            look.node.color = color;
        }
        Ok(())
    }

    pub(super) fn option(
        &mut self,
        pic: &mut Pic,
        look: &mut Look,
        key: &'static [u8],
        value: &'static [u8],
        node: bool,
        depth: u32,
    ) -> R<()> {
        if let Some(name) = key.strip_suffix(b"/.style").or_else(|| key.strip_suffix(b"/.append style")) {
            let value = self.lasting(value);
            if name == b"every node" {
                pic.every_node = value;
            } else {
                let name = self.lasting(name);
                pic.styles.push((name, value));
            }
            return Ok(());
        }
        let pen = &mut look.pen;
        match key {
            b"scale" => {
                if !node {
                    let v = self.value(value)?;
                    look.m = look.m.scaled(v, v);
                }
            }
            b"xscale" => {
                let v = self.value(value)?;
                look.m = look.m.scaled(v, 1.0);
            }
            b"yscale" => {
                let v = self.value(value)?;
                look.m = look.m.scaled(1.0, v);
            }
            b"rotate" => {
                let v = self.value(value)?;
                if node {
                    look.node.turn += v;
                } else {
                    look.m = look.m.rotated(v);
                }
            }
            b"xshift" => {
                let v = self.value(value)?;
                look.m = look.m.shifted(v, 0.0);
            }
            b"yshift" => {
                let v = self.value(value)?;
                look.m = look.m.shifted(0.0, v);
            }
            b"shift" => {
                let inner = trim(value);
                if inner.len() < 2 || inner[0] != b'(' {
                    return self.fail_at(value, E_OPTION);
                }
                let (x, y) = self.pair(&inner[1..inner.len() - 1])?;
                look.m = look.m.shifted(x, y);
            }
            b"line width" => pen.width = self.value(value)?,
            b"ultra thin" => pen.width = 0.1,
            b"very thin" => pen.width = 0.2,
            b"thin" => pen.width = 0.4,
            b"semithick" => pen.width = 0.6,
            b"thick" => pen.width = 0.8,
            b"very thick" => pen.width = 1.2,
            b"ultra thick" => pen.width = 1.6,
            b"solid" => pen.dash = (0.0, 0.0),
            b"dashed" => pen.dash = (3.0, 3.0),
            b"densely dashed" => pen.dash = (3.0, 2.0),
            b"loosely dashed" => pen.dash = (3.0, 6.0),
            b"dotted" => pen.dash = (0.4, 2.0),
            b"densely dotted" => pen.dash = (0.4, 1.0),
            b"loosely dotted" => pen.dash = (0.4, 4.0),
            b"line cap" | b"line join" => pen.round = value == b"round",
            b"draw" => {
                if value == b"none" {
                    pen.drawn = false;
                    look.node.border = false;
                } else {
                    pen.drawn = true;
                    look.node.border = node;
                    if !value.is_empty() {
                        self.set_color(look, value, false)?;
                    }
                }
            }
            b"fill" => {
                let color = if value.is_empty() { Some(pen.stroke) } else { self.color_value(value) };
                let Some(color) = color else {
                    return self.fail_at(value, E_OPTION);
                };
                pen.fill = color;
            }
            b"color" => self.set_color(look, value, node)?,
            b"text" => {
                let Some(color) = self.color_value(value) else {
                    return self.fail_at(value, E_OPTION);
                };
                look.node.color = color;
            }
            b">" => match tip_named(value) {
                Some(tip) => pen.arrow = tip,
                None => return self.fail_at(value, E_OPTION),
            },
            b"font" => {
                let mut at = 0;
                while at < value.len() {
                    if value[at] != b'\\' {
                        at += 1;
                        continue;
                    }
                    let start = at + 1;
                    at = start;
                    while at < value.len() && value[at].is_ascii_alphabetic() {
                        at += 1;
                    }
                    let name = &value[start..at];
                    if let Some(size) = font_size(name, self.doc.pt) {
                        look.node.size = size;
                    }
                    match name {
                        b"bfseries" | b"bf" => look.node.font = FONT_MAIN_BOLD,
                        b"itshape" | b"it" => look.node.font = FONT_MAIN_ITALIC,
                        b"ttfamily" | b"tt" => look.node.font = FONT_TYPEWRITER_REGULAR,
                        b"normalfont" | b"rmfamily" | b"mdseries" | b"upshape" => look.node.font = 0,
                        _ => {}
                    }
                }
            }
            b"inner sep" => look.node.sep = self.value(value)?,
            b"outer sep" | b"transform shape" | b"rectangle" => {}
            b"minimum size" => {
                let v = self.value(value)?;
                look.node.least = (v, v);
            }
            b"minimum width" => look.node.least.0 = self.value(value)?,
            b"minimum height" => look.node.least.1 = self.value(value)?,
            b"text width" => look.node.wrap = self.value(value)?,
            b"align" => {
                look.node.align = match value {
                    b"left" => ALIGN_LEFT,
                    b"center" => ALIGN_CENTER,
                    b"right" => ALIGN_RIGHT,
                    b"flush left" => ALIGN_LEFT | ALIGN_FLUSH,
                    b"flush center" => ALIGN_CENTER | ALIGN_FLUSH,
                    b"flush right" => ALIGN_RIGHT | ALIGN_FLUSH,
                    b"none" | b"justify" => ALIGN_NONE,
                    _ => return self.fail_at(value, E_OPTION),
                };
            }
            b"rounded corners" => pen.corner = if value.is_empty() { 4.0 } else { self.value(value)? },
            b"sharp corners" => pen.corner = 0.0,
            b"x" | b"y" | b"z" => {
                let index = (key[0] - b'x') as usize;
                look.axes[index] = self.axis_vector(index, value)?;
            }
            b"step" => {
                let v = self.grid_step(look, value, 0)?;
                look.step = (v, self.grid_step(look, value, 1)?);
            }
            b"xstep" => look.step.0 = self.grid_step(look, value, 0)?,
            b"ystep" => look.step.1 = self.grid_step(look, value, 1)?,
            b"circle" => look.node.round = true,
            b"anchor" => match anchor_value(value) {
                Some(a) => look.node.anchor = a,
                None => return self.fail_at(value, E_OPTION),
            },
            b"midway" => look.node.place = 0.5,
            b"near start" => look.node.place = 0.25,
            b"near end" => look.node.place = 0.75,
            b"at start" => look.node.place = 0.0,
            b"at end" => look.node.place = 1.0,
            b"pos" => look.node.place = self.value(value)?,
            _ => {
                if let Some(anchor) = placement(key) {
                    look.node.anchor = anchor;
                    look.node.gap = if value.is_empty() { 0.0 } else { self.value(value)? };
                    return Ok(());
                }
                if let Some(color) = self.color_value(key) {
                    pen.stroke = color;
                    if node {
                        look.node.color = color;
                    }
                    return Ok(());
                }
                for i in (0..pic.styles.len()).rev() {
                    let (name, body) = pic.styles[i];
                    if name == key {
                        return self.options(pic, look, body, node, depth + 1);
                    }
                }
                if let Some((start, end)) = tips(key) {
                    look.pen.start = start;
                    look.pen.end = end;
                    return Ok(());
                }
                return self.fail_at(key, E_OPTION);
            }
        }
        Ok(())
    }

    fn pair(&mut self, text: &'static [u8]) -> R<(f64, f64)> {
        let mut depth = 0;
        for (i, c) in text.iter().enumerate() {
            match c {
                b'(' | b'{' => depth += 1,
                b')' | b'}' => depth -= 1,
                b',' if depth == 0 => {
                    return Ok((self.axis_value(&text[..i])?, self.axis_value(&text[i + 1..])?));
                }
                _ => {}
            }
        }
        self.fail_at(text, E_UNIT)
    }

    fn axis_vector(&mut self, index: usize, value: &'static [u8]) -> R<(f64, f64)> {
        let t = trim(value);
        if t.first() == Some(&b'(') && t.last() == Some(&b')') {
            return self.pair(&t[1..t.len() - 1]);
        }
        let v = self.axis_value(t)?;
        Ok(if index == 1 { (0.0, v) } else { (v, 0.0) })
    }

    fn grid_step(&mut self, look: &Look, value: &'static [u8], index: usize) -> R<f64> {
        match evaluate(trim(value), 0.0, 0.0) {
            Some((v, _)) if v.is_nan() || v <= 0.0 => self.fail_at(value, E_OPTION),
            Some((v, true)) => Ok(v),
            Some((v, false)) => Ok(v * length_of(look.axes[index])),
            None => self.fail_at(value, E_OPTION),
        }
    }

    fn coordinates(&mut self, look: &Look, text: &'static [u8]) -> R<(f64, f64)> {
        let mut parts = [&text[0..0]; 3];
        let (mut count, mut depth, mut start) = (0, 0, 0);
        for (i, c) in text.iter().enumerate() {
            match c {
                b'(' | b'{' => depth += 1,
                b')' | b'}' => depth -= 1,
                b',' if depth == 0 => {
                    if count == 2 {
                        return self.fail_at(text, E_UNIT);
                    }
                    parts[count] = &text[start..i];
                    count += 1;
                    start = i + 1;
                }
                _ => {}
            }
        }
        if count == 0 {
            return self.fail_at(text, E_UNIT);
        }
        parts[count] = &text[start..];
        let mut point = (0.0, 0.0);
        for (index, part) in parts.iter().enumerate().take(count + 1) {
            let Some((v, unit)) = evaluate(trim(part), 0.0, 0.0).filter(|(v, _)| !v.is_nan()) else {
                return self.fail_at(part, E_UNIT);
            };
            let axis = look.axes[index];
            if unit {
                let along = if index == 1 { (0.0, v) } else { (v, 0.0) };
                point = (point.0 + along.0, point.1 + along.1);
            } else {
                point = (point.0 + v * axis.0, point.1 + v * axis.1);
            }
        }
        Ok(point)
    }

    fn axis_value(&mut self, text: &'static [u8]) -> R<f64> {
        match evaluate(trim(text), 0.0, 0.0) {
            Some((v, unit)) if !v.is_nan() => Ok(if unit { v } else { v * CM }),
            _ => self.fail_at(text, E_UNIT),
        }
    }

    fn plain_value(&mut self, text: &'static [u8]) -> R<f64> {
        match evaluate(trim(text), 0.0, 0.0) {
            Some((v, _)) if !v.is_nan() => Ok(v),
            _ => self.fail_at(text, E_UNIT),
        }
    }

    fn plain_pair(&mut self, text: &'static [u8]) -> R<(f64, f64)> {
        match text.iter().position(|c| *c == b',') {
            Some(i) => Ok((self.plain_value(&text[..i])?, self.plain_value(&text[i + 1..])?)),
            None => self.fail_at(text, E_UNIT),
        }
    }

    fn resolve(&mut self, pic: &Pic, look: &Look, text: &'static [u8]) -> R<(f64, f64)> {
        let held = self.pos;
        match self.resolve_exact(pic, look, text) {
            Ok(point) => Ok(point),
            Err(()) if matches!(self.err, E_OPTION | E_ARG | E_UNIT | E_MISPLACED) => {
                self.err = 0;
                self.note(held);
                self.pos = held;
                Ok((0.0, 0.0))
            }
            Err(()) => Err(()),
        }
    }

    fn color_value(&self, text: &[u8]) -> Option<u32> {
        parse_color(text, &|name| self.named_color(name))
    }

    fn resolve_exact(&mut self, pic: &Pic, look: &Look, text: &'static [u8]) -> R<(f64, f64)> {
        let text = self.expand_text(text);
        let t = trim(text);
        if t.first() == Some(&b'$') && t.last() == Some(&b'$') && t.len() >= 2 {
            return self.calc_point(pic, look, &t[1..t.len() - 1]);
        }
        if let Some(rest) = t.strip_prefix(b"rel axis cs:") {
            let Some(map) = pic.axis else {
                return self.fail_at(t, E_MISPLACED);
            };
            let (x, y) = self.plain_pair(rest)?;
            return Ok(map.relative(x, y));
        }
        if let Some(rest) = t.strip_prefix(b"axis cs:") {
            let Some(map) = pic.axis else {
                return self.fail_at(t, E_MISPLACED);
            };
            let (x, y) = self.plain_pair(rest)?;
            return Ok(map.at(x, y));
        }
        if t.contains(&b',') {
            if let Some(map) = pic.axis {
                let (x, y) = self.plain_pair(t)?;
                return Ok(map.at(x, y));
            }
            let p = self.coordinates(look, t)?;
            return Ok(look.m.at(p));
        }
        if let Some(i) = t.iter().position(|c| *c == b':') {
            let angle = self.plain_value(&t[..i])?;
            let (rx, ry) = self.axis_radius(look, &t[i + 1..])?;
            return Ok(look.m.at((rx * cos_deg(angle), ry * sin_deg(angle))));
        }
        let name = match t.iter().position(|c| *c == b'.') {
            Some(i) => &t[..i],
            None => t,
        };
        for known in pic.coords.iter().rev() {
            if known.name == name {
                pic.extent.set(known.extent);
                return Ok((known.x, known.y));
            }
        }
        self.fail_at(t, E_ARG)
    }

    fn calc_point(&mut self, pic: &Pic, look: &Look, text: &'static [u8]) -> R<(f64, f64)> {
        self.enter()?;
        let total = self.calc_sum(pic, look, text)?;
        self.depth -= 1;
        Ok(total)
    }

    fn calc_sum(&mut self, pic: &Pic, look: &Look, text: &'static [u8]) -> R<(f64, f64)> {
        let mut total = (0.0, 0.0);
        let mut sign = 1.0;
        let mut i = 0;
        while i < text.len() {
            match text[i] {
                b' ' | b'\n' | b'\t' => i += 1,
                b'+' => {
                    sign = 1.0;
                    i += 1;
                }
                b'-' => {
                    sign = -1.0;
                    i += 1;
                }
                _ => {
                    let mut factor = 1.0;
                    if text[i] != b'(' {
                        let Some(open) = text[i..].iter().position(|c| *c == b'(') else {
                            return self.fail_at(text, E_ARG);
                        };
                        let head = trim(&text[i..i + open]);
                        factor = self.plain_value(head.strip_suffix(b"*").unwrap_or(head))?;
                        i += open;
                    }
                    let (inner, next) = self.calc_term(text, i)?;
                    let mut p = self.resolve(pic, look, inner)?;
                    i = next;
                    if i < text.len() && text[i] == b'!' {
                        let Some(mid) = text[i + 1..].iter().position(|c| *c == b'!') else {
                            return self.fail_at(text, E_ARG);
                        };
                        let amount = self.plain_value(&text[i + 1..i + 1 + mid])?;
                        let (other, after) = self.calc_term(text, i + mid + 2)?;
                        let q = self.resolve(pic, look, other)?;
                        p = (p.0 + (q.0 - p.0) * amount, p.1 + (q.1 - p.1) * amount);
                        i = after;
                    }
                    total.0 += sign * factor * p.0;
                    total.1 += sign * factor * p.1;
                }
            }
        }
        Ok(total)
    }

    fn calc_term(&mut self, text: &'static [u8], start: usize) -> R<(&'static [u8], usize)> {
        if start >= text.len() || text[start] != b'(' {
            return self.fail_at(text, E_ARG);
        }
        let mut depth = 0;
        for (i, c) in text.iter().enumerate().skip(start) {
            match c {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok((&text[start + 1..i], i + 1));
                    }
                }
                _ => {}
            }
        }
        self.fail_at(text, E_ARG)
    }

    pub(super) fn word(&mut self) -> &'static [u8] {
        let start = self.pos;
        while self.peek().is_ascii_alphabetic() {
            self.pos += 1;
        }
        self.slice(start, self.pos)
    }

    fn target(&mut self, pic: &Pic, look: &Look, cur: (f64, f64)) -> R<((f64, f64), bool)> {
        self.skip_gap();
        pic.extent.set(Extent::default());
        let mut relative = 0;
        while self.peek() == b'+' {
            relative += 1;
            self.pos += 1;
        }
        if self.peek() != b'(' {
            return self.fail(E_ARG);
        }
        let inner = self.nested(b'(', b')')?;
        if relative == 0 {
            return Ok((self.resolve(pic, look, inner)?, true));
        }
        let t = trim(inner);
        let v = if pic.axis.is_some() { self.plain_pair(t)? } else { self.coordinates(look, t)? };
        let d = if pic.axis.is_some() { v } else { look.m.vector(v) };
        Ok(((cur.0 + d.0, cur.1 + d.1), relative == 2))
    }

    fn axis_radius(&mut self, look: &Look, text: &'static [u8]) -> R<(f64, f64)> {
        match evaluate(trim(text), 0.0, 0.0) {
            Some((v, true)) if !v.is_nan() => Ok((v, v)),
            Some((v, false)) if !v.is_nan() => Ok((v * length_of(look.axes[0]), v * length_of(look.axes[1]))),
            _ => self.fail_at(text, E_UNIT),
        }
    }

    fn radii(&mut self, look: &Look, text: &'static [u8]) -> R<(f64, f64)> {
        let t = trim(text);
        if let Some(i) = t.windows(5).position(|w| w == b" and ") {
            return Ok((self.axis_radius(look, &t[..i])?.0, self.axis_radius(look, &t[i + 5..])?.1));
        }
        self.axis_radius(look, t)
    }

    pub(super) fn sweep(&mut self, pic: &mut Pic, look: &Look, center: (f64, f64), r: (f64, f64), from: f64, to: f64) {
        let to = from + (to - from).clamp(-MAX_SWEEP, MAX_SWEEP);
        let turns = abs(to - from);
        let steps = (turns / 90.0) as i32 + 1;
        let step = (to - from) / steps as f64;
        let k = 4.0 / 3.0 * sin_deg(step / 4.0) / cos_deg(step / 4.0);
        let at = |a: f64| (r.0 * cos_deg(a), r.1 * sin_deg(a));
        let slope = |a: f64| (-r.0 * sin_deg(a), r.1 * cos_deg(a));
        for i in 0..steps {
            let a = from + step * i as f64;
            let b = a + step;
            let (p, q) = (at(a), at(b));
            let (dp, dq) = (slope(a), slope(b));
            let points = [(p.0 + k * dp.0, p.1 + k * dp.1), (q.0 - k * dq.0, q.1 - k * dq.1), q];
            for point in points {
                let v = look.m.vector(point);
                pic.path.push((SEG_CURVE, center.0 + v.0, center.1 + v.1));
            }
        }
    }

    fn walk_path(&mut self, d: &mut Doc, pic: &mut Pic, look: &Look) -> R<()> {
        let mut cur = (0.0, 0.0);
        let mut last = cur;
        let mut first = cur;
        let mut held = Extent::default();
        loop {
            self.skip_gap();
            if self.at_end() {
                return self.fail(E_END);
            }
            match self.peek() {
                b';' => {
                    self.pos += 1;
                    return Ok(());
                }
                b'(' | b'+' => {
                    let (p, keep) = self.target(pic, look, cur)?;
                    if keep {
                        cur = p;
                    }
                    first = p;
                    last = p;
                    held = pic.extent.get();
                    if matches!(pic.path.last(), Some((SEG_MOVE, _, _))) {
                        pic.path.pop();
                    }
                    pic.path.push((SEG_MOVE, p.0, p.1));
                }
                b'-' if self.peek_at(1) == b'-' => {
                    self.pos += 2;
                    self.skip_gap();
                    if self.slice(self.pos, (self.pos + 5).min(self.len)) == b"cycle" {
                        self.pos += 5;
                        pic.path.push((SEG_CLOSE, first.0, first.1));
                        cur = first;
                        continue;
                    }
                    let nodes = self.skip_nodes()?;
                    let (p, keep) = self.target(pic, look, cur)?;
                    self.segment_nodes(d, pic, look, nodes, cur, p)?;
                    let reached = pic.extent.get();
                    let from = border(cur, held, p);
                    let to = border(p, reached, cur);
                    if held.w > 0.0 {
                        if matches!(pic.path.last(), Some((SEG_MOVE, _, _))) {
                            pic.path.pop();
                        }
                        pic.path.push((SEG_MOVE, from.0, from.1));
                    }
                    pic.path.push((SEG_LINE, to.0, to.1));
                    last = cur;
                    held = reached;
                    if keep {
                        cur = p;
                    }
                }
                b'-' | b'|' => {
                    let vertical_first = self.peek() == b'|';
                    if self.peek_at(1) != if vertical_first { b'-' } else { b'|' } {
                        return self.fail(E_MISPLACED);
                    }
                    self.pos += 2;
                    let (p, keep) = self.target(pic, look, cur)?;
                    let corner = if vertical_first { (cur.0, p.1) } else { (p.0, cur.1) };
                    pic.path.push((SEG_LINE, corner.0, corner.1));
                    pic.path.push((SEG_LINE, p.0, p.1));
                    last = corner;
                    if keep {
                        cur = p;
                    }
                }
                b'.' => {
                    self.pos += 2;
                    self.skip_gap();
                    if self.word() != b"controls" {
                        return self.fail(E_MISPLACED);
                    }
                    let (a, _) = self.target(pic, look, cur)?;
                    self.skip_gap();
                    let mut b = a;
                    if self.peek() == b'a' {
                        self.word();
                        b = self.target(pic, look, cur)?.0;
                        self.skip_gap();
                    }
                    if self.peek() != b'.' {
                        return self.fail(E_MISPLACED);
                    }
                    self.pos += 2;
                    let (p, keep) = self.target(pic, look, cur)?;
                    pic.path.push((SEG_CURVE, a.0, a.1));
                    pic.path.push((SEG_CURVE, b.0, b.1));
                    pic.path.push((SEG_CURVE, p.0, p.1));
                    last = cur;
                    if keep {
                        cur = p;
                    }
                }
                _ => {
                    let at = self.pos;
                    match self.word() {
                        b"rectangle" => {
                            let (p, keep) = self.target(pic, look, cur)?;
                            let size = look.m.undo((p.0 - cur.0, p.1 - cur.1));
                            let across = look.m.vector((size.0, 0.0));
                            let down = look.m.vector((0.0, size.1));
                            pic.path.push((SEG_LINE, cur.0 + across.0, cur.1 + across.1));
                            pic.path.push((SEG_LINE, p.0, p.1));
                            pic.path.push((SEG_LINE, cur.0 + down.0, cur.1 + down.1));
                            pic.path.push((SEG_CLOSE, cur.0, cur.1));
                            if keep {
                                cur = p;
                            }
                        }
                        b"circle" | b"ellipse" => {
                            let r = self.round_size(look)?;
                            let start = look.m.vector((r.0, 0.0));
                            pic.path.push((SEG_MOVE, cur.0 + start.0, cur.1 + start.1));
                            self.sweep(pic, look, cur, r, 0.0, 360.0);
                            pic.path.push((SEG_CLOSE, cur.0, cur.1));
                        }
                        b"arc" => {
                            let (from, to, r) = self.arc_size(look)?;
                            let v = look.m.vector((r.0 * cos_deg(from), r.1 * sin_deg(from)));
                            let center = (cur.0 - v.0, cur.1 - v.1);
                            self.sweep(pic, look, center, r, from, to);
                            let end = look.m.vector((r.0 * cos_deg(to), r.1 * sin_deg(to)));
                            last = cur;
                            cur = (center.0 + end.0, center.1 + end.1);
                        }
                        b"coordinate" => {
                            self.square_options()?;
                            self.skip_gap();
                            let name = self.nested(b'(', b')')?;
                            let name = trim(self.expand_text(name));
                            let name = self.lasting(name);
                            pic.coords.push(Named { name, x: cur.0, y: cur.1, extent: Extent::default() });
                        }
                        b"node" => {
                            let mut l = *look;
                            self.node_options(pic, &mut l)?;
                            let t = l.node.place;
                            let p = (last.0 + (cur.0 - last.0) * t, last.1 + (cur.1 - last.1) * t);
                            self.node(d, pic, &mut l, Some(p))?;
                        }
                        b"to" => {
                            let bend = self.square_options()?;
                            let nodes = self.skip_nodes()?;
                            self.skip_gap();
                            let (p, keep) = self.target(pic, look, cur)?;
                            self.segment_nodes(d, pic, look, nodes, cur, p)?;
                            match self.to_controls(bend, cur, p)? {
                                Some((a, b)) => {
                                    pic.path.push((SEG_CURVE, a.0, a.1));
                                    pic.path.push((SEG_CURVE, b.0, b.1));
                                    pic.path.push((SEG_CURVE, p.0, p.1));
                                }
                                None => pic.path.push((SEG_LINE, p.0, p.1)),
                            }
                            last = cur;
                            if keep {
                                cur = p;
                            }
                        }
                        b"edge" => {
                            self.square_options()?;
                            let nodes = self.skip_nodes()?;
                            self.skip_gap();
                            let (p, _) = self.target(pic, look, cur)?;
                            self.segment_nodes(d, pic, look, nodes, cur, p)?;
                            pic.path.push((SEG_MOVE, cur.0, cur.1));
                            pic.path.push((SEG_LINE, p.0, p.1));
                            pic.path.push((SEG_MOVE, cur.0, cur.1));
                        }
                        b"grid" => {
                            let step = self.square_options()?;
                            let (p, keep) = self.target(pic, look, cur)?;
                            self.add_grid(pic, look, cur, p, step);
                            if keep {
                                cur = p;
                            }
                        }
                        _ => {
                            self.pos = at;
                            self.note(at);
                            if self.peek() == b'[' {
                                self.square_options()?;
                                continue;
                            }
                            return self.skip_statement();
                        }
                    }
                }
            }
        }
    }

    fn to_controls(&mut self, options: &'static [u8], from: (f64, f64), to: (f64, f64)) -> R<Option<((f64, f64), (f64, f64))>> {
        let toward = angle_of(to.1 - from.1, to.0 - from.0);
        let (mut leave, mut arrive, mut loose) = (None, None, 1.0);
        let mut at = 0;
        while let Some((key, value)) = next_option(options, &mut at) {
            let number = |engine: &mut Self, fallback: f64| if value.is_empty() { Ok(fallback) } else { engine.plain_value(value) };
            match key {
                b"out" => leave = Some(self.plain_value(value)?),
                b"in" => arrive = Some(self.plain_value(value)?),
                b"looseness" => loose = self.plain_value(value)?,
                b"bend left" => {
                    let bend = number(self, 30.0)?;
                    leave = Some(toward + bend);
                    arrive = Some(toward + 180.0 - bend);
                }
                b"bend right" => {
                    let bend = number(self, 30.0)?;
                    leave = Some(toward - bend);
                    arrive = Some(toward + 180.0 + bend);
                }
                _ => {}
            }
        }
        if leave.is_none() && arrive.is_none() {
            return Ok(None);
        }
        let reach = TO_REACH * loose * length_of((to.0 - from.0, to.1 - from.1));
        let (leave, arrive) = (leave.unwrap_or(toward), arrive.unwrap_or(toward + 180.0));
        Ok(Some((
            (from.0 + reach * cos_deg(leave), from.1 + reach * sin_deg(leave)),
            (to.0 + reach * cos_deg(arrive), to.1 + reach * sin_deg(arrive)),
        )))
    }

    fn skip_nodes(&mut self) -> R<Option<usize>> {
        let mut first = None;
        loop {
            self.skip_gap();
            if self.peek() != b'n' || self.slice(self.pos, (self.pos + 4).min(self.len)) != b"node" || self.peek_at(4).is_ascii_alphabetic()
            {
                return Ok(first);
            }
            first = first.or(Some(self.pos));
            self.pos += 4;
            self.square_options()?;
            self.skip_gap();
            if self.peek() == b'(' {
                self.nested(b'(', b')')?;
                self.skip_gap();
            }
            if self.peek() == b'{' {
                self.group_text()?;
            }
        }
    }

    fn segment_nodes(&mut self, d: &mut Doc, pic: &mut Pic, look: &Look, nodes: Option<usize>, from: (f64, f64), to: (f64, f64)) -> R<()> {
        let Some(at) = nodes else {
            return Ok(());
        };
        let held = self.pos;
        let extent = pic.extent.get();
        self.pos = at;
        while self.slice(self.pos, (self.pos + 4).min(self.len)) == b"node" && !self.peek_at(4).is_ascii_alphabetic() {
            self.pos += 4;
            let mut l = *look;
            l.node.place = 0.5;
            self.node_options(pic, &mut l)?;
            let t = l.node.place;
            let p = (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t);
            self.node(d, pic, &mut l, Some(p))?;
            self.skip_gap();
        }
        self.pos = held;
        pic.extent.set(extent);
        Ok(())
    }

    fn add_grid(&mut self, pic: &mut Pic, look: &Look, from: (f64, f64), to: (f64, f64), options: &'static [u8]) {
        let mut step = look.step;
        let mut at = 0;
        while let Some((key, value)) = next_option(options, &mut at) {
            if let Some((v, unit)) = evaluate(trim(value), 0.0, 0.0).filter(|(v, _)| *v > 0.0) {
                let across = if unit { v } else { v * length_of(look.axes[0]) };
                let down = if unit { v } else { v * length_of(look.axes[1]) };
                match key {
                    b"step" => step = (across, down),
                    b"xstep" => step.0 = across,
                    b"ystep" => step.1 = down,
                    _ => {}
                }
            }
        }
        let (a, b) = (look.m.undo(from), look.m.undo(to));
        let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
        let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
        let mut lines = 0;
        let mut x = ceil_steps(x0, step.0) * step.0;
        while x <= x1 + 0.001 && lines < 2000 {
            let (p, q) = (look.m.vector((x, y0)), look.m.vector((x, y1)));
            let base = look.m.vector((0.0, 0.0));
            let shift = (from.0 - look.m.vector(a).0 + base.0, from.1 - look.m.vector(a).1 + base.1);
            pic.path.push((SEG_MOVE, p.0 + shift.0, p.1 + shift.1));
            pic.path.push((SEG_LINE, q.0 + shift.0, q.1 + shift.1));
            x += step.0;
            lines += 1;
        }
        let mut y = ceil_steps(y0, step.1) * step.1;
        while y <= y1 + 0.001 && lines < 4000 {
            let (p, q) = (look.m.vector((x0, y)), look.m.vector((x1, y)));
            let base = look.m.vector((0.0, 0.0));
            let shift = (from.0 - look.m.vector(a).0 + base.0, from.1 - look.m.vector(a).1 + base.1);
            pic.path.push((SEG_MOVE, p.0 + shift.0, p.1 + shift.1));
            pic.path.push((SEG_LINE, q.0 + shift.0, q.1 + shift.1));
            y += step.1;
            lines += 1;
        }
        pic.path.push((SEG_MOVE, to.0, to.1));
    }

    fn round_size(&mut self, look: &Look) -> R<(f64, f64)> {
        self.skip_gap();
        if self.peek() == b'(' {
            let inner = self.nested(b'(', b')')?;
            return self.radii(look, inner);
        }
        let text = self.square_options()?;
        let mut at = 0;
        let mut r = (CM, CM);
        while let Some((key, value)) = next_option(text, &mut at) {
            let v = self.axis_radius(look, value)?;
            match key {
                b"radius" => r = v,
                b"x radius" => r.0 = v.0,
                b"y radius" => r.1 = v.1,
                _ => {}
            }
        }
        Ok(r)
    }

    fn arc_size(&mut self, look: &Look) -> R<(f64, f64, (f64, f64))> {
        self.skip_gap();
        if self.peek() == b'(' {
            let inner = trim(self.nested(b'(', b')')?);
            let mut parts = inner.splitn(3, |c| *c == b':');
            let (Some(a), Some(b), Some(r)) = (parts.next(), parts.next(), parts.next()) else {
                return self.fail_at(inner, E_ARG);
            };
            return Ok((self.plain_value(a)?, self.plain_value(b)?, self.radii(look, r)?));
        }
        let text = self.square_options()?;
        let mut at = 0;
        let (mut from, mut to, mut r) = (0.0, 90.0, (CM, CM));
        while let Some((key, value)) = next_option(text, &mut at) {
            match key {
                b"start angle" => from = self.plain_value(value)?,
                b"end angle" => to = self.plain_value(value)?,
                b"radius" => r = self.axis_radius(look, value)?,
                _ => {}
            }
        }
        Ok((from, to, r))
    }

    pub(super) fn stroke_path(&mut self, pic: &mut Pic, pen: &Pen) {
        if pic.path.is_empty() {
            return;
        }
        let mut heads = [None, None];
        if pen.drawn {
            heads[0] = self.trim_tip(pic, pen, pen.start, true);
            heads[1] = self.trim_tip(pic, pen, pen.end, false);
        }
        let solid = pen.solid || pen.fill != NO_COLOR;
        self.flush_path(pic, pen, pen.drawn, solid);
        for head in heads.into_iter().flatten() {
            self.draw_tip(pic, pen, head);
        }
    }

    fn flush_path(&mut self, pic: &mut Pic, pen: &Pen, drawn: bool, solid: bool) {
        if pen.corner > 0.0 {
            round_corners(&mut pic.path, pen.corner);
        }
        let k = 1000.0 / self.doc.pt as f64;
        let s = self.doc.segs.len() as u32;
        for i in 0..pic.path.len() {
            let (op, x, y) = pic.path[i];
            pic.grow(x, y);
            self.doc.segs.push(Seg { op, x: to_units(x * k), y: to_units(-y * k) });
        }
        pic.path.clear();
        if !drawn && !solid {
            self.doc.segs.truncate(s as usize);
            return;
        }
        let fill = if !solid {
            NO_COLOR
        } else if pen.fill != NO_COLOR {
            pen.fill
        } else {
            pen.stroke
        };
        pic.art.push(Art {
            s,
            e: self.doc.segs.len() as u32,
            x: 0,
            y: 0,
            stroke: if drawn { pen.stroke } else { NO_COLOR },
            fill,
            width: (pen.width * k) as i32,
            dash: ((pen.dash.0 * k) as i32, (pen.dash.1 * k) as i32),
            round: pen.round,
            text: false,
            cos: 0,
            sin: 0,
        });
    }

    fn trim_tip(&self, pic: &mut Pic, pen: &Pen, tip: Tip, start: bool) -> Option<(Tip, (f64, f64), (f64, f64))> {
        let tip = if tip.kind == TIP_DEFAULT { Tip { length: tip.length.max(pen.arrow.length), ..pen.arrow } } else { tip };
        if tip.kind == TIP_NONE || pic.path.len() < 2 {
            return None;
        }
        let n = pic.path.len();
        let (end, near) = if start { (0, 1) } else { (n - 1, n - 2) };
        let (op, ex, ey) = pic.path[end];
        if op == SEG_CLOSE {
            return None;
        }
        let (_, nx, ny) = pic.path[near];
        let (dx, dy) = (ex - nx, ey - ny);
        let size = sqrt(dx * dx + dy * dy);
        if size == 0.0 {
            return None;
        }
        let dir = (dx / size, dy / size);
        let length = Self::tip_length(tip, pen);
        let back = if tip.kind == TIP_STEALTH { length * 0.675 } else { pen.width / 2.0 };
        pic.path[end].1 = ex - dir.0 * back;
        pic.path[end].2 = ey - dir.1 * back;
        Some((tip, (ex, ey), dir))
    }

    fn tip_length(tip: Tip, pen: &Pen) -> f64 {
        if tip.length > 0.0 {
            tip.length
        } else if tip.kind == TIP_STEALTH {
            3.0 + 4.5 * pen.width
        } else {
            1.8 + 2.5 * pen.width
        }
    }

    fn draw_tip(&mut self, pic: &mut Pic, pen: &Pen, head: (Tip, (f64, f64), (f64, f64))) {
        let (tip, at, dir) = head;
        let length = Self::tip_length(tip, pen);
        let normal = (-dir.1, dir.0);
        let mark = |back: f64, side: f64| (at.0 - dir.0 * back + normal.0 * side, at.1 - dir.1 * back + normal.1 * side);
        let plain = Pen { dash: (0.0, 0.0), fill: NO_COLOR, solid: false, ..*pen };
        if tip.kind == TIP_STEALTH {
            let half = length * 0.375;
            let (a, b, c) = (mark(length, half), mark(length * 0.675, 0.0), mark(length, -half));
            pic.path.push((SEG_MOVE, at.0, at.1));
            pic.path.push((SEG_LINE, a.0, a.1));
            pic.path.push((SEG_LINE, b.0, b.1));
            pic.path.push((SEG_LINE, c.0, c.1));
            pic.path.push((SEG_CLOSE, at.0, at.1));
            self.flush_path(pic, &plain, false, true);
        } else {
            let half = length * 0.6;
            let (a, b) = (mark(length, half), mark(length, -half));
            pic.path.push((SEG_MOVE, a.0, a.1));
            pic.path.push((SEG_LINE, at.0, at.1));
            pic.path.push((SEG_LINE, b.0, b.1));
            self.flush_path(pic, &plain, true, false);
        }
    }

    fn node_options(&mut self, pic: &mut Pic, look: &mut Look) -> R<()> {
        look.pen.drawn = false;
        look.pen.fill = NO_COLOR;
        look.pen.solid = false;
        look.pen.start = first_look().pen.start;
        look.pen.end = first_look().pen.end;
        look.pen.dash = (0.0, 0.0);
        let every = pic.every_node;
        self.options(pic, look, every, true, 0)?;
        let text = self.square_options()?;
        self.options(pic, look, text, true, 0)
    }

    pub(super) fn text_ctx(&mut self, d: &Doc, size: u16, color: u32) -> Ctx {
        let mut c = d.ctx;
        if size != 0 {
            c.size = size;
        }
        let mut c = self.restyle(&c, ST_T);
        if color != 0 {
            let mut hex = *b"#000000";
            let digits = b"0123456789abcdef";
            for i in 0..6 {
                hex[6 - i] = digits[((color >> (i * 4)) & 15) as usize];
            }
            if let Ok(id) = self.intern_color(&hex) {
                c.color = id;
                c.grp = self.grp(id, c.scale);
            }
        }
        c
    }

    fn node(&mut self, d: &mut Doc, pic: &mut Pic, look: &mut Look, at: Option<(f64, f64)>) -> R<()> {
        let mut point = at;
        let mut named: &'static [u8] = b"";
        loop {
            self.skip_gap();
            match self.peek() {
                b'[' => {
                    let text = self.square_options()?;
                    self.options(pic, look, text, true, 0)?;
                }
                b'(' => {
                    named = trim(self.nested(b'(', b')')?);
                }
                b'a' if self.peek_at(1) == b't' => {
                    self.pos += 2;
                    self.skip_gap();
                    if self.peek() != b'(' {
                        return self.fail(E_ARG);
                    }
                    let inner = self.nested(b'(', b')')?;
                    point = Some(self.resolve(pic, look, inner)?);
                }
                b'{' => break,
                _ => return self.fail(E_ARG),
            }
        }
        let p = point.unwrap_or_else(|| look.m.at((0.0, 0.0)));
        let c = self.text_ctx(d, look.node.size, look.node.color);
        self.pos += 1;
        let mut font = look.node.font;
        self.inline(d, &c, &mut font, true)?;
        let b = if look.node.wrap > 0.0 { self.wrapped_box(d, &c, look.node.wrap, look.node.align) } else { self.line_box(d, &c) };
        self.skip_gap();
        let n = look.node;
        let (center, extent) = self.place_text(pic, &look.pen, b, p, &n);
        if !named.is_empty() {
            let named = self.lasting(named);
            pic.coords.push(Named { name: named, x: center.0, y: center.1, extent });
        }
        Ok(())
    }

    fn wrapped_box(&mut self, d: &mut Doc, c: &Ctx, width: f64, align: u8) -> Bx {
        self.flush_word(d, false);
        let width = to_units(width * 1000.0 / self.doc.pt as f64);
        let first = self.words.first().map_or(self.end(), |word| word.bx.s);
        let mark = self.doc.items.len();
        let mut f = d.clone();
        f.left = 0;
        f.top = 0;
        f.text_w = width;
        f.text_h = super::float::WIDE;
        f.base = 0;
        f.dep = 0;
        f.skip = 0;
        f.note_h = 0;
        f.bottom_h = 0;
        f.topskip = 0;
        f.fresh = true;
        f.word = None;
        f.label = None;
        let flush = align & ALIGN_FLUSH != 0;
        let align = align & !ALIGN_FLUSH;
        f.line_fill = match align {
            _ if flush => 0,
            ALIGN_NONE => 0,
            ALIGN_CENTER => CENTERED_FILL,
            _ => RAGGED_FILL,
        };
        self.set_lines(&mut f, c, 0, width, 0, align != ALIGN_NONE, align == ALIGN_CENTER);
        let h = f.base + f.dep;
        let lines = self.doc.items.split_off(mark);
        self.items.truncate(first as usize);
        let s = self.end();
        self.items.extend(lines.iter().map(|it| Item { y: it.y - h, ..*it }));
        self.close(s, width, h, 0)
    }

    pub(super) fn place_text(&mut self, pic: &mut Pic, pen: &Pen, b: Bx, p: (f64, f64), n: &NodeLook) -> ((f64, f64), Extent) {
        let pt = self.doc.pt as f64 / 1000.0;
        let (w, top, bottom) = (b.w as f64 * pt, b.h as f64 * pt, b.d as f64 * pt);
        let half = ((w / 2.0 + n.sep).max(n.least.0 / 2.0), ((top + bottom) / 2.0 + n.sep).max(n.least.1 / 2.0));
        let (cos, sin) = (cos_deg(n.turn), sin_deg(n.turn));
        let turn = |v: (f64, f64)| (v.0 * cos - v.1 * sin, v.0 * sin + v.1 * cos);
        let reach = (-(n.anchor.0 as f64) * (half.0 + n.gap), -(n.anchor.1 as f64) * (half.1 + n.gap));
        let shift = turn(reach);
        let center = (p.0 + shift.0, p.1 + shift.1);
        let radius = sqrt(half.0 * half.0 + half.1 * half.1);
        let extent = if n.round { Extent { w: radius, h: radius, round: true } } else { Extent { w: half.0, h: half.1, round: false } };
        let span = (half.0 * abs(cos) + half.1 * abs(sin), half.0 * abs(sin) + half.1 * abs(cos));
        pic.grow(center.0 - span.0, center.1 - span.1);
        pic.grow(center.0 + span.0, center.1 + span.1);
        if n.border || pen.fill != NO_COLOR {
            let border =
                Pen { drawn: n.border, solid: pen.fill != NO_COLOR, start: first_look().pen.start, end: first_look().pen.end, ..*pen };
            core::mem::swap(&mut pic.path, &mut pic.aside);
            pic.path.clear();
            if n.round {
                let r = radius;
                let look = first_look();
                pic.path.push((SEG_MOVE, center.0 + r, center.1));
                self.sweep(pic, &look, center, (r, r), 0.0, 360.0);
                pic.path.push((SEG_CLOSE, center.0, center.1));
            } else {
                for (i, corner) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].iter().enumerate() {
                    let v = turn((corner.0 * half.0, corner.1 * half.1));
                    pic.path.push((if i == 0 { SEG_MOVE } else { SEG_LINE }, center.0 + v.0, center.1 + v.1));
                }
                pic.path.push((SEG_CLOSE, center.0, center.1));
            }
            self.flush_path(pic, &border, border.drawn, border.solid);
            core::mem::swap(&mut pic.path, &mut pic.aside);
        }
        let k = 1000.0 / self.doc.pt as f64;
        let (cx, cy) = (to_units(center.0 * k), to_units(-center.1 * k));
        let lift = (b.h - b.d) / 2;
        if n.turn == 0.0 {
            self.shift(b.s, b.e, cx - b.w / 2, cy + lift);
            return (center, extent);
        }
        self.shift(b.s, b.e, -b.w / 2, lift);
        let s = self.doc.spun.len() as u32;
        self.doc.spun.extend_from_slice(&self.items[b.s as usize..b.e as usize]);
        self.items.truncate(b.s as usize);
        pic.art.push(Art {
            s,
            e: self.doc.spun.len() as u32,
            x: cx,
            y: cy,
            stroke: NO_COLOR,
            fill: NO_COLOR,
            width: 0,
            dash: (0, 0),
            round: false,
            text: true,
            cos: (cos * 1000.0) as i32,
            sin: (-sin * 1000.0) as i32,
        });
        (center, extent)
    }

    pub(super) fn label(&mut self, pic: &mut Pic, look: &Look, b: Bx, p: (f64, f64), anchor: (i8, i8), turn: f64) {
        let n = NodeLook { anchor, sep: 0.0, gap: 0.0, turn, ..look.node };
        let pen = Pen { fill: NO_COLOR, ..look.pen };
        self.place_text(pic, &pen, b, p, &n);
    }

    pub(super) fn skip_statement(&mut self) -> R<()> {
        let mut depth = 0;
        loop {
            if self.at_end() {
                return if self.user_src { self.fail(E_END) } else { Ok(()) };
            }
            match self.peek() {
                b'\\' if depth == 0 && self.slice(self.pos, (self.pos + 5).min(self.len)) == b"\\end{" => return Ok(()),
                b'\\' => self.pos += 1,
                b'{' => depth += 1,
                b'}' => depth -= 1,
                b';' if depth == 0 => {
                    self.pos += 1;
                    return Ok(());
                }
                _ => {}
            }
            self.pos += 1;
        }
    }

    pub(super) fn scene(&mut self, d: &mut Doc, pic: &mut Pic, look: Look, closing: &[u8]) -> R<()> {
        self.enter()?;
        let mark = pic.saved.len();
        loop {
            self.skip_gap();
            if self.at_end() {
                if closing.is_empty() {
                    self.depth -= 1;
                    return Ok(());
                }
                return self.fail(E_END);
            }
            if self.peek() != b'\\' {
                self.note(self.pos);
                self.pos += 1;
                continue;
            }
            let start = self.pos;
            self.pos += 1;
            let name = self.read_name();
            let (p, len, user, depth) = (self.p, self.len, self.user_src, self.depth);
            match self.statement(d, pic, &look, closing, mark, name, start) {
                Ok(true) => return Ok(()),
                Ok(false) => {}
                Err(()) if matches!(self.err, E_DEPTH | E_END | E_ENV) => return Err(()),
                Err(()) => {
                    self.err = 0;
                    (self.p, self.len, self.user_src, self.depth) = (p, len, user, depth);
                    self.words.clear();
                    d.word = None;
                    self.note(start);
                    self.pos = start;
                    if name == b"begin" {
                        self.pos += b"\\begin".len();
                        let inner = self.loose_argument();
                        self.skip_environment(inner);
                    } else {
                        self.skip_statement()?;
                    }
                }
            }
        }
    }

    fn statement(
        &mut self,
        d: &mut Doc,
        pic: &mut Pic,
        look: &Look,
        closing: &[u8],
        mark: usize,
        name: &'static [u8],
        start: usize,
    ) -> R<bool> {
        {
            match name {
                b"draw" | b"fill" | b"filldraw" | b"path" | b"shade" | b"shadedraw" => {
                    let mut l = *look;
                    l.pen.drawn = matches!(name, b"draw" | b"filldraw" | b"shadedraw");
                    l.pen.solid = matches!(name, b"fill" | b"filldraw" | b"shade" | b"shadedraw");
                    let text = self.square_options()?;
                    self.options(pic, &mut l, text, false, 0)?;
                    pic.path.clear();
                    self.walk_path(d, pic, &l)?;
                    self.stroke_path(pic, &l.pen);
                }
                b"clip" | b"useasboundingbox" => {
                    self.square_options()?;
                    pic.path.clear();
                    self.walk_path(d, pic, &look)?;
                    pic.path.clear();
                }
                b"pic" | b"matrix" | b"graph" | b"datavisualization" => {
                    self.note(start);
                    self.skip_statement()?;
                }
                b"node" => {
                    let mut l = *look;
                    self.node_options(pic, &mut l)?;
                    self.node(d, pic, &mut l, None)?;
                    self.skip_gap();
                    if self.peek() == b';' {
                        self.pos += 1;
                    }
                }
                b"coordinate" => {
                    self.square_options()?;
                    self.skip_gap();
                    let label = self.nested(b'(', b')')?;
                    let label = trim(self.expand_text(label));
                    let label = self.lasting(label);
                    self.skip_gap();
                    if self.word() != b"at" {
                        return self.fail(E_ARG);
                    }
                    self.skip_gap();
                    if self.peek() != b'(' {
                        return self.fail(E_ARG);
                    }
                    let inner = self.nested(b'(', b')')?;
                    let p = self.resolve(pic, look, inner)?;
                    pic.coords.push(Named { name: label, x: p.0, y: p.1, extent: Extent::default() });
                    self.skip_statement()?;
                }
                b"tikzset" => {
                    let text = self.group_text()?;
                    let mut l = *look;
                    self.options(pic, &mut l, text, false, 0)?;
                }
                b"addplot" if pic.axis.is_some() => self.skip_statement()?,
                b"legend" if pic.axis.is_some() => {
                    self.group_text()?;
                }
                b"addlegendentry" if pic.axis.is_some() => {
                    self.group_text()?;
                }
                b"begin" => {
                    let (s, e) = self.raw_group()?;
                    match self.slice(s, e) {
                        b"scope" => {
                            let mut l = *look;
                            let text = self.square_options()?;
                            self.options(pic, &mut l, text, false, 0)?;
                            self.scene(d, pic, l, b"scope")?;
                        }
                        b"axis" => self.axis(d, pic, look)?,
                        _ => {
                            self.note(s);
                            let name = self.slice(s, e);
                            self.skip_options();
                            self.scene(d, pic, *look, name)?;
                        }
                    }
                }
                b"end" => {
                    let (s, e) = self.raw_group()?;
                    if self.slice(s, e) != closing {
                        self.pos = s;
                        return self.fail(E_ENV);
                    }
                    self.restore_defs(pic, mark)?;
                    self.depth -= 1;
                    return Ok(true);
                }
                _ if name.starts_with(b"tkz") => self.tkz_statement(d, pic, look, name)?,
                b"relax" => {}
                b"foreach" => self.scene_foreach(d, pic, *look)?,
                b"pgfmathsetmacro" | b"pgfmathtruncatemacro" | b"pgfmathparse" => self.pgf_math(pic, name)?,
                b"usetikzlibrary" | b"pgfplotsset" | b"tikzstyle" => self.scene_setting(pic, name, *look)?,
                _ if self.conditional(name)? => {}
                _ if super::is_definition(name) => {
                    let mut c = d.ctx;
                    let mut r = self.new_run(0, NONE, c);
                    self.core(name, &mut r, &mut c)?;
                }
                _ => {
                    if let Some(def) = self.find_def(name) {
                        self.enter()?;
                        let body = self.expansion(&def, false)?;
                        let src = self.push_src(body.as_ptr(), body.len() - 1);
                        self.scene(d, pic, *look, b"")?;
                        self.pop_src(src);
                        self.depth -= 1;
                        return Ok(false);
                    }
                    self.note(start);
                    self.skip_loose();
                }
            }
        }
        Ok(false)
    }

    pub(super) fn picture(&mut self, d: &mut Doc, left: i32, width: i32) -> R<()> {
        let mut pic = core::mem::take(&mut self.store.pic);
        pic.coords.clear();
        pic.styles.clear();
        pic.saved.clear();
        pic.every_node = b"";
        pic.path.clear();
        pic.art.clear();
        pic.drawn = false;
        pic.budget = OPTION_BUDGET;
        pic.axis = None;
        pic.line = width as f64 * self.doc.pt as f64 / 1000.0;
        let mut look = first_look();
        for i in 0..self.store.settings.len() {
            let text = self.store.settings[i];
            self.options(&mut pic, &mut look, text, false, 0)?;
        }
        let done = self.picture_run(d, &mut pic, left, width);
        self.store.pic = pic;
        done
    }

    fn picture_run(&mut self, d: &mut Doc, pic: &mut Pic, left: i32, width: i32) -> R<()> {
        let mut look = first_look();
        let text = self.square_options()?;
        self.options(pic, &mut look, text, false, 0)?;
        let s = self.end();
        self.scene(d, pic, look, b"tikzpicture")?;
        let e = self.end();
        let (x0, y0, x1, y1) = pic.bounds;
        if !pic.drawn {
            return Ok(());
        }
        let k = 1000.0 / self.doc.pt as f64;
        let (dx, dy) = (to_units(-x0 * k), to_units(y0 * k));
        let (w, h) = (to_units((x1 - x0) * k), to_units((y1 - y0) * k));
        if let Some(border) = d.standalone {
            self.doc.paper = 0;
            self.doc.w = w.saturating_add(2 * border);
            self.doc.h = h.saturating_add(2 * border);
            d.left = 0;
            d.top = 0;
            d.frame_left = 0;
            d.frame_top = 0;
            let bottom = border.saturating_add(h);
            self.shift(s, e, dx, dy);
            self.commit(d, s, e, border, bottom);
            for i in 0..pic.art.len() {
                let mut a = pic.art[i];
                a.x += dx + border;
                a.y += dy + bottom;
                self.doc.art.push(a);
            }
            return Ok(());
        }
        let y = self.place_line(d, h, 0);
        let x = left + if d.center { ((width - w) / 2).max(0) } else { 0 };
        self.shift(s, e, dx, dy);
        self.commit(d, s, e, x, y);
        for i in 0..pic.art.len() {
            let mut a = pic.art[i];
            a.x += dx + d.left + x;
            a.y += dy + d.top + y;
            self.doc.art.push(a);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

impl Engine {
    fn tkz_statement(&mut self, d: &mut Doc, pic: &mut Pic, look: &Look, name: &[u8]) -> R<()> {
        let start = self.pos;
        let options = self.square_options()?;
        let mut generated = alloc::string::String::new();
        let as_text = |t: &[u8]| -> alloc::string::String { core::str::from_utf8(t).unwrap_or("").into() };
        match name {
            b"tkzDefPoint" => {
                let p = self.nested(b'(', b')')?;
                let label = self.group_text()?;
                generated = alloc::format!("\\coordinate ({}) at ({});", as_text(label), as_text(p));
            }
            b"tkzDefPoints" => {
                let points = self.group_text()?;
                for point in points.split(|c| *c == b',' || c.is_ascii_whitespace()).filter(|p| !p.is_empty()) {
                    let fields: Vec<_> = point.split(|c| *c == b'/').collect();
                    if fields.len() != 3 {
                        self.note(start);
                        continue;
                    }
                    generated.push_str(&alloc::format!(
                        "\\coordinate ({}) at ({},{});",
                        as_text(fields[2]),
                        as_text(fields[0]),
                        as_text(fields[1])
                    ));
                }
            }
            b"tkzDrawSegment" | b"tkzDrawSegments" | b"tkzDrawPolygon" => {
                let points = self.nested(b'(', b')')?;
                let style = as_text(options);
                if name == b"tkzDrawSegments" {
                    for pair in points.split(|c| c.is_ascii_whitespace()).filter(|p| !p.is_empty()) {
                        let fields: Vec<_> = pair.split(|c| *c == b',').collect();
                        if fields.len() == 2 {
                            generated.push_str(&alloc::format!("\\draw[{style}] ({}) -- ({});", as_text(fields[0]), as_text(fields[1])));
                        } else {
                            self.note(start);
                        }
                    }
                } else {
                    generated.push_str(&alloc::format!("\\draw[{style}] "));
                    for (i, p) in points.split(|c| *c == b',').enumerate() {
                        if i > 0 {
                            generated.push_str(" -- ");
                        }
                        generated.push_str(&alloc::format!("({})", as_text(trim(p))));
                    }
                    if name == b"tkzDrawPolygon" {
                        generated.push_str(" -- cycle");
                    }
                    generated.push(';');
                }
            }
            b"tkzDrawPoint" | b"tkzDrawPoints" | b"tkzLabelPoints" => {
                let points = self.nested(b'(', b')')?;
                for label in points.split(|c| *c == b',') {
                    if name == b"tkzLabelPoints" {
                        generated.push_str(&alloc::format!(
                            "\\node[{}] at ({}) {{{}}};",
                            if options.is_empty() { "above".into() } else { as_text(options) },
                            as_text(label),
                            as_text(label)
                        ));
                    } else {
                        generated.push_str(&alloc::format!("\\fill ({}) circle[radius=1.5pt];", as_text(label)));
                    }
                }
            }
            b"tkzLabelPoint" => {
                let p = self.nested(b'(', b')')?;
                let label = self.group_text()?;
                generated = alloc::format!(
                    "\\node[{}] at ({}) {{{}}};",
                    if options.is_empty() { "above".into() } else { as_text(options) },
                    as_text(p),
                    as_text(label)
                );
            }
            b"tkzDrawCircle" | b"tkzDefMidPoint" => {
                let points = self.nested(b'(', b')')?;
                let fields: Vec<_> = points.split(|c| *c == b',').collect();
                if fields.len() != 2 {
                    return self.fail(E_ARG);
                }
                let a = self.resolve(pic, look, trim(fields[0]))?;
                let b = self.resolve(pic, look, trim(fields[1]))?;
                if name == b"tkzDefMidPoint" {
                    pic.coords.push(Named { name: b"_tkz_result", x: (a.0 + b.0) / 2.0, y: (a.1 + b.1) / 2.0, extent: Extent::default() });
                    return Ok(());
                }
                let r = sqrt((a.0 - b.0) * (a.0 - b.0) + (a.1 - b.1) * (a.1 - b.1));
                generated = alloc::format!("\\draw[{}] ({}) circle[radius={}pt];", as_text(options), as_text(fields[0]), r);
            }
            b"tkzGetPoint" => {
                let label = self.group_text()?;
                let label = self.lasting(label);
                let p = self.resolve(pic, look, b"_tkz_result")?;
                pic.coords.push(Named { name: label, x: p.0, y: p.1, extent: Extent::default() });
                return Ok(());
            }
            _ => {
                self.note(start);
                self.skip_arguments();
                return Ok(());
            }
        }
        let kept = self.source_copy(generated.as_bytes());
        let src = self.push_src(kept.as_ptr(), kept.len());
        let done = self.scene(d, pic, *look, b"");
        self.pop_src(src);
        done
    }
}
