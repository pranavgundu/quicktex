//! A deliberately bounded, in-process BibTeX reader. No external BibTeX/Biber process.
use alloc::vec::Vec;
#[cfg(test)]
use alloc::{format, string::String};

#[inline(never)]
fn message(parts: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    for part in parts {
        out.extend_from_slice(part);
    }
    out
}
#[inline(never)]
fn decimal(out: &mut Vec<u8>, value: usize) {
    if value >= 10 {
        decimal(out, value / 10);
    }
    out.push(b'0' + (value % 10) as u8);
}

#[derive(Clone, Debug)]
struct Entry {
    key: Vec<u8>,
    fields: Vec<(Vec<u8>, Vec<u8>)>,
}
impl Entry {
    #[inline(never)]
    fn field(&self, name: &[u8]) -> &[u8] {
        self.fields.iter().find(|(n, _)| n == name).map_or(b"", |(_, v)| v)
    }
    #[inline(never)]
    fn authors(&self, full: bool) -> Vec<u8> {
        let author = if self.field(b"author").is_empty() { self.field(b"editor") } else { self.field(b"author") };
        if author.is_empty() {
            return self.key.clone();
        }
        let count = author.windows(5).filter(|w| *w == b" and ").count() + 1;
        let mut text = Vec::new();
        let mut rest = author;
        for at in 0..count {
            let end = rest.windows(5).position(|w| w == b" and ").unwrap_or(rest.len());
            let name = rest[..end].trim_ascii();
            let last = if name.starts_with(b"{") && name.ends_with(b"}") {
                name
            } else if let Some(comma) = name.iter().position(|b| *b == b',') {
                name[..comma].trim_ascii()
            } else {
                name.rsplit(|b| b.is_ascii_whitespace()).find(|part| !part.is_empty()).unwrap_or(name)
            };
            if at > 0 {
                text.extend_from_slice(if count == 2 { b" and " } else { b", " });
            }
            text.extend_from_slice(last);
            if count > 2 && !full {
                text.extend_from_slice(b" et al.");
                break;
            }
            if end == rest.len() {
                break;
            }
            rest = &rest[end + 5..];
        }
        text
    }
}

#[derive(Default)]
pub(super) struct Bibliography {
    entries: Vec<Entry>,
    keys: Vec<Vec<u8>>,
    resources: Vec<Vec<u8>>,
    author_year: bool,
    active: bool,
    uncertain: bool,
}

#[derive(Default)]
struct Requests {
    files: Vec<(Vec<u8>, usize)>,
    keys: Vec<Vec<u8>>,
    styles: Vec<(Vec<u8>, usize)>,
    numbers: bool,
    author_year: bool,
    uncertain: bool,
    scanned: usize,
    visits: usize,
    limit_at: Option<usize>,
}

#[inline(never)]
fn skip_balanced(source: &[u8], p: &mut usize, open: u8, close: u8) {
    if source.get(*p) != Some(&open) {
        return;
    }
    *p += 1;
    let mut depth = 1;
    while *p < source.len() && depth > 0 {
        match source[*p] {
            b'\\' => {
                *p = (*p + 2).min(source.len());
                continue;
            }
            b'%' => {
                while *p < source.len() && source[*p] != b'\n' {
                    *p += 1;
                }
                continue;
            }
            ch if ch == open => depth += 1,
            ch if ch == close => depth -= 1,
            _ => {}
        }
        *p += 1;
    }
}

// This source scan only discovers resources and citation ordering. The normal
// engine remains responsible for TeX parsing and expansion.
#[inline(never)]
fn requests(source: &[u8], files: &[(Vec<u8>, Vec<u8>)], req: &mut Requests, stack: &mut Vec<Vec<u8>>, origin: usize) {
    req.visits += 1;
    req.scanned = req.scanned.saturating_add(source.len());
    if req.visits > 4096 || req.scanned > 64 * 1024 * 1024 {
        req.limit_at.get_or_insert(origin);
        return;
    }
    let mut p = 0;
    while p < source.len() {
        if req.limit_at.is_some() {
            return;
        }
        if req.keys.len() + req.files.len() > 16384 {
            req.limit_at = Some(origin);
            return;
        }
        if source[p] == b'%' {
            while p < source.len() && source[p] != b'\n' {
                p += 1;
            }
            continue;
        }
        if source[p] != b'\\' {
            p += 1;
            continue;
        }
        let start = p;
        p += 1;
        let at = p;
        while p < source.len() && source[p].is_ascii_alphabetic() {
            p += 1;
        }
        let command = &source[at..p];
        if command.is_empty() {
            p = (p + 1).min(source.len());
            continue;
        }
        if source.get(p) == Some(&b'*') {
            p += 1;
        }
        if command == b"end" {
            let mut at = p;
            while source.get(at).is_some_and(u8::is_ascii_whitespace) {
                at += 1;
            }
            if source.get(at..at.saturating_add(10)) == Some(b"{document}") {
                return;
            }
        }
        if command == b"iffalse" {
            req.uncertain = true;
            let mut depth = 1;
            while p < source.len() && depth > 0 {
                if source[p] == b'%' {
                    while p < source.len() && source[p] != b'\n' {
                        p += 1;
                    }
                    continue;
                }
                if source[p] != b'\\' {
                    p += 1;
                    continue;
                }
                p += 1;
                let at = p;
                while source.get(p).is_some_and(u8::is_ascii_alphabetic) {
                    p += 1;
                }
                let name = &source[at..p];
                if name.starts_with(b"if") && name != b"iff" {
                    depth += 1;
                }
                if name == b"fi" {
                    depth -= 1;
                }
                if name == b"else" && depth == 1 {
                    break;
                }
            }
            continue;
        }
        if command.starts_with(b"if") && command != b"iff" {
            req.uncertain = true;
        }
        if matches!(
            command,
            b"newcommand"
                | b"renewcommand"
                | b"providecommand"
                | b"DeclareRobustCommand"
                | b"newenvironment"
                | b"renewenvironment"
                | b"def"
                | b"gdef"
                | b"edef"
                | b"xdef"
        ) {
            while source.get(p).is_some_and(u8::is_ascii_whitespace) {
                p += 1;
            }
            if matches!(command, b"def" | b"gdef" | b"edef" | b"xdef") {
                while p < source.len() && source[p] != b'{' {
                    p += 1;
                }
                skip_balanced(source, &mut p, b'{', b'}');
            } else {
                if source.get(p) == Some(&b'{') {
                    skip_balanced(source, &mut p, b'{', b'}');
                } else if source.get(p) == Some(&b'\\') {
                    p += 1;
                    while source.get(p).is_some_and(u8::is_ascii_alphabetic) {
                        p += 1;
                    }
                }
                loop {
                    while source.get(p).is_some_and(u8::is_ascii_whitespace) {
                        p += 1;
                    }
                    if source.get(p) != Some(&b'[') {
                        break;
                    }
                    skip_balanced(source, &mut p, b'[', b']');
                }
                skip_balanced(source, &mut p, b'{', b'}');
                if command == b"newenvironment" || command == b"renewenvironment" {
                    while source.get(p).is_some_and(u8::is_ascii_whitespace) {
                        p += 1;
                    }
                    skip_balanced(source, &mut p, b'{', b'}');
                }
            }
            continue;
        }
        if command == b"verb" {
            if let Some(&delimiter) = source.get(p) {
                p += 1;
                while source.get(p).is_some_and(|b| *b != delimiter) {
                    p += 1;
                }
                p = (p + 1).min(source.len());
            }
            continue;
        }
        if command == b"begin" {
            let mut at = p;
            while source.get(at).is_some_and(u8::is_ascii_whitespace) {
                at += 1;
            }
            if source.get(at) == Some(&b'{') {
                let end = source[at + 1..].iter().position(|b| *b == b'}').map(|n| at + 1 + n);
                if let Some(end) = end {
                    let name = &source[at + 1..end];
                    if matches!(name, b"verbatim" | b"verbatim*" | b"Verbatim" | b"lstlisting" | b"minted" | b"comment") {
                        let mut close = b"\\end{".to_vec();
                        close.extend_from_slice(name);
                        close.push(b'}');
                        p = source[end + 1..]
                            .windows(close.len())
                            .position(|w| w == close)
                            .map_or(source.len(), |n| end + 1 + n + close.len());
                        continue;
                    }
                }
            }
        }
        let mut options = Vec::new();
        loop {
            while source.get(p).is_some_and(u8::is_ascii_whitespace) {
                p += 1;
            }
            if source.get(p) != Some(&b'[') {
                break;
            }
            p += 1;
            let at = p;
            while p < source.len() && source[p] != b']' {
                p += 1;
            }
            options.extend_from_slice(&source[at..p]);
            if p < source.len() {
                p += 1;
            }
        }
        let relevant = matches!(
            command,
            b"bibliography"
                | b"bibliographystyle"
                | b"addbibresource"
                | b"input"
                | b"include"
                | b"usepackage"
                | b"RequirePackage"
                | b"nocite"
                | b"cite"
                | b"citep"
                | b"citet"
                | b"citealt"
                | b"citealp"
                | b"citeauthor"
                | b"citeyear"
                | b"citeyearpar"
                | b"Cite"
                | b"Citep"
                | b"Citet"
                | b"parencite"
                | b"textcite"
                | b"autocite"
                | b"footcite"
        );
        if !relevant {
            continue;
        }
        let arg;
        if source.get(p) == Some(&b'{') {
            p += 1;
            let at = p;
            let mut depth = 1;
            while p < source.len() {
                if source[p] == b'\\' {
                    p = (p + 2).min(source.len());
                    continue;
                }
                if source[p] == b'{' {
                    depth += 1;
                }
                if source[p] == b'}' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                p += 1;
            }
            arg = &source[at..p];
            if p < source.len() {
                p += 1;
            }
        } else if command == b"input" || command == b"include" {
            let at = p;
            while p < source.len() && !source[p].is_ascii_whitespace() && source[p] != b'%' {
                p += 1;
            }
            arg = &source[at..p];
        } else {
            continue;
        }
        let pos = if stack.is_empty() { start } else { origin };
        match command {
            b"bibliography" | b"addbibresource" => {
                for name in arg.split(|b| *b == b',') {
                    req.files.push((name.trim_ascii().to_vec(), pos));
                }
            }
            b"bibliographystyle" => req.styles.push((arg.trim_ascii().to_vec(), pos)),
            b"input" | b"include" => {
                let name = arg.strip_prefix(b"./").unwrap_or(arg);
                if let Some((name, text)) = files.iter().find(|(n, _)| n == name || (n.starts_with(name) && &n[name.len()..] == b".tex")) {
                    if !stack.contains(name) && stack.len() < 64 {
                        stack.push(name.clone());
                        requests(text, files, req, stack, pos);
                        stack.pop();
                    }
                }
            }
            b"usepackage" | b"RequirePackage" => {
                if arg.split(|b| *b == b',').any(|n| n.trim_ascii() == b"natbib") {
                    for option in options.split(|b| *b == b',') {
                        if option.trim_ascii() == b"numbers" || option.trim_ascii() == b"super" {
                            req.numbers = true;
                        }
                        if option.trim_ascii() == b"authoryear" {
                            req.author_year = true;
                        }
                    }
                }
            }
            _ => {
                for key in arg.split(|b| *b == b',') {
                    let key = key.trim_ascii().to_vec();
                    if !req.keys.contains(&key) {
                        req.keys.push(key);
                    }
                }
            }
        }
    }
}

impl Bibliography {
    #[inline(never)]
    pub(super) fn prepare(source: &[u8], files: &[(Vec<u8>, Vec<u8>)]) -> Result<Self, (Vec<u8>, usize)> {
        // Inspect command starts once. Repeated whole-source substring searches
        // make ordinary large documents pay for bibliography discovery five times.
        let mut rest = source;
        let mut resources = false;
        while let Some(at) = rest.iter().position(|byte| *byte == b'\\') {
            rest = &rest[at + 1..];
            let relevant = match rest.first() {
                Some(b'b') => rest.starts_with(b"bibliography"),
                Some(b'a') => rest.starts_with(b"addbibresource"),
                Some(b'i') => rest.starts_with(b"input") || rest.starts_with(b"include"),
                Some(b'p') => rest.starts_with(b"printbibliography"),
                _ => false,
            };
            if relevant {
                resources = true;
                break;
            }
        }
        if !resources {
            return Ok(Self::default());
        }
        let mut req = Requests::default();
        requests(source, files, &mut req, &mut Vec::new(), 0);
        if let Some(at) = req.limit_at {
            return Err((b"bibliography source discovery limit exceeded (too many repeated includes or source bytes)".to_vec(), at));
        }
        let mut result = Self::default();
        result.active = !req.files.is_empty();
        result.uncertain = req.uncertain && result.active;
        for (style, pos) in &req.styles {
            match style.as_slice() {
                b"plain" | b"unsrt" | b"abbrv" => result.author_year = false,
                b"plainnat" | b"unsrtnat" | b"abbrvnat" => result.author_year = true,
                _ => {
                    return Err((
                        message(&[
                            b"unsupported bibliography style '",
                            style,
                            b"'; supported: plain, unsrt, abbrv, plainnat, unsrtnat, abbrvnat",
                        ]),
                        *pos,
                    ));
                }
            }
        }
        if req.author_year {
            result.author_year = true;
        }
        if req.numbers {
            result.author_year = false;
        }
        let mut loaded = Vec::new();
        let mut expanded = 0;
        let mut asset_bytes = 0usize;
        for (name, pos) in req.files {
            let name = name.strip_prefix(b"./").unwrap_or(&name);
            let Some((path, bytes)) = files.iter().find(|(n, _)| n == name || (n.starts_with(name) && &n[name.len()..] == b".bib")) else {
                return Err((message(&[b"bibliography file '", name, b"' was not found in project files"]), pos));
            };
            if loaded.contains(path) {
                continue;
            }
            loaded.push(path.clone());
            asset_bytes = asset_bytes.saturating_add(bytes.len());
            if asset_bytes > 64 * 1024 * 1024 {
                return Err((b"bibliography asset byte limit exceeded (64 MiB total)".to_vec(), pos));
            }
            let entries = parse_limited(bytes, &mut expanded)
                .map_err(|why| (message(&[b"invalid bibliography '", path, b"': ", why.as_bytes()]), pos))?;
            for entry in entries {
                if result.entries.len() >= 4096 {
                    return Err((b"bibliography entry limit exceeded (4096 entries)".to_vec(), pos));
                }
                if result.entries.iter().any(|e| e.key == entry.key) {
                    return Err((message(&[b"duplicate bibliography key '", &entry.key, b"'"]), pos));
                }
                result.entries.push(entry);
            }
        }
        result.resources = loaded;
        let all = req.keys.iter().any(|k| k == b"*");
        result.keys = req.keys.into_iter().filter(|k| k != b"*").collect();
        if all {
            for e in &result.entries {
                if !result.keys.contains(&e.key) {
                    result.keys.push(e.key.clone());
                }
            }
        }
        result.keys.retain(|key| result.entries.iter().any(|entry| &entry.key == key));
        if !req.styles.last().is_some_and(|(s, _)| s == b"unsrt" || s == b"unsrtnat") {
            let mut orders = Vec::with_capacity(result.keys.len());
            for key in &result.keys {
                let entry = result.entries.iter().find(|e| &e.key == key).unwrap();
                let mut order = entry.authors(true);
                order.extend_from_slice(entry.field(b"year"));
                order.extend_from_slice(entry.field(b"title"));
                order.make_ascii_lowercase();
                orders.push(order);
            }
            // At most 4096 entries; this compact stable sort keeps the WASM
            // renderer small and computes author sort keys only once.
            for at in 1..orders.len() {
                let mut i = at;
                while i > 0 && orders[i] < orders[i - 1] {
                    orders.swap(i, i - 1);
                    result.keys.swap(i, i - 1);
                    i -= 1;
                }
            }
        }
        Ok(result)
    }

    #[inline(never)]
    pub(super) fn has_resource(&self, names: &[u8]) -> bool {
        names.split(|b| *b == b',').all(|name| {
            let name = name.trim_ascii();
            let name = name.strip_prefix(b"./").unwrap_or(name);
            self.resources.iter().any(|n| n == name || (n.starts_with(name) && &n[name.len()..] == b".bib"))
        })
    }

    #[inline(never)]
    pub(super) fn uncertain(&self) -> bool {
        self.uncertain
    }

    #[inline(never)]
    pub(super) fn is_active(&self) -> bool {
        self.active
    }

    #[inline(never)]
    pub(super) fn citation(&self, command: &[u8], keys: &[u8], full: bool, pre: &[u8], post: &[u8]) -> Option<(Vec<u8>, bool)> {
        if !self.active {
            return None;
        }
        let mut text = Vec::new();
        let mut missing = false;
        let author_only = command == b"citeauthor";
        let year_only = matches!(command, b"citeyear" | b"citeyearpar");
        let textual = matches!(command, b"citet" | b"Citet" | b"textcite");
        let bare = matches!(command, b"citealt" | b"citealp" | b"citeauthor" | b"citeyear");
        let enclosed = !bare && !textual;
        let (open, close) = if self.author_year || year_only { (b'(', b')') } else { (b'[', b']') };
        if enclosed {
            text.push(open);
        }
        if !pre.is_empty() {
            text.extend_from_slice(pre);
            text.push(b' ');
        }
        for (at, key) in keys.split(|b| *b == b',').enumerate() {
            if at > 0 {
                text.extend_from_slice(if self.author_year { b"; " } else { b", " });
            }
            let Some(entry) = self.entries.iter().find(|e| e.key == key.trim_ascii()) else {
                text.extend_from_slice(b"??");
                missing = true;
                continue;
            };
            if !self.keys.contains(&entry.key) {
                missing = true;
            }
            if author_only {
                text.extend(entry.authors(full));
                continue;
            }
            let year = if entry.field(b"year").is_empty() { b"n.d." } else { entry.field(b"year") };
            if year_only {
                text.extend_from_slice(year);
                continue;
            }
            if self.author_year || textual || command == b"citealt" {
                text.extend(entry.authors(full));
                if self.author_year && !textual && command != b"citealt" {
                    text.push(b',');
                }
                text.push(b' ');
            }
            if textual {
                text.push(open);
            }
            if self.author_year {
                text.extend_from_slice(year);
            } else if let Some(number) = self.keys.iter().position(|k| k == &entry.key) {
                decimal(&mut text, number + 1);
            } else {
                text.extend_from_slice(b"??");
                missing = true;
            }
            if textual {
                text.push(close);
            }
        }
        if !post.is_empty() {
            text.extend_from_slice(b", ");
            text.extend_from_slice(post);
        }
        if enclosed {
            text.push(close);
        }
        Some((text, missing))
    }

    #[inline(never)]
    pub(super) fn document(&self) -> Vec<u8> {
        if !self.active {
            return Vec::new();
        }
        let mut out = b"\\begin{thebibliography}{99}".to_vec();
        for key in &self.keys {
            let Some(entry) = self.entries.iter().find(|e| &e.key == key) else {
                continue;
            };
            out.extend_from_slice(b"\\bibitem{");
            out.extend_from_slice(key);
            out.extend_from_slice(b"}");
            for name in [
                b"author".as_slice(),
                b"title",
                b"journal",
                b"booktitle",
                b"publisher",
                b"volume",
                b"number",
                b"pages",
                b"year",
                b"doi",
                b"url",
                b"note",
            ] {
                let value = if name == b"author" && entry.field(name).is_empty() { entry.field(b"editor") } else { entry.field(name) };
                if !value.is_empty() {
                    if name == b"url" || name == b"doi" {
                        out.extend_from_slice(b"\\nolinkurl{");
                    }
                    out.extend_from_slice(value);
                    if name == b"url" || name == b"doi" {
                        out.push(b'}');
                    }
                    out.extend_from_slice(b". ");
                }
            }
            out.push(b'\n');
        }
        out.extend_from_slice(b"\\end{thebibliography}");
        out
    }
}

struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
    strings: Vec<(Vec<u8>, Vec<u8>)>,
    expanded: usize,
}
#[inline(never)]
fn append_value(out: &mut Vec<u8>, value: &[u8], expanded: &mut usize) -> Result<(), &'static str> {
    if value.len() > (1024 * 1024usize).saturating_sub(out.len()) || value.len() > (8 * 1024 * 1024usize).saturating_sub(*expanded) {
        return Err("BibTeX value expansion limit exceeded (1 MiB per value, 8 MiB total)");
    }
    *expanded += value.len();
    out.extend_from_slice(value);
    Ok(())
}

impl Parser<'_> {
    #[inline(never)]
    fn space(&mut self) {
        loop {
            while self.bytes.get(self.at).is_some_and(u8::is_ascii_whitespace) {
                self.at += 1;
            }
            if self.bytes.get(self.at) != Some(&b'%') {
                break;
            }
            while self.at < self.bytes.len() && self.bytes[self.at] != b'\n' {
                self.at += 1;
            }
        }
    }
    #[inline(never)]
    fn token(&mut self) -> Vec<u8> {
        self.space();
        let at = self.at;
        while self.bytes.get(self.at).is_some_and(|b| !b.is_ascii_whitespace() && !b"=,{}()#\"".contains(b)) {
            self.at += 1;
        }
        self.bytes[at..self.at].to_vec()
    }
    #[inline(never)]
    fn value(&mut self) -> Result<Vec<u8>, &'static str> {
        let mut out = Vec::new();
        loop {
            self.space();
            let Some(&open) = self.bytes.get(self.at) else {
                return Err("missing field value");
            };
            if open == b'{' || open == b'"' {
                self.at += 1;
                let at = self.at;
                let mut depth = 0;
                loop {
                    let Some(&b) = self.bytes.get(self.at) else {
                        return Err("unclosed field value");
                    };
                    if b == b'\\' {
                        self.at = (self.at + 2).min(self.bytes.len());
                        continue;
                    }
                    if depth == 0 && ((open == b'{' && b == b'}') || (open == b'"' && b == b'"')) {
                        break;
                    }
                    if b == b'{' {
                        depth += 1;
                    }
                    if b == b'}' {
                        if depth == 0 {
                            return Err("unbalanced field braces");
                        }
                        depth -= 1;
                    }
                    self.at += 1;
                }
                append_value(&mut out, &self.bytes[at..self.at], &mut self.expanded)?;
                self.at += 1;
            } else {
                let token = self.token();
                if token.is_empty() {
                    return Err("missing field value");
                }
                let mut name = token.clone();
                name.make_ascii_lowercase();
                if token.iter().all(u8::is_ascii_digit) {
                    append_value(&mut out, &token, &mut self.expanded)?;
                } else if let Some((_, value)) = self.strings.iter().rev().find(|(n, _)| n == &name) {
                    append_value(&mut out, value, &mut self.expanded)?;
                } else {
                    return Err("undefined BibTeX string (quote literal field values)");
                }
            }
            self.space();
            if self.bytes.get(self.at) != Some(&b'#') {
                break;
            }
            self.at += 1;
        }
        Ok(out)
    }
}

#[cfg(test)]
fn parse(bytes: &[u8]) -> Result<Vec<Entry>, &'static str> {
    parse_limited(bytes, &mut 0)
}

#[inline(never)]
fn parse_limited(bytes: &[u8], expanded: &mut usize) -> Result<Vec<Entry>, &'static str> {
    let mut p = Parser { bytes, at: 0, strings: Vec::new(), expanded: *expanded };
    for (name, value) in [
        ("jan", "January"),
        ("feb", "February"),
        ("mar", "March"),
        ("apr", "April"),
        ("may", "May"),
        ("jun", "June"),
        ("jul", "July"),
        ("aug", "August"),
        ("sep", "September"),
        ("oct", "October"),
        ("nov", "November"),
        ("dec", "December"),
    ] {
        p.strings.push((name.as_bytes().to_vec(), value.as_bytes().to_vec()));
    }
    let mut entries = Vec::new();
    while p.at < bytes.len() {
        p.space();
        if p.at == bytes.len() {
            break;
        }
        if bytes[p.at] != b'@' {
            p.at += 1;
            continue;
        }
        p.at += 1;
        let mut kind = p.token();
        kind.make_ascii_lowercase();
        p.space();
        let close = match bytes.get(p.at) {
            Some(b'{') => b'}',
            Some(b'(') => b')',
            _ => return Err("expected '{' after entry type"),
        };
        p.at += 1;
        if kind == b"comment" || kind == b"preamble" {
            let mut depth = 1;
            while p.at < bytes.len() && depth > 0 {
                if bytes[p.at] == b'\\' {
                    p.at = (p.at + 2).min(bytes.len());
                    continue;
                }
                if bytes[p.at] == close {
                    depth -= 1;
                }
                if bytes[p.at] == if close == b'}' { b'{' } else { b'(' } {
                    depth += 1;
                }
                p.at += 1;
            }
            if depth != 0 {
                return Err("unclosed bibliography comment or preamble");
            }
            continue;
        }
        let key = p.token();
        if key.is_empty() {
            return Err("missing bibliography key");
        }
        if kind == b"string" {
            p.space();
            if bytes.get(p.at) != Some(&b'=') {
                return Err("expected '=' in string definition");
            }
            p.at += 1;
            let value = p.value()?;
            let mut key = key;
            key.make_ascii_lowercase();
            p.strings.push((key, value));
            p.space();
            if bytes.get(p.at) == Some(&b',') {
                p.at += 1;
                p.space();
            }
            if bytes.get(p.at) != Some(&close) {
                return Err("unclosed string definition");
            }
            p.at += 1;
            continue;
        }
        p.space();
        if bytes.get(p.at) != Some(&b',') {
            return Err("expected ',' after bibliography key");
        }
        p.at += 1;
        let mut fields = Vec::new();
        loop {
            p.space();
            if bytes.get(p.at) == Some(&close) {
                p.at += 1;
                break;
            }
            let mut name = p.token();
            name.make_ascii_lowercase();
            if name.is_empty() {
                return Err("missing field name or closing brace");
            }
            p.space();
            if bytes.get(p.at) != Some(&b'=') {
                return Err("expected '=' after field name");
            }
            p.at += 1;
            if matches!(name.as_slice(), b"crossref" | b"xdata" | b"entryset") {
                return Err("BibTeX crossref/xdata/entryset inheritance is unsupported; inline inherited fields");
            }
            let value = p.value()?;
            fields.push((name, value));
            p.space();
            if bytes.get(p.at) == Some(&b',') {
                p.at += 1;
            } else if bytes.get(p.at) != Some(&close) {
                return Err("expected ',' between fields");
            }
        }
        if entries.len() >= 4096 {
            return Err("bibliography entry limit exceeded (4096 entries)");
        }
        entries.push(Entry { key, fields });
    }
    *expanded = p.expanded;
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_strings_quotes_and_concatenation() {
        let entries = parse(br#"@string{journal = "A Journal"} @article{key, author={Jane Doe and Roe, John}, title={A {Nested} Title}, journal=journal # "!", year=2024, month=jan}"#).unwrap();
        assert_eq!(entries[0].field(b"title"), b"A {Nested} Title");
        assert_eq!(entries[0].field(b"journal"), b"A Journal!");
        assert_eq!(entries[0].authors(false), b"Doe and Roe");
        assert_eq!(entries[0].field(b"month"), b"January");
    }
    #[test]
    fn author_surnames_preserve_unicode_bytes() {
        let entries = parse("@article{a,author={Zoë García and 王小明 and Müller, Jörg}}".as_bytes()).unwrap();
        assert_eq!(entries[0].authors(false), "García et al.".as_bytes());
        assert_eq!(entries[0].authors(true), "García, 王小明, Müller".as_bytes());
        let institution = parse("@book{b,editor={{École de Paris} and Jane Doe}}".as_bytes()).unwrap();
        assert_eq!(institution[0].authors(false), "{École de Paris} and Doe".as_bytes());
    }
    #[test]
    fn truncated_bibliographies_do_not_panic() {
        let bytes = br#"@article{a, author={A {Nested} Author}, title="Title", year=2024}"#;
        for end in 0..bytes.len() {
            let _ = parse(&bytes[..end]);
        }
    }
    #[test]
    fn exponential_string_expansion_has_a_byte_budget() {
        let mut source = String::from("@string{s0 = \"x\"}\n");
        for n in 1..30 {
            source.push_str(&format!("@string{{s{n}=s{} # s{}}}\n", n - 1, n - 1));
        }
        assert!(parse(source.as_bytes()).unwrap_err().contains("expansion limit"));
    }
    #[test]
    fn aggregate_expansion_budget_is_shared_across_assets() {
        let source = format!("@article{{a,title={{{}}}}}", "x".repeat(600_000));
        let mut budget = 0;
        for _ in 0..13 {
            parse_limited(source.as_bytes(), &mut budget).unwrap();
        }
        assert!(parse_limited(source.as_bytes(), &mut budget).unwrap_err().contains("expansion limit"));
    }
    #[test]
    fn repeated_include_subtrees_have_a_discovery_budget() {
        let mut files = Vec::new();
        for n in 0..18 {
            files.push((format!("part{n}.tex").into_bytes(), format!("\\input{{part{}}}\\input{{part{}}}", n + 1, n + 1).into_bytes()));
        }
        files.push((b"part18.tex".to_vec(), b"text".to_vec()));
        let error = Bibliography::prepare(b"\\input{part0}", &files).err().unwrap();
        assert!(String::from_utf8_lossy(&error.0).contains("discovery limit"));
    }
}
