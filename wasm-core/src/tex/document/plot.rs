use super::figure::*;
use super::*;

const TICK: f64 = 4.27;
const TICK_GAP: f64 = 3.3;
const LABEL_GAP: f64 = 4.0;
const TITLE_GAP: f64 = 6.0;
const OUTER: f64 = 45.0;
const MAX_TICKS: usize = 40;
const MAX_SAMPLES: f64 = 2000.0;
const LEGEND_LINE: f64 = 17.0;
const LEGEND_PAD: f64 = 3.3;
const MAX_LEGEND: usize = 16;

const MARK_NONE: u8 = 0;
const MARK_DOT: u8 = 1;
const MARK_RING: u8 = 2;
const MARK_SQUARE: u8 = 3;
const MARK_BOX: u8 = 4;
const MARK_TRIANGLE: u8 = 5;
const MARK_DELTA: u8 = 6;
const MARK_CROSS: u8 = 7;
const MARK_PLUS: u8 = 8;

#[derive(Clone, Copy)]
pub(super) struct AxisMap {
    x0: f64,
    y0: f64,
    w: f64,
    h: f64,
    xmin: f64,
    xmax: f64,
    ymin: f64,
    ymax: f64,
}

impl AxisMap {
    pub(super) fn at(&self, x: f64, y: f64) -> (f64, f64) {
        (self.x0 + (x - self.xmin) / (self.xmax - self.xmin) * self.w, self.y0 + (y - self.ymin) / (self.ymax - self.ymin) * self.h)
    }

    pub(super) fn relative(&self, x: f64, y: f64) -> (f64, f64) {
        (self.x0 + x * self.w, self.y0 + y * self.h)
    }
}

#[derive(Clone, Copy)]
pub(super) struct Plot {
    s: u32,
    e: u32,
    pen: Pen,
    mark: u8,
    size: f64,
    lines: bool,
}

struct Axis {
    width: f64,
    height: f64,
    only: bool,
    title: &'static [u8],
    xlabel: &'static [u8],
    ylabel: &'static [u8],
    grid: (bool, bool),
    limits: [Option<f64>; 4],
    legend: ((f64, f64), (i8, i8)),
    tick_size: u16,
    label_size: u16,
    legend_size: u16,
    ticks: [&'static [u8]; 2],
    enlarge: bool,
}

fn mark_value(name: &[u8]) -> Option<u8> {
    Some(match name {
        b"*" => MARK_DOT,
        b"o" => MARK_RING,
        b"square*" => MARK_SQUARE,
        b"square" => MARK_BOX,
        b"triangle*" => MARK_TRIANGLE,
        b"triangle" => MARK_DELTA,
        b"x" => MARK_CROSS,
        b"+" => MARK_PLUS,
        b"none" => MARK_NONE,
        _ => return None,
    })
}

fn tick_step(min: f64, max: f64, length: f64) -> f64 {
    let range = max - min;
    let raw = range / (length / 35.0).max(4.0);
    if !(raw > 0.0) {
        return 1.0;
    }
    let mut unit = 1.0;
    while raw / unit >= 10.0 {
        unit *= 10.0;
    }
    while raw / unit < 1.0 {
        unit /= 10.0;
    }
    let m = raw / unit;
    unit * if m < 1.5 {
        1.0
    } else if m < 2.25 {
        2.0
    } else if m < 3.0 {
        2.5
    } else if m < 7.0 {
        5.0
    } else {
        10.0
    }
}

fn format_number(v: f64, out: &mut [u8; 32]) -> usize {
    let mut n = 0;
    let negative = v < 0.0;
    let size = if negative { -v } else { v };
    let scaled = (size * 10000.0 + 0.5) as u64;
    let (whole, part) = (scaled / 10000, scaled % 10000);
    if negative && scaled != 0 {
        out[n] = b'-';
        n += 1;
    }
    let mut digits = [0u8; 20];
    let mut count = 0;
    let mut w = whole;
    loop {
        digits[count] = b'0' + (w % 10) as u8;
        count += 1;
        w /= 10;
        if w == 0 {
            break;
        }
    }
    while count > 0 {
        count -= 1;
        out[n] = digits[count];
        n += 1;
    }
    if part != 0 {
        out[n] = b'.';
        n += 1;
        let mut p = part;
        let mut place = 1000;
        while p != 0 {
            out[n] = b'0' + (p / place) as u8;
            n += 1;
            p %= place;
            place /= 10;
        }
    }
    n
}

fn clip(mut p: (f64, f64), mut q: (f64, f64), map: &AxisMap) -> Option<((f64, f64), (f64, f64))> {
    let (dx, dy) = (q.0 - p.0, q.1 - p.1);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (d, near, far) in [(dx, p.0 - map.xmin, map.xmax - p.0), (dy, p.1 - map.ymin, map.ymax - p.1)] {
        for (delta, room) in [(-d, near), (d, far)] {
            if delta == 0.0 {
                if room < 0.0 {
                    return None;
                }
            } else {
                let t = room / delta;
                if delta < 0.0 {
                    t0 = t0.max(t);
                } else {
                    t1 = t1.min(t);
                }
            }
        }
    }
    if t0 > t1 {
        return None;
    }
    let start = (p.0 + t0 * dx, p.1 + t0 * dy);
    q = (p.0 + t1 * dx, p.1 + t1 * dy);
    p = start;
    Some((p, q))
}

fn plain(color: u32, width: f64) -> Pen {
    Pen { stroke: color, width, drawn: true, ..first_look().pen }
}

impl Engine {
    fn style_font(&self, text: &'static [u8]) -> u16 {
        let mut at = 0;
        while let Some((key, value)) = next_option(text, &mut at) {
            if key == b"font" {
                let name = value.strip_prefix(b"\\").unwrap_or(value);
                return font_size(name, self.doc.pt).unwrap_or(0);
            }
        }
        0
    }

    fn axis_options(&mut self, pic: &Pic, text: &'static [u8]) -> R<Axis> {
        let mut a = Axis {
            width: 240.0,
            height: 207.0,
            only: false,
            title: b"",
            xlabel: b"",
            ylabel: b"",
            grid: (false, false),
            limits: [None; 4],
            legend: ((0.97, 0.97), (1, 1)),
            tick_size: 0,
            label_size: 0,
            legend_size: 0,
            ticks: [b"", b""],
            enlarge: true,
        };
        let mut at = 0;
        while let Some((key, value)) = next_option(text, &mut at) {
            match key {
                b"width" | b"height" => {
                    let Some((v, _)) = evaluate(value, 0.0, pic.line) else {
                        return self.fail_at(value, E_OPTION);
                    };
                    if key == b"width" {
                        a.width = v;
                    } else {
                        a.height = v;
                    }
                }
                b"scale only axis" => a.only = true,
                b"title" => a.title = value,
                b"xlabel" => a.xlabel = value,
                b"ylabel" => a.ylabel = value,
                b"grid" => {
                    a.grid = if value == b"none" { (false, false) } else { (true, true) };
                }
                b"xmajorgrids" => a.grid.0 = true,
                b"ymajorgrids" => a.grid.1 = true,
                b"xmin" | b"xmax" | b"ymin" | b"ymax" => {
                    let Some((v, _)) = evaluate(value, 0.0, 0.0) else {
                        return self.fail_at(value, E_OPTION);
                    };
                    let i = match key {
                        b"xmin" => 0,
                        b"xmax" => 1,
                        b"ymin" => 2,
                        _ => 3,
                    };
                    a.limits[i] = Some(v);
                }
                b"legend pos" => {
                    a.legend = match value {
                        b"north west" => ((0.03, 0.97), (-1, 1)),
                        b"north east" => ((0.97, 0.97), (1, 1)),
                        b"south west" => ((0.03, 0.03), (-1, -1)),
                        b"south east" => ((0.97, 0.03), (1, -1)),
                        b"outer north east" => ((1.03, 1.0), (-1, 1)),
                        _ => return self.fail_at(value, E_OPTION),
                    };
                }
                b"legend style" => a.legend_size = self.style_font(value),
                b"tick label style" | b"ticklabel style" => a.tick_size = self.style_font(value),
                b"label style" => a.label_size = self.style_font(value),
                b"title style" | b"legend cell align" | b"axis line style" | b"compat" => {}
                b"xtick" => a.ticks[0] = value,
                b"ytick" => a.ticks[1] = value,
                b"enlargelimits" => a.enlarge = value != b"false",
                _ => return self.fail_at(key, E_OPTION),
            }
        }
        if !a.only {
            a.width = (a.width - OUTER).max(20.0);
            a.height = (a.height - OUTER).max(20.0);
        }
        Ok(a)
    }

    fn add_plot(&mut self, pic: &mut Pic, look: &Look, a: &Axis, cycle: &mut usize) -> R<()> {
        let plus = self.peek() == b'+';
        if plus {
            self.pos += 1;
        }
        let text = self.square_options()?;
        let mut l = *look;
        l.pen = plain(0, 0.4);
        let mut plot = Plot { s: 0, e: 0, pen: l.pen, mark: MARK_NONE, size: 2.0, lines: true };
        if text.is_empty() || plus {
            const CYCLE: [(u32, u8); 5] = [(0x0000ff, MARK_DOT), (0xff0000, MARK_SQUARE), (0xbf8040, MARK_TRIANGLE), (0x000000, MARK_RING), (0x008080, MARK_BOX)];
            let (color, mark) = CYCLE[*cycle % CYCLE.len()];
            *cycle += 1;
            l.pen.stroke = color;
            plot.mark = mark;
        }
        let (mut from, mut to, mut samples) = (a.limits[0].unwrap_or(-5.0), a.limits[1].unwrap_or(5.0), 25.0);
        let mut at = 0;
        while let Some((key, value)) = next_option(text, &mut at) {
            match key {
                b"only marks" => {
                    plot.lines = false;
                    if plot.mark == MARK_NONE {
                        plot.mark = MARK_DOT;
                    }
                }
                b"no marks" | b"no markers" => plot.mark = MARK_NONE,
                b"mark" => match mark_value(value) {
                    Some(m) => plot.mark = m,
                    None => return self.fail_at(value, E_OPTION),
                },
                b"mark size" => match length_pt(value) {
                    Some(v) => plot.size = v,
                    None => return self.fail_at(value, E_OPTION),
                },
                b"domain" => {
                    let Some(i) = value.iter().position(|c| *c == b':') else {
                        return self.fail_at(value, E_OPTION);
                    };
                    let (Some((f, _)), Some((t, _))) = (evaluate(trim(&value[..i]), 0.0, 0.0), evaluate(trim(&value[i + 1..]), 0.0, 0.0)) else {
                        return self.fail_at(value, E_OPTION);
                    };
                    from = f;
                    to = t;
                }
                b"samples" => match evaluate(value, 0.0, 0.0) {
                    Some((v, _)) => samples = v.clamp(2.0, MAX_SAMPLES),
                    None => return self.fail_at(value, E_OPTION),
                },
                b"smooth" | b"sharp plot" | b"forget plot" | b"mark options" => {}
                _ => self.option(pic, &mut l, key, value, false, 0)?,
            }
        }
        plot.pen = l.pen;
        plot.s = pic.points.len() as u32;
        self.skip_gap();
        let (body, formula): (&'static [u8], bool) = if self.peek() == b'{' {
            (self.group_text()?, true)
        } else {
            let at = self.pos;
            match self.word() {
                b"coordinates" => (self.group_text()?, false),
                b"expression" => (self.group_text()?, true),
                _ => {
                    self.pos = at;
                    return self.fail(E_OPTION);
                }
            }
        };
        if formula {
            let count = samples as i32;
            for i in 0..count {
                let x = from + (to - from) * i as f64 / (count - 1) as f64;
                let Some((y, _)) = evaluate(trim(body), x, 0.0) else {
                    return self.fail_at(body, E_OPTION);
                };
                if !y.is_nan() {
                    pic.points.push((x, y));
                }
            }
        } else {
            let mut i = 0;
            while i < body.len() {
                if body[i] != b'(' {
                    i += 1;
                    continue;
                }
                let Some(close) = body[i..].iter().position(|c| *c == b')') else {
                    return self.fail_at(body, E_ARG);
                };
                let inner = &body[i + 1..i + close];
                let Some(comma) = inner.iter().position(|c| *c == b',') else {
                    return self.fail_at(inner, E_ARG);
                };
                let (Some((x, _)), Some((y, _))) = (evaluate(trim(&inner[..comma]), 0.0, 0.0), evaluate(trim(&inner[comma + 1..]), 0.0, 0.0)) else {
                    return self.fail_at(inner, E_UNIT);
                };
                pic.points.push((x, y));
                i += close + 1;
            }
        }
        plot.e = pic.points.len() as u32;
        pic.plots.push(plot);
        self.skip_gap();
        if self.peek() != b';' {
            return self.fail(E_MISPLACED);
        }
        self.pos += 1;
        Ok(())
    }

    fn collect_plots(&mut self, pic: &mut Pic, look: &Look, a: &Axis) -> R<()> {
        let mut cycle = 0;
        loop {
            self.skip_gap();
            if self.at_end() {
                return self.fail(E_END);
            }
            if self.peek() != b'\\' {
                return self.fail(E_MISPLACED);
            }
            self.pos += 1;
            match self.read_name() {
                b"addplot" => self.add_plot(pic, look, a, &mut cycle)?,
                b"addlegendentry" => {
                    let text = self.group_text()?;
                    pic.entries.push(text);
                }
                b"end" => return Ok(()),
                _ => self.skip_statement()?,
            }
        }
    }

    fn limits(&self, pic: &Pic, a: &Axis, near: usize, y: bool) -> (f64, f64) {
        let (mut min, mut max) = (f64::INFINITY, f64::NEG_INFINITY);
        for p in &pic.points {
            let v = if y { p.1 } else { p.0 };
            if v.is_finite() {
                min = min.min(v);
                max = max.max(v);
            }
        }
        if min > max {
            min = 0.0;
            max = 1.0;
        }
        let pad = if a.enlarge { (max - min) * 0.1 } else { 0.0 };
        let mut low = a.limits[near].unwrap_or(min - pad);
        let mut high = a.limits[near + 1].unwrap_or(max + pad);
        if !(high > low) {
            low -= 1.0;
            high = low + 2.0;
        }
        (low, high)
    }

    fn tick_values(&mut self, text: &'static [u8], min: f64, max: f64, length: f64, out: &mut [f64; MAX_TICKS]) -> R<usize> {
        let mut count = 0;
        if !text.is_empty() {
            for part in text.split(|c| *c == b',') {
                let Some((v, _)) = evaluate(trim(part), 0.0, 0.0) else {
                    return self.fail_at(text, E_OPTION);
                };
                if count < MAX_TICKS {
                    out[count] = v;
                    count += 1;
                }
            }
            return Ok(count);
        }
        let step = tick_step(min, max, length);
        let first = -floor(-(min / step));
        let mut i = 0.0;
        while count < MAX_TICKS {
            let mut v = (first + i) * step;
            if v > max + step * 1e-9 {
                break;
            }
            if v.abs() < step * 1e-9 {
                v = 0.0;
            }
            out[count] = v;
            count += 1;
            i += 1.0;
        }
        Ok(count)
    }

    fn segment(&mut self, pic: &mut Pic, pen: &Pen, p: (f64, f64), q: (f64, f64)) {
        pic.path.push((SEG_MOVE, p.0, p.1));
        pic.path.push((SEG_LINE, q.0, q.1));
        self.stroke_path(pic, pen);
    }

    fn mark(&mut self, pic: &mut Pic, plot: &Plot, p: (f64, f64)) {
        let r = plot.size;
        let mut pen = Pen { dash: (0.0, 0.0), drawn: true, fill: NO_COLOR, solid: false, ..plot.pen };
        pen.width = 0.4;
        pen.start = first_look().pen.start;
        pen.end = first_look().pen.end;
        match plot.mark {
            MARK_DOT | MARK_RING => {
                if plot.mark == MARK_DOT {
                    pen.solid = true;
                } else {
                    pen.fill = 0xffffff;
                }
                let look = first_look();
                pic.path.push((SEG_MOVE, p.0 + r, p.1));
                self.sweep(pic, &look, p, (r, r), 0.0, 360.0);
                pic.path.push((SEG_CLOSE, p.0, p.1));
            }
            MARK_SQUARE | MARK_BOX | MARK_TRIANGLE | MARK_DELTA => {
                let square = matches!(plot.mark, MARK_SQUARE | MARK_BOX);
                if matches!(plot.mark, MARK_SQUARE | MARK_TRIANGLE) {
                    pen.solid = true;
                } else {
                    pen.fill = 0xffffff;
                }
                if square {
                    pic.path.push((SEG_MOVE, p.0 - r, p.1 - r));
                    pic.path.push((SEG_LINE, p.0 + r, p.1 - r));
                    pic.path.push((SEG_LINE, p.0 + r, p.1 + r));
                    pic.path.push((SEG_LINE, p.0 - r, p.1 + r));
                } else {
                    pic.path.push((SEG_MOVE, p.0, p.1 + r));
                    pic.path.push((SEG_LINE, p.0 + r, p.1 - r));
                    pic.path.push((SEG_LINE, p.0 - r, p.1 - r));
                }
                pic.path.push((SEG_CLOSE, p.0, p.1));
            }
            MARK_CROSS => {
                pic.path.push((SEG_MOVE, p.0 - r, p.1 - r));
                pic.path.push((SEG_LINE, p.0 + r, p.1 + r));
                pic.path.push((SEG_MOVE, p.0 - r, p.1 + r));
                pic.path.push((SEG_LINE, p.0 + r, p.1 - r));
            }
            MARK_PLUS => {
                pic.path.push((SEG_MOVE, p.0 - r, p.1));
                pic.path.push((SEG_LINE, p.0 + r, p.1));
                pic.path.push((SEG_MOVE, p.0, p.1 - r));
                pic.path.push((SEG_LINE, p.0, p.1 + r));
            }
            _ => return,
        }
        self.stroke_path(pic, &pen);
    }

    fn draw_plot(&mut self, pic: &mut Pic, plot: &Plot, map: &AxisMap) {
        let (s, e) = (plot.s as usize, plot.e as usize);
        if plot.lines {
            let mut open = false;
            for i in s + 1..e {
                match clip(pic.points[i - 1], pic.points[i], map) {
                    Some((a, b)) => {
                        let (from, to) = (map.at(a.0, a.1), map.at(b.0, b.1));
                        if !open {
                            pic.path.push((SEG_MOVE, from.0, from.1));
                        }
                        pic.path.push((SEG_LINE, to.0, to.1));
                        open = b == pic.points[i];
                        if !open {
                            let pen = plot.pen;
                            self.stroke_path(pic, &pen);
                        }
                    }
                    None => open = false,
                }
            }
            let pen = plot.pen;
            self.stroke_path(pic, &pen);
        }
        if plot.mark != MARK_NONE {
            for i in s..e {
                let (x, y) = pic.points[i];
                if x >= map.xmin && x <= map.xmax && y >= map.ymin && y <= map.ymax {
                    self.mark(pic, plot, map.at(x, y));
                }
            }
        }
    }

    fn text_box(&mut self, d: &mut Doc, c: &Ctx, text: &'static [u8]) -> R<Bx> {
        self.sub_text(d, text, c, 0)?;
        Ok(self.line_box(d, c))
    }

    fn number_box(&mut self, c: &Ctx, v: f64) -> R<Bx> {
        let mut text = [0u8; 32];
        let n = format_number(v, &mut text);
        self.list_in(text.as_ptr(), n, *c)
    }

    fn draw_legend(&mut self, d: &mut Doc, pic: &mut Pic, look: &Look, a: &Axis, map: &AxisMap) -> R<()> {
        let count = pic.entries.len().min(pic.plots.len()).min(MAX_LEGEND);
        if count == 0 {
            return Ok(());
        }
        let c = self.text_ctx(d, a.legend_size, 0);
        let pt = self.doc.pt as f64 / 1000.0;
        let mut boxes = [Bx::default(); MAX_LEGEND];
        let (mut text_w, mut row) = (0.0f64, 0.0f64);
        for i in 0..count {
            boxes[i] = self.text_box(d, &c, pic.entries[i])?;
            text_w = text_w.max(boxes[i].w as f64 * pt);
            row = row.max((boxes[i].h + boxes[i].d) as f64 * pt);
        }
        row += 2.0;
        let w = LEGEND_LINE + LEGEND_PAD * 3.0 + text_w;
        let h = row * count as f64 + LEGEND_PAD * 2.0;
        let (at, anchor) = a.legend;
        let origin = map.relative(at.0, at.1);
        let left = origin.0 - (anchor.0 as f64 + 1.0) / 2.0 * w;
        let top = origin.1 + (1.0 - anchor.1 as f64) / 2.0 * h;
        let frame = Pen { fill: 0xffffff, ..plain(0, 0.4) };
        pic.path.push((SEG_MOVE, left, top));
        pic.path.push((SEG_LINE, left + w, top));
        pic.path.push((SEG_LINE, left + w, top - h));
        pic.path.push((SEG_LINE, left, top - h));
        pic.path.push((SEG_CLOSE, left, top));
        self.stroke_path(pic, &frame);
        for i in 0..count {
            let plot = pic.plots[i];
            let y = top - LEGEND_PAD - row * (i as f64 + 0.5);
            let x = left + LEGEND_PAD;
            if plot.lines {
                self.segment(pic, &plot.pen, (x, y), (x + LEGEND_LINE, y));
            }
            self.mark(pic, &plot, (x + LEGEND_LINE / 2.0, y));
            self.label(pic, look, boxes[i], (x + LEGEND_LINE + LEGEND_PAD, y), (-1, 0), 0.0);
        }
        Ok(())
    }

    pub(super) fn axis(&mut self, d: &mut Doc, pic: &mut Pic, look: &Look) -> R<()> {
        let text = self.square_options()?;
        let a = self.axis_options(pic, text)?;
        let body = self.pos;
        pic.points.clear();
        pic.plots.clear();
        pic.entries.clear();
        self.collect_plots(pic, look, &a)?;
        let (xmin, xmax) = self.limits(pic, &a, 0, false);
        let (ymin, ymax) = self.limits(pic, &a, 2, true);
        let origin = (0.0, 0.0);
        let map = AxisMap { x0: origin.0, y0: origin.1, w: a.width, h: a.height, xmin, xmax, ymin, ymax };
        let mut xs = [0.0; MAX_TICKS];
        let mut ys = [0.0; MAX_TICKS];
        let nx = self.tick_values(a.ticks[0], xmin, xmax, a.width, &mut xs)?;
        let ny = self.tick_values(a.ticks[1], ymin, ymax, a.height, &mut ys)?;
        let grid = plain(0xbfbfbf, 0.4);
        let frame = plain(0, 0.4);
        let (right, top) = (map.x0 + map.w, map.y0 + map.h);
        for &x in &xs[..nx] {
            let p = map.at(x, ymin);
            if a.grid.0 {
                self.segment(pic, &grid, (p.0, map.y0), (p.0, top));
            }
        }
        for &y in &ys[..ny] {
            let p = map.at(xmin, y);
            if a.grid.1 {
                self.segment(pic, &grid, (map.x0, p.1), (right, p.1));
            }
        }
        for i in 0..pic.plots.len() {
            let plot = pic.plots[i];
            self.draw_plot(pic, &plot, &map);
        }
        pic.path.push((SEG_MOVE, map.x0, map.y0));
        pic.path.push((SEG_LINE, right, map.y0));
        pic.path.push((SEG_LINE, right, top));
        pic.path.push((SEG_LINE, map.x0, top));
        pic.path.push((SEG_CLOSE, map.x0, map.y0));
        self.stroke_path(pic, &frame);
        let tick = self.text_ctx(d, a.tick_size, 0);
        let label = self.text_ctx(d, a.label_size, 0);
        let pt = self.doc.pt as f64 / 1000.0;
        let (mut tall, mut wide) = (0.0f64, 0.0f64);
        for &x in &xs[..nx] {
            let p = map.at(x, ymin);
            self.segment(pic, &frame, (p.0, map.y0), (p.0, map.y0 + TICK));
            self.segment(pic, &frame, (p.0, top), (p.0, top - TICK));
            let b = self.number_box(&tick, x)?;
            tall = tall.max((b.h + b.d) as f64 * pt);
            self.label(pic, look, b, (p.0, map.y0 - TICK_GAP), (0, 1), 0.0);
        }
        for &y in &ys[..ny] {
            let p = map.at(xmin, y);
            self.segment(pic, &frame, (map.x0, p.1), (map.x0 + TICK, p.1));
            self.segment(pic, &frame, (right, p.1), (right - TICK, p.1));
            let b = self.number_box(&tick, y)?;
            wide = wide.max(b.w as f64 * pt);
            self.label(pic, look, b, (map.x0 - TICK_GAP, p.1), (1, 0), 0.0);
        }
        let middle = (map.x0 + map.w / 2.0, map.y0 + map.h / 2.0);
        if !a.title.is_empty() {
            let b = self.text_box(d, &label, a.title)?;
            self.label(pic, look, b, (middle.0, top + TITLE_GAP), (0, -1), 0.0);
        }
        if !a.xlabel.is_empty() {
            let b = self.text_box(d, &label, a.xlabel)?;
            self.label(pic, look, b, (middle.0, map.y0 - TICK_GAP - tall - LABEL_GAP), (0, 1), 0.0);
        }
        if !a.ylabel.is_empty() {
            let b = self.text_box(d, &label, a.ylabel)?;
            self.label(pic, look, b, (map.x0 - TICK_GAP - wide - LABEL_GAP, middle.1), (0, -1), 90.0);
        }
        self.draw_legend(d, pic, look, &a, &map)?;
        self.pos = body;
        pic.axis = Some(map);
        let done = self.scene(d, pic, *look, b"axis");
        pic.axis = None;
        done
    }
}

#[cfg(test)]
mod tests;
