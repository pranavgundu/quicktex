use super::figure::{Look, Pic, evaluate};
use super::*;
use crate::packages::Native;
use crate::tex::macros::Def;

const POOL_CHUNK: usize = 1 << 16;
const MAX_WARNINGS: usize = 256;
const MAX_LOOP: usize = 100_000;

fn opens_conditional(name: &[u8]) -> bool {
    name.starts_with(b"if") && !matches!(name, b"iff" | b"ifthenelse" | b"ifstrequal" | b"ifdefstring" | b"ifbool" | b"iftoggle")
}

fn format_number(value: f64, out: &mut Vec<u8>) {
    let negative = value < 0.0;
    let scaled = (if negative { -value } else { value }) * 100000.0 + 0.5;
    let whole = scaled as u64;
    if negative && whole != 0 {
        out.push(b'-');
    }
    let int = whole / 100000;
    let frac = whole % 100000;
    let mut digits = [0u8; 20];
    let mut n = 0;
    let mut v = int;
    loop {
        digits[n] = b'0' + (v % 10) as u8;
        n += 1;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    while n > 0 {
        n -= 1;
        out.push(digits[n]);
    }
    out.push(b'.');
    let mut div = 10000;
    while div > 0 {
        out.push(b'0' + ((frac / div) % 10) as u8);
        div /= 10;
    }
}

const MAX_SPLICED: usize = 64 << 20;

impl Engine {
    pub fn warning_list(&mut self) -> i32 {
        self.out.clear();
        for i in 0..self.warned.len() {
            let mut digits = [0u8; 10];
            let mut n = 0;
            let mut v = self.warned[i];
            loop {
                digits[n] = b'0' + (v % 10) as u8;
                n += 1;
                v /= 10;
                if v == 0 {
                    break;
                }
            }
            while n > 0 {
                n -= 1;
                self.out.push(digits[n]);
            }
            self.out.push(b'\n');
        }
        self.out.len() as i32
    }

    #[cold]
    #[inline(never)]
    pub(in crate::tex) fn note(&mut self, at: usize) {
        if self.warned.len() < MAX_WARNINGS {
            let at = if self.user_src { at.min(self.len) } else { self.call_pos };
            self.warned.push(at as u32);
        }
    }

    #[cold]
    #[inline(never)]
    pub(in crate::tex) fn ignore_command(&mut self, at: usize) {
        self.note(at);
        self.skip_options();
    }

    pub(in crate::tex) fn splice(&mut self, body: &[u8]) -> R<()> {
        let rest = self.slice(self.pos, self.len);
        self.spliced += body.len() + rest.len();
        if self.spliced > MAX_SPLICED {
            return self.fail(E_DEPTH);
        }
        let mut text = Vec::with_capacity(body.len() + rest.len() + 1);
        text.extend_from_slice(body);
        text.extend_from_slice(rest);
        text.push(0);
        let kept = self.pooled(&text);
        if self.user_src {
            self.call_pos = self.pos;
        }
        self.p = kept.as_ptr();
        self.len = kept.len() - 1;
        self.pos = 0;
        self.user_src = false;
        Ok(())
    }

    pub(super) fn open_group(&mut self, d: &mut Doc) {
        self.store.groups.push((d.ctx, d.body_font, d.center));
        d.open_groups += 1;
    }

    pub(super) fn group_holds_blocks(&self) -> bool {
        let mut depth = 0;
        let mut i = self.pos;
        while i < self.len {
            match unsafe { *self.p.add(i) } {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return false;
                    }
                }
                b'\\' => {
                    if self.slice(i, (i + 7).min(self.len)) == b"\\begin{" {
                        return true;
                    }
                    i += 1;
                }
                b'%' => {
                    while i < self.len && unsafe { *self.p.add(i) } != b'\n' {
                        i += 1;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        false
    }

    pub(super) fn skip_pair_of_parens(&mut self) {
        self.skip_pair(b'(', b')');
    }

    pub(super) fn skip_overlay(&mut self) {
        if self.peek() == b'<' {
            self.skip_pair(b'<', b'>');
        }
    }

    pub(super) fn loose_options(&mut self) -> &'static [u8] {
        let start = self.pos + 1;
        self.skip_pair(b'[', b']');
        self.slice(start, self.pos.saturating_sub(1).max(start))
    }

    pub(in crate::tex) fn skip_options(&mut self) {
        while self.peek() == b'[' {
            // Implemented options are parsed by their command handlers. Reaching
            // this fallback means the option was discarded, so report it.
            if self.lenient {
                self.note(self.pos);
            }
            self.skip_pair(b'[', b']');
        }
    }

    fn skip_pair(&mut self, open: u8, close: u8) {
        let mut depth = 0;
        while !self.at_end() {
            let ch = self.peek();
            if ch == b'\\' {
                self.pos += 2;
                continue;
            }
            if ch == b'%' {
                while !self.at_end() && self.peek() != b'\n' {
                    self.pos += 1;
                }
                continue;
            }
            self.pos += 1;
            if ch == open {
                depth += 1;
            } else if ch == close {
                depth -= 1;
                if depth == 0 {
                    return;
                }
            }
        }
        self.pos = self.pos.min(self.len);
    }

    pub(super) fn skip_arguments(&mut self) {
        self.skip_star();
        self.skip_overlay();
        loop {
            self.skip_ws();
            match self.peek() {
                b'[' => self.skip_pair(b'[', b']'),
                b'{' => self.skip_pair(b'{', b'}'),
                _ => return,
            }
        }
    }

    pub(super) fn skip_loose(&mut self) {
        self.skip_star();
        loop {
            self.skip_ws();
            match self.peek() {
                b'[' => self.skip_pair(b'[', b']'),
                b'{' => self.skip_pair(b'{', b'}'),
                b'(' => self.skip_pair(b'(', b')'),
                b';' => {
                    self.pos += 1;
                    return;
                }
                _ => return,
            }
        }
    }

    pub(super) fn skip_environment(&mut self, name: &[u8]) {
        while !self.at_end() {
            if self.peek() == b'\\' {
                if self.slice(self.pos, (self.pos + 5).min(self.len)) == b"\\end{" {
                    let s = self.pos + 5;
                    let e = (s + name.len()).min(self.len);
                    if self.slice(s, e) == name && self.peek_at(5 + name.len()) == b'}' {
                        self.pos = e + 1;
                        return;
                    }
                }
                self.pos += 2;
                continue;
            }
            self.pos += 1;
        }
        self.pos = self.pos.min(self.len);
    }

    pub(super) fn named_color(&self, name: &[u8]) -> Option<u32> {
        for &(known, id) in self.color_defs.iter().rev() {
            if known == name {
                let entry = self.colors[id as usize];
                let bytes = &self.color_bytes[entry.0 as usize..(entry.0 + entry.1) as usize];
                let digits = bytes.strip_prefix(b"#")?;
                let mut value = 0u32;
                for ch in digits {
                    value = value * 16 + (*ch as char).to_digit(16)?;
                }
                return Some(value);
            }
        }
        super::figure::color_by_name(name)
    }

    pub(super) fn source_copy(&mut self, text: &[u8]) -> &'static [u8] {
        let mut owned = Vec::with_capacity(text.len() + 1);
        owned.extend_from_slice(text);
        owned.push(0);
        let kept = self.pooled(&owned);
        &kept[..text.len()]
    }

    pub(in crate::tex) fn lasting(&mut self, text: &'static [u8]) -> &'static [u8] {
        if self.user_src { text } else { self.pooled(text) }
    }

    pub(super) fn pooled(&mut self, text: &[u8]) -> &'static [u8] {
        let need = text.len();
        if self.pool.is_empty() {
            self.pool.push(Vec::with_capacity(POOL_CHUNK.max(need)));
        }
        if self.pool[self.pool_at].capacity() - self.pool[self.pool_at].len() < need {
            self.pool_at += 1;
            if self.pool_at == self.pool.len() {
                self.pool.push(Vec::with_capacity(POOL_CHUNK.max(need)));
            } else {
                self.pool[self.pool_at].clear();
                if self.pool[self.pool_at].capacity() < need {
                    let fresh = Vec::with_capacity(need);
                    self.pool.insert(self.pool_at, fresh);
                }
            }
        }
        let chunk = &mut self.pool[self.pool_at];
        let at = chunk.len();
        chunk.extend_from_slice(text);
        unsafe { core::slice::from_raw_parts(chunk.as_ptr().add(at), need) }
    }

    pub(super) fn expand_text(&mut self, text: &'static [u8]) -> &'static [u8] {
        if self.defs.is_empty() || !text.contains(&b'\\') {
            return text;
        }
        let mut current = text;
        for _ in 0..8 {
            let mut out = Vec::new();
            let mut changed = false;
            let mut i = 0;
            while i < current.len() {
                if current[i] == b'\\' {
                    let mut e = i + 1;
                    while e < current.len() && current[e].is_ascii_alphabetic() {
                        e += 1;
                    }
                    if let Some(def) = self.find_def(&current[i + 1..e]) {
                        if def.spec.is_empty() && matches!(def.native, Native::None) {
                            out.extend_from_slice(def.body);
                            changed = true;
                            i = e;
                            while i < current.len() && current[i] == b' ' {
                                i += 1;
                            }
                            continue;
                        }
                    }
                    let stop = e.max(i + 2).min(current.len());
                    out.extend_from_slice(&current[i..stop]);
                    i = stop;
                    continue;
                }
                out.push(current[i]);
                i += 1;
            }
            if !changed {
                return current;
            }
            current = self.pooled(&out);
        }
        current
    }

    fn scoped_macro(&mut self, pic: &mut Pic, name: &'static [u8], value: &[u8]) -> R<()> {
        let name = self.pooled(name);
        let old = self.find_def(name);
        pic.saved.push((name, old));
        let body = self.pooled(value);
        self.define(name, b"", b"", body)
    }

    pub(super) fn restore_defs(&mut self, pic: &mut Pic, mark: usize) -> R<()> {
        while pic.saved.len() > mark {
            let (name, old) = pic.saved.pop().unwrap();
            let def = old.unwrap_or(Def { name, spec: b"", default: b"", body: b"", native: Native::None });
            self.add_def(def)?;
        }
        Ok(())
    }

    fn command_name(&mut self) -> R<&'static [u8]> {
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
        let name = self.read_name();
        if braced {
            self.skip_ws();
            if self.peek() == b'}' {
                self.pos += 1;
            }
        }
        Ok(name)
    }

    pub(super) fn pgf_math(&mut self, pic: &mut Pic, command: &[u8]) -> R<()> {
        let name: &'static [u8] = if command == b"pgfmathparse" { b"pgfmathresult" } else { self.command_name()? };
        let text = self.group_text()?;
        let text = self.expand_text(text);
        let value = match evaluate(text, 0.0, 0.0) {
            Some((v, _)) if !v.is_nan() => v,
            _ => {
                self.note(self.pos);
                0.0
            }
        };
        let mut digits = Vec::with_capacity(24);
        if command == b"pgfmathtruncatemacro" {
            format_number((value as i64) as f64, &mut digits);
            digits.truncate(digits.len() - 6);
        } else {
            format_number(value, &mut digits);
        }
        self.scoped_macro(pic, name, &digits)
    }

    fn int_token(&mut self) -> i64 {
        self.skip_ws();
        if self.peek() == b'\\' {
            self.pos += 1;
            let name = self.read_name();
            let body = self.find_def(name).map_or(&b""[..], |def| def.body);
            return evaluate(trim_space(body), 0.0, 0.0).map_or(0, |(v, _)| v as i64);
        }
        let s = self.pos;
        if matches!(self.peek(), b'-' | b'+') {
            self.pos += 1;
        }
        while self.peek().is_ascii_digit() || self.peek() == b'.' {
            self.pos += 1;
        }
        evaluate(self.slice(s, self.pos), 0.0, 0.0).map_or(0, |(v, _)| v as i64)
    }

    fn skip_branch(&mut self, stop_at_else: bool) {
        let mut depth = 0;
        while !self.at_end() {
            let ch = self.peek();
            if ch == b'%' {
                while !self.at_end() && self.peek() != b'\n' {
                    self.pos += 1;
                }
                continue;
            }
            if ch != b'\\' {
                self.pos += 1;
                continue;
            }
            self.pos += 1;
            let name = self.read_name();
            if opens_conditional(name) {
                depth += 1;
            } else if name == b"fi" {
                if depth == 0 {
                    return;
                }
                depth -= 1;
            } else if name == b"else" && depth == 0 && stop_at_else {
                return;
            }
        }
    }

    fn flag(&self, name: &[u8]) -> Option<usize> {
        self.flags.iter().position(|(flag, _)| *flag == name)
    }

    #[inline(never)]
    pub(in crate::tex) fn conditional(&mut self, name: &'static [u8]) -> R<bool> {
        let truth = match name {
            b"fi" => return Ok(true),
            b"else" => {
                self.skip_branch(false);
                return Ok(true);
            }
            b"newif" => {
                let full = self.command_name()?;
                if let Some(rest) = full.strip_prefix(b"if") {
                    let rest = self.lasting(rest);
                    self.flags.push((rest, false));
                }
                return Ok(true);
            }
            b"ifnum" | b"ifdim" => {
                let a = self.int_token();
                while self.peek().is_ascii_alphabetic() {
                    self.pos += 1;
                }
                self.skip_ws();
                let relation = self.peek();
                self.pos += 1;
                let b = self.int_token();
                while self.peek().is_ascii_alphabetic() {
                    self.pos += 1;
                }
                match relation {
                    b'<' => a < b,
                    b'>' => a > b,
                    _ => a == b,
                }
            }
            b"iftrue" => true,
            b"iffalse" => false,
            b"ifx" | b"if" => {
                self.skip_ws();
                let (a, b) = (self.token_text(), self.token_text());
                if b == b"\\undefined" || b == b"\\relax" { a.len() > 1 && !self.defined_token(a) && b == b"\\undefined" } else { a == b }
            }
            b"ifdefined" => {
                self.skip_ws();
                let a = self.token_text();
                self.defined_token(a)
            }
            b"ifcsname" | b"ifcase" | b"ifodd" | b"ifvmode" | b"ifhmode" | b"ifmmode" | b"ifinner" | b"ifeof" | b"ifcat" | b"ifvoid"
            | b"ifhbox" | b"ifvbox" | b"ifpdf" | b"ifxetex" | b"ifluatex" => false,
            _ => {
                if let Some(rest) = name.strip_prefix(b"if") {
                    if let Some(i) = self.flag(rest) {
                        if self.flags[i].1 {
                            return Ok(true);
                        }
                        self.skip_branch(true);
                        return Ok(true);
                    }
                }
                let on = name.strip_suffix(b"true").map(|n| (n, true));
                let off = name.strip_suffix(b"false").map(|n| (n, false));
                if let Some((rest, value)) = on.or(off) {
                    if let Some(i) = self.flag(rest) {
                        self.flags[i].1 = value;
                        return Ok(true);
                    }
                }
                return Ok(false);
            }
        };
        if !truth {
            self.skip_branch(true);
        }
        Ok(true)
    }

    pub(super) fn alias(&mut self) -> R<()> {
        self.skip_ws();
        let target = self.token_text();
        self.skip_ws();
        if self.peek() == b'=' {
            self.pos += 1;
        }
        let source = self.token_text();
        let (Some(name), Some(from)) = (target.strip_prefix(b"\\"), source.strip_prefix(b"\\")) else {
            return Ok(());
        };
        if !self.user_src {
            return Ok(());
        }
        if let Some(mut def) = self.find_def(from) {
            def.name = name;
            return self.add_def(def);
        }
        self.define(name, b"", b"", source)
    }

    pub(in crate::tex) fn loose_argument(&mut self) -> &'static [u8] {
        self.skip_ws();
        if self.peek() != b'{' {
            return self.token_text();
        }
        let s = self.pos + 1;
        self.skip_pair(b'{', b'}');
        self.slice(s, self.pos.saturating_sub(1).max(s))
    }

    fn token_text(&mut self) -> &'static [u8] {
        self.skip_ws();
        let s = self.pos;
        if self.peek() == b'\\' {
            self.pos += 1;
            self.read_name();
        } else if !self.at_end() {
            self.pos += 1;
        }
        self.slice(s, self.pos)
    }

    fn defined_token(&self, token: &[u8]) -> bool {
        let name = token.strip_prefix(b"\\").unwrap_or(token);
        self.find_def(name).is_some() || lookup(name).is_some()
    }

    pub(super) fn scene_setting(&mut self, pic: &mut Pic, name: &[u8], look: Look) -> R<()> {
        if name == b"tikzstyle" {
            self.style_line(Some(pic), look)
        } else {
            self.skip_arguments();
            Ok(())
        }
    }

    pub(super) fn preamble_style(&mut self) -> R<()> {
        self.style_line(None, super::figure::first_look())
    }

    fn style_line(&mut self, pic: Option<&mut Pic>, look: Look) -> R<()> {
        self.skip_ws();
        let name = if self.peek() == b'{' { self.group_text()? } else { b"" };
        self.skip_ws();
        if self.peek() == b'+' {
            self.pos += 1;
        }
        if self.peek() == b'=' {
            self.pos += 1;
        }
        self.skip_ws();
        let body = if self.peek() == b'[' { self.bracketed()? } else { b"" };
        let mut text = Vec::with_capacity(name.len() + body.len() + 12);
        text.extend_from_slice(name);
        text.extend_from_slice(b"/.style={");
        text.extend_from_slice(body);
        text.push(b'}');
        let text = self.pooled(&text);
        match pic {
            Some(pic) => {
                let mut l = look;
                self.options(pic, &mut l, text, false, 0)
            }
            None => {
                self.store.settings.push(text);
                Ok(())
            }
        }
    }

    pub(super) fn scene_foreach(&mut self, d: &mut Doc, pic: &mut Pic, look: Look) -> R<()> {
        self.skip_gap();
        let mut names: Vec<&'static [u8]> = Vec::new();
        loop {
            self.skip_ws();
            if self.peek() != b'\\' {
                break;
            }
            self.pos += 1;
            names.push(self.read_name());
            if self.peek() == b'/' {
                self.pos += 1;
                continue;
            }
            break;
        }
        self.skip_ws();
        let mut count: Option<&'static [u8]> = None;
        let mut evaluated: Option<(&'static [u8], &'static [u8], &'static [u8])> = None;
        if self.peek() == b'[' {
            let text = self.bracketed()?;
            let mut at = 0;
            while let Some((key, value)) = super::figure::next_option(text, &mut at) {
                if key == b"count" {
                    let value = super::figure::trim(value);
                    let end = value.iter().position(|c| *c == b' ').unwrap_or(value.len());
                    count = value.get(1..end);
                } else if key == b"evaluate" {
                    if let Some((from, into, how)) = evaluate_option(value) {
                        evaluated = Some((from, into, how));
                    }
                }
            }
            self.skip_ws();
        }
        if self.word() != b"in" {
            return self.fail(E_ARG);
        }
        self.skip_ws();
        let list = if self.peek() == b'{' { self.group_text()? } else { self.token_text() };
        let list = self.expand_text(list);
        let values = spread(list);
        self.skip_gap();
        let body = if self.peek() == b'{' {
            self.group_text()?
        } else {
            let s = self.pos;
            self.skip_statement()?;
            self.slice(s, self.pos)
        };
        let body = self.source_copy(body);
        let mark = pic.saved.len();
        for (index, value) in values.iter().enumerate().take(MAX_LOOP) {
            self.expansions += 1;
            if self.expansions > 400_000 {
                return self.fail(E_DEPTH);
            }
            let text: &[u8] = value;
            let mut part = 0;
            let mut last: &[u8] = text;
            for piece in text.split(|c| *c == b'/') {
                if part < names.len() {
                    let piece = super::figure::trim(piece);
                    let piece = strip_braces(piece);
                    self.scoped_macro(pic, names[part], piece)?;
                    part += 1;
                    last = piece;
                }
            }
            while part < names.len() {
                self.scoped_macro(pic, names[part], last)?;
                part += 1;
            }
            if let Some(name) = count {
                let mut digits = Vec::new();
                format_number(index as f64 + 1.0, &mut digits);
                digits.truncate(digits.len() - 6);
                self.scoped_macro(pic, name, &digits)?;
            }
            if let Some((_, into, how)) = evaluated {
                let how = self.expand_text(how);
                let v = evaluate(how, 0.0, 0.0).map_or(0.0, |(v, _)| v);
                let mut digits = Vec::new();
                format_number(v, &mut digits);
                self.scoped_macro(pic, into, &digits)?;
            }
            self.enter()?;
            let src = self.push_src(body.as_ptr(), body.len());
            self.scene(d, pic, look, b"")?;
            self.pop_src(src);
            self.depth -= 1;
        }
        self.restore_defs(pic, mark)
    }
}

fn trim_space(text: &[u8]) -> &[u8] {
    text.trim_ascii()
}

fn strip_braces(text: &[u8]) -> &[u8] {
    if text.len() >= 2 && text[0] == b'{' && text[text.len() - 1] == b'}' { &text[1..text.len() - 1] } else { text }
}

fn evaluate_option(value: &'static [u8]) -> Option<(&'static [u8], &'static [u8], &'static [u8])> {
    let text = value.trim_ascii();
    let at = text.windows(4).position(|w| w == b" as ")?;
    let from = text[..at].trim_ascii().strip_prefix(b"\\")?;
    let rest = text[at + 4..].trim_ascii();
    let eq = rest.iter().position(|c| *c == b' ' || *c == b'=').unwrap_or(rest.len());
    let into = rest[..eq].strip_prefix(b"\\")?;
    let how = rest[eq..].trim_ascii();
    let how = how.strip_prefix(b"using").unwrap_or(how).trim_ascii();
    Some((from, into, how))
}

fn spread(list: &[u8]) -> Vec<Vec<u8>> {
    let mut parts: Vec<&[u8]> = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (i, c) in list.iter().enumerate() {
        match c {
            b'{' | b'(' => depth += 1,
            b'}' | b')' => depth -= 1,
            b',' if depth == 0 => {
                parts.push(list[start..i].trim_ascii());
                start = i + 1;
            }
            _ => {}
        }
    }
    let tail = list[start..].trim_ascii();
    if !tail.is_empty() {
        parts.push(tail);
    }
    let mut values: Vec<Vec<u8>> = Vec::with_capacity(parts.len());
    let mut i = 0;
    while i < parts.len() {
        if parts[i] == b"..." && !values.is_empty() && i + 1 < parts.len() {
            let last = values[values.len() - 1].clone();
            let before = if values.len() >= 2 { Some(values[values.len() - 2].clone()) } else { None };
            let end = parts[i + 1];
            if let (Some((a, _)), Some((z, _))) = (evaluate(&last, 0.0, 0.0), evaluate(end, 0.0, 0.0)) {
                let step = before.and_then(|b| evaluate(&b, 0.0, 0.0)).map_or(if z >= a { 1.0 } else { -1.0 }, |(b, _)| a - b);
                if step != 0.0 {
                    let mut k = 1.0;
                    while (a + k * step - z) * step <= 1e-9 && values.len() < MAX_LOOP {
                        let mut text = Vec::new();
                        let v = a + k * step;
                        format_number(v, &mut text);
                        if v == (v as i64) as f64 {
                            text.truncate(text.len() - 6);
                        }
                        values.push(text);
                        k += 1.0;
                    }
                }
            } else if last.len() == 1 && end.len() == 1 {
                let mut c = last[0];
                while c != end[0] && values.len() < MAX_LOOP {
                    c = if end[0] > c { c + 1 } else { c - 1 };
                    values.push([c].to_vec());
                }
            }
            i += 2;
            continue;
        }
        values.push(parts[i].to_vec());
        i += 1;
    }
    values
}
