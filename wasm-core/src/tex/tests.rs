use super::Engine;

mod errors;
mod geometry;
mod macros;
mod packages;
mod robust;
mod syntax;
mod tables;

pub(crate) type Rendered = Result<String, (String, usize)>;

pub(crate) fn feed(e: &mut Engine, s: &str) -> usize {
    let p = e.input_buffer(s.len());
    unsafe { core::ptr::copy_nonoverlapping(s.as_ptr(), p, s.len()) };
    s.len()
}

pub(crate) fn output(e: &Engine, size: usize) -> String {
    let bytes = unsafe { core::slice::from_raw_parts(e.output_ptr(), size) };
    String::from_utf8_lossy(bytes).into_owned()
}

pub(crate) fn render_with(e: &mut Engine, s: &str, display: bool) -> Rendered {
    let n = feed(e, s);
    let size = e.render(n, display);
    if size >= 0 {
        Ok(output(e, size as usize))
    } else {
        Err((output(e, (-size) as usize), e.error_pos()))
    }
}

pub(crate) fn inline(s: &str) -> Rendered {
    render_with(&mut Engine::new(), s, false)
}

pub(crate) fn display(s: &str) -> Rendered {
    render_with(&mut Engine::new(), s, true)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Size {
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) depth: i32,
}

fn attribute<'a>(svg: &'a str, name: &str) -> &'a str {
    let key = format!("{name}=\"");
    let start = svg.find(&key).unwrap_or_else(|| panic!("no {name} in {svg}")) + key.len();
    let end = svg[start..].find('"').unwrap() + start;
    &svg[start..end]
}

pub(crate) fn size_of(svg: &str) -> Size {
    let box_: Vec<i32> = attribute(svg, "viewBox").split(' ').map(|v| v.parse().unwrap()).collect();
    Size { width: box_[2], height: -box_[1], depth: box_[3] + box_[1] }
}

pub(crate) fn measure(s: &str) -> Size {
    size_of(&inline(s).unwrap_or_else(|e| panic!("{s}: {e:?}")))
}

pub(crate) fn measure_display(s: &str) -> Size {
    size_of(&display(s).unwrap_or_else(|e| panic!("{s}: {e:?}")))
}

pub(crate) fn paths(svg: &str) -> usize {
    svg.matches("<path").count()
}

pub(crate) fn well_formed(svg: &str) -> Result<(), String> {
    if !svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"") {
        return Err("bad header".into());
    }
    if !svg.trim_end().ends_with("</svg>") {
        return Err("no closing tag".into());
    }
    if svg.matches('"').count() % 2 != 0 {
        return Err("unbalanced quotes".into());
    }
    if svg.matches("<g").count() != svg.matches("</g>").count() {
        return Err("unbalanced groups".into());
    }
    if svg.contains("NaN") || svg.contains("inf") {
        return Err("non finite number".into());
    }
    let open = svg.matches('<').count();
    let shut = svg.matches('>').count();
    if open != shut {
        return Err(format!("{open} tags opened, {shut} closed"));
    }
    let mut at = 0;
    while let Some(found) = svg[at..].find(" d=\"") {
        let start = at + found + 4;
        let end = start + svg[start..].find('"').ok_or("unterminated path")?;
        let data = &svg[start..end];
        if let Some(c) = data.chars().find(|c| !"MmLlHhVvCcSsQqTtZz0123456789 ,.-".contains(*c)) {
            return Err(format!("bad path character {c:?}"));
        }
        at = end;
    }
    Ok(())
}

pub(crate) fn check<T: core::fmt::Debug>(rows: &[T], mut test: impl FnMut(&T) -> Result<(), String>) {
    let failures: Vec<String> = rows.iter().filter_map(|row| test(row).err().map(|e| format!("{row:?}: {e}"))).collect();
    assert!(failures.is_empty(), "{} of {} rows failed\n{}", failures.len(), rows.len(), failures.join("\n"));
}

pub(crate) fn valid(s: &str) -> Result<(), String> {
    for rendered in [inline(s), display(s)] {
        let svg = rendered.map_err(|e| format!("{e:?}"))?;
        well_formed(&svg)?;
    }
    Ok(())
}

pub(crate) fn same(a: &str, b: &str) -> Result<(), String> {
    let (x, y) = (display(a).map_err(|e| format!("left {e:?}"))?, display(b).map_err(|e| format!("right {e:?}"))?);
    if x == y { Ok(()) } else { Err(format!("differ: {:?} vs {:?}", size_of(&x), size_of(&y))) }
}

pub(crate) struct Rng(pub(crate) u64);

impl Rng {
    pub(crate) fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    pub(crate) fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

pub(crate) fn boundary(s: &str, mut at: usize) -> usize {
    at = at.min(s.len());
    while !s.is_char_boundary(at) {
        at -= 1;
    }
    at
}

pub(crate) fn mutate(rng: &mut Rng, base: &str, tokens: &[&str]) -> String {
    let mut s = base.to_string();
    for _ in 0..1 + rng.below(4) {
        let at = boundary(&s, rng.below(s.len() + 1));
        match rng.below(5) {
            0 => s.insert_str(at, tokens[rng.below(tokens.len())]),
            1 => {
                let end = boundary(&s, at + rng.below(24));
                s.replace_range(at..end, "");
            }
            2 => {
                let end = boundary(&s, at + rng.below(12));
                let copy = s[at..end].to_string();
                let to = boundary(&s, rng.below(s.len() + 1));
                s.insert_str(to, &copy);
            }
            3 => s.truncate(at),
            _ => {
                let end = boundary(&s, at + 1);
                s.replace_range(at..end, tokens[rng.below(tokens.len())]);
            }
        }
    }
    s
}
