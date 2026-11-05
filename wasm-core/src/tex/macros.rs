use super::*;
use crate::packages::*;
use alloc::vec::Vec;

const NOVALUE: &[u8] = b"-NoValue-";
const MS: &[u8] = b"mmmmmmmmm";
const OS: &[u8] = b"Dmmmmmmmm";
const MAX_EXPANSIONS: u32 = 400_000;
const MAX_EXPANSION_BYTES: usize = 256 << 20;
const MAX_DEFS: usize = 900;
const DEF_MASK: usize = DEF_SLOTS - 1;

#[derive(Clone, Copy)]
pub(super) struct Def {
    pub(super) name: &'static [u8],
    pub(super) spec: &'static [u8],
    pub(super) default: &'static [u8],
    pub(super) body: &'static [u8],
    pub(super) native: Native,
}

impl Engine {
    pub(super) fn reset_macros(&mut self) {
        if !self.defs.is_empty() {
            self.defs.clear();
            self.def_hash = [0; DEF_SLOTS];
            while self.synth.pop().is_some() {}
        }
        self.pkgs = 0;
        self.expansions = 0;
        self.exp_bytes = 0;
        self.color_defs.clear();
        let mut m = self.session;
        while m != 0 {
            let i = m.trailing_zeros() as usize;
            m &= m - 1;
            let _ = self.load_package(i);
        }
    }

    pub fn command_list(&mut self) -> i32 {
        self.out.clear();
        for entry in ENTRIES.iter() {
            let name = &NAMES[entry.off as usize..][..entry.len as usize];
            if name.is_empty() || !name.iter().all(u8::is_ascii_alphabetic) {
                continue;
            }
            let kind = entry.sym.kind;
            self.out.extend_from_slice(name);
            self.out.extend_from_slice(&[b' ', b'0' + kind / 10, b'0' + kind % 10, b' ']);
            self.out.push(b'0' + (entry.sym.flags & F_TEXT != 0) as u8 + 2 * (entry.sym.flags & F_TEXTONLY != 0) as u8);
            self.out.push(b'\n');
        }
        for package in PACKAGES {
            self.out.push(b'=');
            self.out.extend_from_slice(package.name.as_bytes());
            self.out.push(b'\n');
            let defs = package.defs.iter().map(|d| (d.0, d.1));
            let natives = package.natives.iter().map(|n| (n.0, n.1));
            for (name, spec) in defs.chain(natives) {
                self.out.extend_from_slice(name.as_bytes());
                self.out.push(b' ');
                self.out.extend_from_slice(spec.as_bytes());
                self.out.push(b'\n');
            }
        }
        for name in NOOPS {
            self.out.push(b'+');
            self.out.extend_from_slice(name.as_bytes());
            self.out.push(b'\n');
        }
        self.out.len() as i32
    }

    pub fn use_session_package(&mut self, len: usize) -> i32 {
        let name = unsafe { core::slice::from_raw_parts(self.input_ptr(), len) };
        if let Some(i) = find_package(name) {
            self.session |= 1 << i;
            0
        } else if is_noop(name) {
            0
        } else {
            -1
        }
    }

    #[inline]
    pub(super) fn find_def(&self, name: &[u8]) -> Option<Def> {
        let mut i = hash_name(name) & DEF_MASK;
        loop {
            let e = self.def_hash[i];
            if e == 0 {
                return None;
            }
            let d = &self.defs[e as usize - 1];
            if d.name == name {
                return Some(*d);
            }
            i = (i + 1) & DEF_MASK;
        }
    }

    pub(super) fn add_def(&mut self, d: Def) -> R<()> {
        let mut i = hash_name(d.name) & DEF_MASK;
        loop {
            let e = self.def_hash[i];
            if e == 0 {
                break;
            }
            if self.defs[e as usize - 1].name == d.name {
                self.defs[e as usize - 1] = d;
                return Ok(());
            }
            i = (i + 1) & DEF_MASK;
        }
        if self.defs.len() >= MAX_DEFS {
            return self.fail(E_DEPTH);
        }
        self.defs.push(d);
        self.def_hash[i] = self.defs.len() as u16;
        Ok(())
    }

    pub(super) fn define(&mut self, name: &'static [u8], spec: &'static [u8], default: &'static [u8], body: &'static [u8]) -> R<()> {
        self.add_def(Def { name, spec, default, body, native: Native::None })
    }

    fn defined(&self, name: &[u8]) -> bool {
        self.find_def(name).is_some() || lookup(name).is_some()
    }

    pub(super) fn load_package(&mut self, i: usize) -> R<()> {
        if self.pkgs & (1 << i) != 0 {
            return Ok(());
        }
        self.pkgs |= 1 << i;
        let p = &PACKAGES[i];
        for &(name, spec, body) in p.defs {
            self.define(name.as_bytes(), spec.as_bytes(), b"", body.as_bytes())?;
        }
        for &(name, spec, native) in p.natives {
            self.add_def(Def { name: name.as_bytes(), spec: spec.as_bytes(), default: b"", body: b"", native })?;
        }
        Ok(())
    }

    fn use_packages(&mut self) -> R<()> {
        self.skip_ws();
        if self.peek() == b'[' {
            self.bracketed()?;
        }
        self.skip_ws();
        if self.peek() != b'{' {
            return self.fail(E_ARG);
        }
        let list = self.balanced()?;
        for part in list.split(|b| *b == b',') {
            let name = part.trim_ascii();
            if name.is_empty() || is_noop(name) {
                continue;
            }
            match find_package(name) {
                Some(i) => self.load_package(i)?,
                None => return self.fail(E_PACKAGE),
            }
        }
        Ok(())
    }

    fn scan(&mut self, open: u8, close: u8, stop: u8) -> R<&'static [u8]> {
        self.pos += 1;
        let s = self.pos;
        let mut depth = 0;
        loop {
            if self.at_end() {
                return self.fail(E_END);
            }
            let ch = self.peek();
            if ch == b'\\' {
                self.pos += 2;
                continue;
            }
            if ch == stop && depth == 0 {
                let e = self.pos;
                self.pos += 1;
                return Ok(self.slice(s, e));
            }
            if ch == open {
                depth += 1;
            } else if ch == close {
                depth -= 1;
            }
            self.pos += 1;
        }
    }

    pub(super) fn balanced(&mut self) -> R<&'static [u8]> {
        self.scan(b'{', b'}', b'}')
    }

    pub(super) fn bracketed(&mut self) -> R<&'static [u8]> {
        self.scan(b'{', b'}', b']')
    }

    fn macro_arg(&mut self) -> R<&'static [u8]> {
        self.skip_ws();
        let s = self.pos;
        let ch = self.peek();
        if self.at_end() {
            return self.fail(E_ARG);
        }
        match ch {
            b'{' => self.balanced(),
            b'\\' => {
                self.pos += 1;
                self.read_name();
                Ok(self.slice(s, self.pos))
            }
            _ if ch >= 0x80 => {
                let (b, _) = self.read_utf8()?;
                Ok(b)
            }
            _ => {
                self.pos += 1;
                Ok(self.slice(s, self.pos))
            }
        }
    }

    fn def_name(&mut self) -> R<&'static [u8]> {
        self.skip_ws();
        let braced = self.peek() == b'{';
        if braced {
            self.pos += 1;
            self.skip_ws();
        }
        if self.peek() != b'\\' {
            return self.fail(E_ARG);
        }
        self.pos += 1;
        let n = self.read_name();
        if braced {
            self.skip_ws();
            if self.peek() != b'}' {
                return self.fail(E_ARG);
            }
            self.pos += 1;
        }
        Ok(n)
    }

    fn require_source(&mut self) -> R<()> {
        if self.user_src || !self.assets.active.is_empty() { Ok(()) } else { self.fail(E_MISPLACED) }
    }

    pub(super) fn skip_star(&mut self) -> bool {
        if self.peek() == b'*' {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn new_command(&mut self, provide: bool) -> R<()> {
        self.require_source()?;
        self.skip_star();
        let name = self.def_name()?;
        self.skip_ws();
        let mut n = 0usize;
        if self.peek() == b'[' {
            let b = self.bracketed()?;
            if b.len() != 1 || !b[0].is_ascii_digit() {
                return self.fail(E_ARG);
            }
            n = (b[0] - b'0') as usize;
        }
        self.skip_ws();
        let mut spec = &MS[..n];
        let mut default: &'static [u8] = b"";
        if self.peek() == b'[' {
            default = self.bracketed()?;
            if n == 0 {
                return self.fail(E_ARG);
            }
            spec = &OS[..n];
        }
        let body = self.macro_arg()?;
        if provide && self.defined(name) {
            return Ok(());
        }
        self.define(name, spec, default, body)
    }

    fn tex_def(&mut self) -> R<()> {
        self.require_source()?;
        self.skip_ws();
        if self.peek() != b'\\' {
            return self.fail(E_ARG);
        }
        self.pos += 1;
        let name = self.read_name();
        let mut n = 0usize;
        while self.peek() == b'#' {
            self.pos += 1;
            if !self.peek().is_ascii_digit() {
                return self.fail(E_ARG);
            }
            self.pos += 1;
            n += 1;
        }
        if n > 9 {
            return self.fail(E_ARG);
        }
        self.skip_ws();
        if self.peek() != b'{' {
            return self.fail(E_ARG);
        }
        let body = self.balanced()?;
        self.define(name, &MS[..n], b"", body)
    }

    fn keep(&mut self, b: Vec<u8>) -> &'static [u8] {
        self.synth.push(b);
        let v = &self.synth[self.synth.len() - 1];
        unsafe { core::slice::from_raw_parts(v.as_ptr(), v.len()) }
    }

    fn math_operator(&mut self) -> R<()> {
        self.require_source()?;
        let star = self.skip_star();
        let name = self.def_name()?;
        let text = self.macro_arg()?;
        let mut b = Vec::with_capacity(text.len() + 18);
        b.extend_from_slice(if star { b"\\operatorname*{" } else { b"\\operatorname{" });
        b.extend_from_slice(text);
        b.push(b'}');
        let body = self.keep(b);
        self.define(name, b"", b"", body)
    }

    fn paired_delimiter(&mut self) -> R<()> {
        self.require_source()?;
        self.skip_star();
        let name = self.def_name()?;
        let l = self.macro_arg()?;
        let r = self.macro_arg()?;
        let mut b = Vec::with_capacity(96 + l.len() * 3 + r.len() * 3);
        let mut add = |parts: &[&[u8]]| {
            for p in parts {
                b.extend_from_slice(p);
            }
        };
        add(&[b"\\IfBooleanTF{#1}{\\left", l, b" #3\\right", r, b"}{\\IfNoValueTF{#2}{\\mathopen{", l, b"}#3\\mathclose{", r]);
        add(&[b"}}{\\mathopen{#2", l, b"}#3\\mathclose{#2", r, b"}}}"]);
        let body = self.keep(b);
        self.define(name, b"som", b"", body)
    }

    fn document_command(&mut self, provide: bool) -> R<()> {
        self.require_source()?;
        let name = self.def_name()?;
        let spec = self.macro_arg()?;
        let body = self.macro_arg()?;
        if provide && self.defined(name) {
            return Ok(());
        }
        self.define(name, spec, b"", body)
    }

    fn define_color(&mut self) -> R<()> {
        self.require_source()?;
        let name = self.macro_arg()?;
        let model = self.macro_arg()?;
        let value = self.macro_arg()?;
        let mut hex = [0u8; 3];
        match model {
            b"HTML" | b"html" => {
                if value.len() != 6 || !value.iter().all(|c| c.is_ascii_hexdigit()) {
                    return self.fail(E_COLOR);
                }
                let v = |c: u8| (c as char).to_digit(16).unwrap_or(0) as u8;
                for i in 0..3 {
                    hex[i] = v(value[i * 2]) * 16 + v(value[i * 2 + 1]);
                }
            }
            b"rgb" | b"RGB" => {
                let mut i = 0;
                for part in value.split(|c| *c == b',') {
                    if i == 3 {
                        return self.fail(E_COLOR);
                    }
                    let Some(n) = parse_milli(part.trim_ascii()) else {
                        return self.fail(E_COLOR);
                    };
                    let v = if model == b"rgb" { n * 255 / 1000 } else { n / 1000 };
                    hex[i] = v.clamp(0, 255) as u8;
                    i += 1;
                }
                if i != 3 {
                    return self.fail(E_COLOR);
                }
            }
            _ => return self.fail(E_COLOR),
        }
        const H: &[u8; 16] = b"0123456789abcdef";
        let mut text = [b'#'; 7];
        for i in 0..3 {
            text[1 + i * 2] = H[(hex[i] >> 4) as usize];
            text[2 + i * 2] = H[(hex[i] & 15) as usize];
        }
        let id = self.intern_color(&text)?;
        self.color_defs.push((name, id));
        Ok(())
    }

    fn branch(&mut self, name: &[u8], r: &mut Run, c: &mut Ctx) -> R<()> {
        let v = self.macro_arg()?;
        let boolean = name.starts_with(b"IfBoolean");
        let hit = if boolean { v == b"1" } else { v == NOVALUE };
        let (yes, no): (&[u8], &[u8]) = match &name[b"IfBoolean".len()..] {
            b"TF" => (self.macro_arg()?, self.macro_arg()?),
            b"T" => (self.macro_arg()?, b""),
            _ => (b"", self.macro_arg()?),
        };
        let pick = if hit { yes } else { no };
        if pick.is_empty() {
            return Ok(());
        }
        let mut b = Vec::with_capacity(pick.len() + 1);
        b.extend_from_slice(pick);
        b.push(0);
        self.enter()?;
        self.run_bytes(b.as_ptr(), pick.len(), r, c)?;
        self.depth -= 1;
        Ok(())
    }

    pub(super) fn core(&mut self, name: &[u8], r: &mut Run, c: &mut Ctx) -> R<bool> {
        match name {
            b"usepackage" | b"RequirePackage" => self.use_packages()?,
            b"newcommand" | b"renewcommand" => self.new_command(false)?,
            b"providecommand" => self.new_command(true)?,
            b"def" => self.tex_def()?,
            b"DeclareMathOperator" => self.math_operator()?,
            b"DeclarePairedDelimiter" => self.paired_delimiter()?,
            b"NewDocumentCommand" | b"RenewDocumentCommand" | b"DeclareDocumentCommand" => self.document_command(false)?,
            b"ProvideDocumentCommand" => self.document_command(true)?,
            b"definecolor" => self.define_color()?,
            b"IfNoValueTF" | b"IfNoValueT" | b"IfNoValueF" | b"IfBooleanTF" | b"IfBooleanT" | b"IfBooleanF" => self.branch(name, r, c)?,
            _ => {
                let Some(d) = self.autoload(name)? else {
                    return Ok(false);
                };
                self.expand_into(r, c, &d)?;
            }
        }
        Ok(true)
    }

    pub(super) fn autoload(&mut self, name: &[u8]) -> R<Option<Def>> {
        if self.pkgs & (1 << AMSMATH) == 0 {
            self.load_package(AMSMATH)?;
            return Ok(self.find_def(name));
        }
        Ok(None)
    }

    pub(super) fn expansion(&mut self, d: &Def, text: bool) -> R<Vec<u8>> {
        self.expansions += 1;
        if self.expansions > MAX_EXPANSIONS {
            return self.fail(E_DEPTH);
        }
        let mut args: [&'static [u8]; 9] = [b""; 9];
        let spec = d.spec;
        let mut i = 0;
        let mut n = 0;
        while i < spec.len() && n < 9 {
            let k = spec[i];
            i += 1;
            let mut default = if k == b'D' { d.default } else { NOVALUE };
            if k == b'O' {
                if spec.get(i) != Some(&b'{') {
                    return self.fail(E_ARG);
                }
                let s = i + 1;
                let e = closing_brace(spec, s);
                default = &spec[s..e];
                i = (e + 1).min(spec.len());
            }
            args[n] = match k {
                b'm' => self.macro_arg()?,
                b's' => {
                    self.skip_ws();
                    if self.skip_star() { b"1" } else { b"0" }
                }
                b'o' | b'O' | b'D' => {
                    self.skip_ws();
                    if self.peek() == b'[' { self.bracketed()? } else { default }
                }
                b'g' => {
                    self.skip_ws();
                    if self.peek() == b'{' { self.balanced()? } else { NOVALUE }
                }
                b' ' => continue,
                _ => return self.fail(E_ARG),
            };
            n += 1;
        }
        let mut out = Vec::with_capacity(d.body.len() + 64);
        match d.native {
            Native::Rewrite(rewrite, slot) => rewrite(args[slot], &mut out),
            Native::Delimited { matrix } => self.delimited(matrix, &mut out)?,
            _ => substitute(d.body, &args, &mut out),
        }
        self.exp_bytes += out.len();
        if self.exp_bytes > MAX_EXPANSION_BYTES {
            return self.fail(E_DEPTH);
        }
        if text {
            out.push(b'}');
        }
        out.push(0);
        Ok(out)
    }

    pub(super) fn run_bytes(&mut self, p: *const u8, len: usize, r: &mut Run, c: &mut Ctx) -> R<()> {
        let src = self.push_src(p, len);
        let track = r.track;
        r.track = true;
        let before = r.count;
        self.run(r, c, S_MACRO)?;
        r.track = track;
        self.pop_src(src);
        if r.count > before && r.x == r.end_x && self.end() == r.last.bx.e {
            self.skip_ws();
            let ch = self.peek();
            if ch == b'^' || ch == b'_' || ch == b'\'' {
                let a = r.last;
                self.shift(a.bx.s, a.bx.e, -r.at_x, 0);
                r.x = r.x0;
                r.prev = r.prev0;
                r.pp = r.pp0;
                r.gap = r.gap0;
                r.prev_s = r.prev_s0;
                r.count -= 1;
                let a = self.scripts(c, a)?;
                self.place(r, c, a);
            }
        }
        Ok(())
    }

    pub(super) fn expand_into(&mut self, r: &mut Run, c: &mut Ctx, d: &Def) -> R<()> {
        self.enter()?;
        if let Native::Strike { slash, backslash } = d.native {
            let a = self.cancel(c, slash, backslash)?;
            let a = self.scripts(c, a)?;
            self.place(r, c, a);
        } else {
            let b = self.expansion(d, false)?;
            self.run_bytes(b.as_ptr(), b.len() - 1, r, c)?;
        }
        self.depth -= 1;
        Ok(())
    }

    pub(super) fn def_box(&mut self, d: &Def, c: &Ctx) -> R<Bx> {
        if let Native::Strike { slash, backslash } = d.native {
            return Ok(self.cancel(c, slash, backslash)?.bx);
        }
        let b = self.expansion(d, false)?;
        self.list_in(b.as_ptr(), b.len() - 1, *c)
    }

    pub(super) fn text_def(&mut self, d: &Def, c: &Ctx, font: u8) -> R<Bx> {
        self.enter()?;
        let b = self.expansion(d, true)?;
        let src = self.push_src(b.as_ptr(), b.len() - 1);
        let bx = self.text_run(c, font)?;
        self.pop_src(src);
        self.depth -= 1;
        Ok(bx)
    }

    fn cancel(&mut self, c: &Ctx, slash: bool, backslash: bool) -> R<Atom> {
        let b = self.arg(c)?;
        let pad = sc(60, c.scale);
        let (x, y, w, h) = (-pad, -(b.h + pad), b.w + 2 * pad, b.h + b.d + 2 * pad);
        if slash {
            self.push_item(c, x, y, w, h, IT_SLASH);
        }
        if backslash {
            self.push_item(c, x, y, w, h, IT_BACKSLASH);
        }
        Ok(self.atom(b.s, b.w, b.h, b.d, ORD))
    }

    fn delimited(&mut self, matrix: bool, out: &mut Vec<u8>) -> R<()> {
        self.skip_ws();
        let (body, l, r, env): (&[u8], &[u8], &[u8], &[u8]) = match self.peek() {
            b'(' => (self.scan(b'(', b')', b')')?, b"(", b")", b"pmatrix"),
            b'[' => (self.scan(b'[', b']', b']')?, b"[", b"]", b"bmatrix"),
            b'|' => (self.scan(0, b'|', b'|')?, b"|", b"|", b"vmatrix"),
            b'{' => (self.balanced()?, b"\\{", b"\\}", b"matrix"),
            _ => return self.fail(E_ARG),
        };
        let parts: [&[u8]; 7] =
            if matrix { [b"\\begin{", env, b"}", body, b"\\end{", env, b"}"] } else { [b"\\left", l, b" ", body, b"\\right", r, b""] };
        for p in parts {
            out.extend_from_slice(p);
        }
        Ok(())
    }
}

fn ends_in_command(text: &[u8]) -> bool {
    let word = text.iter().rev().take_while(|ch| ch.is_ascii_alphabetic()).count();
    word > 0 && text.len() > word && text[text.len() - word - 1] == b'\\'
}

pub(super) fn substitute(body: &[u8], args: &[&[u8]; 9], out: &mut Vec<u8>) {
    let mut start = 0;
    let mut j = 0;
    while let Some(k) = body[j..].iter().position(|ch| *ch == b'#') {
        j += k;
        let Some(&x) = body.get(j + 1) else {
            break;
        };
        if x == b'#' {
            out.extend_from_slice(&body[start..=j]);
            j += 2;
            start = j;
        } else if (b'1'..=b'9').contains(&x) {
            out.extend_from_slice(&body[start..j]);
            let arg = args[(x - b'1') as usize];
            if arg.first().is_some_and(u8::is_ascii_alphabetic) && ends_in_command(out) {
                out.push(b' ');
            }
            out.extend_from_slice(arg);
            j += 2;
            start = j;
        } else {
            j += 1;
        }
    }
    out.extend_from_slice(&body[start..]);
}

fn parse_milli(s: &[u8]) -> Option<i32> {
    let mut v: i32 = 0;
    let mut frac = 0;
    let mut dot = false;
    if s.is_empty() {
        return None;
    }
    for &c in s {
        if c == b'.' {
            if dot {
                return None;
            }
            dot = true;
        } else if c.is_ascii_digit() {
            if !dot {
                v = v.saturating_mul(10).saturating_add((c - b'0') as i32);
            } else if frac < 3 {
                frac += 1;
                v = v.saturating_mul(10).saturating_add((c - b'0') as i32);
            }
        } else {
            return None;
        }
    }
    for _ in frac..3 {
        v = v.saturating_mul(10);
    }
    Some(v)
}
