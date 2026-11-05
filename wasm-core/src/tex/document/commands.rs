use super::figure::length_pt;
use super::*;

const MAX_LABELS: usize = 4096;

const COMPOSED: &[(u32, u8, u32)] = &[
    (0xC0, b'A', 0x2CB),
    (0xC1, b'A', 0x2CA),
    (0xC2, b'A', 0x5E),
    (0xC3, b'A', 0x2DC),
    (0xC4, b'A', 0xA8),
    (0xC5, b'A', 0x2DA),
    (0xC7, b'C', 0xB8),
    (0xC8, b'E', 0x2CB),
    (0xC9, b'E', 0x2CA),
    (0xCA, b'E', 0x5E),
    (0xCB, b'E', 0xA8),
    (0xCC, b'I', 0x2CB),
    (0xCD, b'I', 0x2CA),
    (0xCE, b'I', 0x5E),
    (0xCF, b'I', 0xA8),
    (0xD1, b'N', 0x2DC),
    (0xD2, b'O', 0x2CB),
    (0xD3, b'O', 0x2CA),
    (0xD4, b'O', 0x5E),
    (0xD5, b'O', 0x2DC),
    (0xD6, b'O', 0xA8),
    (0xD9, b'U', 0x2CB),
    (0xDA, b'U', 0x2CA),
    (0xDB, b'U', 0x5E),
    (0xDC, b'U', 0xA8),
    (0xDD, b'Y', 0x2CA),
    (0xE0, b'a', 0x2CB),
    (0xE1, b'a', 0x2CA),
    (0xE2, b'a', 0x5E),
    (0xE3, b'a', 0x2DC),
    (0xE4, b'a', 0xA8),
    (0xE5, b'a', 0x2DA),
    (0xE7, b'c', 0xB8),
    (0xE8, b'e', 0x2CB),
    (0xE9, b'e', 0x2CA),
    (0xEA, b'e', 0x5E),
    (0xEB, b'e', 0xA8),
    (0xEC, 0, 0x2CB),
    (0xED, 0, 0x2CA),
    (0xEE, 0, 0x5E),
    (0xEF, 0, 0xA8),
    (0xF1, b'n', 0x2DC),
    (0xF2, b'o', 0x2CB),
    (0xF3, b'o', 0x2CA),
    (0xF4, b'o', 0x5E),
    (0xF5, b'o', 0x2DC),
    (0xF6, b'o', 0xA8),
    (0xF9, b'u', 0x2CB),
    (0xFA, b'u', 0x2CA),
    (0xFB, b'u', 0x5E),
    (0xFC, b'u', 0xA8),
    (0xFD, b'y', 0x2CA),
    (0xFF, b'y', 0xA8),
    (0x100, b'A', 0x2C9),
    (0x101, b'a', 0x2C9),
    (0x102, b'A', 0x2D8),
    (0x103, b'a', 0x2D8),
    (0x106, b'C', 0x2CA),
    (0x107, b'c', 0x2CA),
    (0x10C, b'C', 0x2C7),
    (0x10D, b'c', 0x2C7),
    (0x10E, b'D', 0x2C7),
    (0x10F, b'd', 0x2C7),
    (0x112, b'E', 0x2C9),
    (0x113, b'e', 0x2C9),
    (0x116, b'E', 0x2D9),
    (0x117, b'e', 0x2D9),
    (0x11A, b'E', 0x2C7),
    (0x11B, b'e', 0x2C7),
    (0x11E, b'G', 0x2D8),
    (0x11F, b'g', 0x2D8),
    (0x130, b'I', 0x2D9),
    (0x143, b'N', 0x2CA),
    (0x144, b'n', 0x2CA),
    (0x147, b'N', 0x2C7),
    (0x148, b'n', 0x2C7),
    (0x150, b'O', 0x2DD),
    (0x151, b'o', 0x2DD),
    (0x158, b'R', 0x2C7),
    (0x159, b'r', 0x2C7),
    (0x15A, b'S', 0x2CA),
    (0x15B, b's', 0x2CA),
    (0x15E, b'S', 0xB8),
    (0x15F, b's', 0xB8),
    (0x160, b'S', 0x2C7),
    (0x161, b's', 0x2C7),
    (0x164, b'T', 0x2C7),
    (0x165, b't', 0x2C7),
    (0x16A, b'U', 0x2C9),
    (0x16B, b'u', 0x2C9),
    (0x16E, b'U', 0x2DA),
    (0x16F, b'u', 0x2DA),
    (0x170, b'U', 0x2DD),
    (0x171, b'u', 0x2DD),
    (0x179, b'Z', 0x2CA),
    (0x17A, b'z', 0x2CA),
    (0x17B, b'Z', 0x2D9),
    (0x17C, b'z', 0x2D9),
    (0x17D, b'Z', 0x2C7),
    (0x17E, b'z', 0x2C7),
];

pub(in crate::tex) fn composed(cp: u32) -> Option<(u32, u32)> {
    let i = COMPOSED.binary_search_by_key(&cp, |row| row.0).ok()?;
    let row = COMPOSED[i];
    Some((if row.1 == 0 { 0x131 } else { row.1 as u32 }, row.2))
}

pub(in crate::tex) fn accent_glyph(accent: u32) -> Option<u16> {
    let name: &[u8] = match accent {
        0x2CA => b"acute",
        0x2CB => b"grave",
        0xA8 => b"ddot",
        0x2DC => b"tilde",
        0x2C9 => b"bar",
        0x2D8 => b"breve",
        0x2C7 => b"check",
        0x5E => b"hat",
        0x2D9 => b"dot",
        0x2DA => b"mathring",
        _ => return None,
    };
    lookup(name).map(|sym| sym.glyph)
}

fn accent_of(name: u8) -> Option<u32> {
    Some(match name {
        b'`' => 0x2CB,
        b'\'' => 0x2CA,
        b'^' => 0x5E,
        b'~' => 0x2DC,
        b'"' => 0xA8,
        b'=' => 0x2C9,
        b'.' => 0x2D9,
        b'r' => 0x2DA,
        b'v' => 0x2C7,
        b'u' => 0x2D8,
        b'H' => 0x2DD,
        b'c' => 0xB8,
        _ => return None,
    })
}

const CONTENTS_INDENT: i32 = 1500;
const CONTENTS_PAGE: i32 = 2500;
const STEP_INDENT: i32 = 1500;

#[derive(Clone, Copy)]
pub(super) struct Contents {
    level: u8,
    numbers: [u32; 3],
    page: u32,
    start: u32,
    end: u32,
}

impl PartialEq for Contents {
    fn eq(&self, other: &Self) -> bool {
        (self.level, self.numbers, self.page) == (other.level, other.numbers, other.page)
    }
}

#[derive(Clone, Copy)]
pub(super) struct Step {
    word: &'static [u8],
    tail: &'static [u8],
    arguments: u8,
    before: i32,
    after: i32,
}

const fn line(word: &'static [u8], tail: &'static [u8], arguments: u8, before: i32, after: i32) -> Option<Step> {
    Some(Step { word, tail, arguments, before, after })
}

pub(super) fn step(name: &[u8]) -> Option<Step> {
    match name {
        b"State" | b"STATE" | b"Statex" => line(b"", b"", 0, 0, 0),
        b"If" | b"IF" => line(b"if", b"then", 1, 0, 1),
        b"ElsIf" | b"ELSIF" => line(b"else if", b"then", 1, -1, 1),
        b"Else" | b"ELSE" => line(b"else", b"", 0, -1, 1),
        b"EndIf" | b"ENDIF" => line(b"end if", b"", 0, -1, 0),
        b"For" | b"FOR" => line(b"for", b"do", 1, 0, 1),
        b"ForAll" | b"FORALL" => line(b"for all", b"do", 1, 0, 1),
        b"EndFor" | b"ENDFOR" => line(b"end for", b"", 0, -1, 0),
        b"While" | b"WHILE" => line(b"while", b"do", 1, 0, 1),
        b"EndWhile" | b"ENDWHILE" => line(b"end while", b"", 0, -1, 0),
        b"Loop" | b"LOOP" => line(b"loop", b"", 0, 0, 1),
        b"EndLoop" | b"ENDLOOP" => line(b"end loop", b"", 0, -1, 0),
        b"Repeat" | b"REPEAT" => line(b"repeat", b"", 0, 0, 1),
        b"Until" | b"UNTIL" => line(b"until", b"", 1, -1, 0),
        b"Procedure" => line(b"procedure", b"", 2, 0, 1),
        b"Function" => line(b"function", b"", 2, 0, 1),
        b"EndProcedure" => line(b"end procedure", b"", 0, -1, 0),
        b"EndFunction" => line(b"end function", b"", 0, -1, 0),
        b"Require" | b"REQUIRE" => line(b"Require:", b"", 0, 0, 0),
        b"Ensure" | b"ENSURE" => line(b"Ensure:", b"", 0, 0, 0),
        _ => None,
    }
}

pub(super) fn silent_setting(name: &[u8]) -> bool {
    matches!(
        name,
        b"appendix"
            | b"mainmatter"
            | b"frontmatter"
            | b"backmatter"
            | b"listoffigures"
            | b"listoftables"
            | b"printbibliography"
            | b"printindex"
            | b"makeindex"
            | b"phantomsection"
            | b"doublespacing"
            | b"onehalfspacing"
            | b"singlespacing"
            | b"pause"
            | b"mode"
            | b"pagestyle"
            | b"thispagestyle"
            | b"pagenumbering"
            | b"bibliographystyle"
            | b"selectfont"
            | b"fontsize"
            | b"fontfamily"
            | b"begingroup"
            | b"endgroup"
            | b"Description"
            | b"AtBeginSubsection"
            | b"AtBeginSubsubsection"
            | b"AtEndDocument"
            | b"setbeamersize"
            | b"addtolength"
            | b"subtitle"
            | b"subject"
            | b"institute"
            | b"titlegraphic"
            | b"affil"
            | b"affiliation"
            | b"address"
            | b"bibliography"
            | b"addbibresource"
            | b"nocite"
            | b"setstretch"
            | b"selectlanguage"
            | b"pdfbookmark"
            | b"addtocontents"
            | b"addcontentsline"
            | b"addtocounter"
            | b"stepcounter"
            | b"refstepcounter"
            | b"newcounter"
            | b"newlength"
            | b"newsavebox"
            | b"hypersetup"
            | b"lstset"
            | b"sisetup"
            | b"markboth"
            | b"markright"
            | b"makeatletter"
            | b"makeatother"
            | b"vfill"
            | b"vfil"
            | b"fussy"
            | b"frenchspacing"
            | b"nonfrenchspacing"
            | b"linespread"
            | b"baselineskip"
            | b"hbadness"
            | b"vbadness"
            | b"hyphenation"
            | b"nopagebreak"
            | b"enlargethispage"
            | b"glsaddall"
            | b"makeglossaries"
            | b"printglossaries"
            | b"usetheme"
            | b"usecolortheme"
            | b"usefonttheme"
            | b"useinnertheme"
            | b"useoutertheme"
            | b"setbeamertemplate"
            | b"setbeamercolor"
            | b"setbeamerfont"
            | b"beamertemplatenavigationsymbolsempty"
            | b"AtBeginSection"
            | b"AtBeginDocument"
            | b"fancyhf"
            | b"fancyhead"
            | b"fancyfoot"
            | b"lhead"
            | b"chead"
            | b"rhead"
            | b"lfoot"
            | b"cfoot"
            | b"rfoot"
            | b"setlist"
            | b"newlist"
            | b"newtoggle"
            | b"toggletrue"
            | b"togglefalse"
            | b"setboolean"
            | b"newboolean"
            | b"titleformat"
            | b"titlelabel"
            | b"pdfstringdefDisableCommands"
            | b"pdfinfo"
            | b"pgfplotsset"
            | b"usepgfplotslibrary"
            | b"tcbuselibrary"
            | b"tcbset"
            | b"bibpunct"
            | b"citestyle"
            | b"setcitestyle"
            | b"DeclareGraphicsExtensions"
            | b"graphicspath"
            | b"linenumbers"
            | b"nolinenumbers"
            | b"theoremstyle"
            | b"newlist*"
            | b"lstdefinestyle"
            | b"lstdefinelanguage"
            | b"DeclareCaptionFormat"
            | b"DeclareCaptionLabelFormat"
    )
}

impl Engine {
    #[inline(always)]
    pub(super) fn set_marker_number(&mut self, n: u32) {
        self.mark = (1, [n, 0, 0]);
    }

    #[inline(always)]
    pub(super) fn set_marker_section(&mut self, level: usize, sections: [u32; 3]) {
        self.mark = (level as u8 + 1, sections);
    }

    pub(super) fn add_label(&mut self, key: &[u8]) {
        if self.labels.len() >= MAX_LABELS {
            return;
        }
        let start = self.label_bytes.len() as u32;
        self.label_bytes.extend_from_slice(key);
        let mid = self.label_bytes.len() as u32;
        let (count, parts) = self.mark;
        for i in 0..count as usize {
            if i > 0 {
                self.label_bytes.push(b'.');
            }
            push_number(&mut self.label_bytes, parts[i]);
        }
        self.labels.push((start, mid, self.label_bytes.len() as u32));
    }

    fn find_label(&self, key: &[u8]) -> Option<(usize, usize)> {
        self.labels.iter().rev().find_map(|row| {
            let (s, m, e) = (row.0 as usize, row.1 as usize, row.2 as usize);
            (&self.label_bytes[s..m] == key).then_some((m, e))
        })
    }

    pub(super) fn scan_labels(&mut self, text: &[u8]) {
        let mut i = 0;
        while let Some(at) = text[i..].windows(7).position(|w| w == b"\\label{") {
            let s = i + at + 7;
            let Some(n) = text[s..].iter().position(|c| *c == b'}') else {
                return;
            };
            self.add_label(&text[s..s + n]);
            i = s + n;
        }
    }

    fn put_text(&mut self, d: &mut Doc, c: &Ctx, font: u8, text: &[u8]) -> R<()> {
        for ch in text {
            if *ch == b' ' {
                self.flush_word(d, false);
            } else {
                self.put_char(d, c, font, *ch as u32)?;
            }
        }
        Ok(())
    }

    fn put_reference(&mut self, d: &mut Doc, c: &Ctx, font: u8, key: &[u8]) -> R<()> {
        match self.find_label(key) {
            Some((s, e)) => {
                let text = self.label_bytes[s..e].to_vec();
                self.put_text(d, c, font, &text)
            }
            None => {
                self.unresolved = true;
                self.put_text(d, c, font, b"??")
            }
        }
    }

    fn put_citation(&mut self, d: &mut Doc, c: &Ctx, font: u8, keys: &'static [u8]) -> R<()> {
        self.put_char(d, c, font, b'[' as u32)?;
        let mut first = true;
        for key in keys.split(|ch| *ch == b',') {
            let key = key.trim_ascii();
            if !first {
                self.put_char(d, c, font, b',' as u32)?;
                self.flush_word(d, false);
            }
            first = false;
            let mut tagged = Vec::with_capacity(key.len() + 5);
            tagged.extend_from_slice(b"cite:");
            tagged.extend_from_slice(key);
            if let Some((s, e)) = self.find_label(&tagged) {
                let text = self.label_bytes[s..e].to_vec();
                self.put_text(d, c, font, &text)?;
                continue;
            }
            let n = match self.cites.iter().position(|known| known.as_slice() == key) {
                Some(i) => i,
                None => {
                    self.cites.push(key.to_vec());
                    self.cites.len() - 1
                }
            };
            self.put_number(d, c, font, n as u32 + 1)?;
        }
        self.put_char(d, c, font, b']' as u32)
    }

    fn put_raw(&mut self, d: &mut Doc, c: &Ctx, text: &[u8]) -> R<()> {
        for ch in text {
            match *ch {
                b' ' | b'\n' => self.add_kern(d, sc(525, c.scale)),
                b'\t' => self.add_kern(d, sc(2100, c.scale)),
                32..=126 => self.put_char(d, c, FONT_TYPEWRITER_REGULAR, *ch as u32)?,
                _ => {}
            }
        }
        Ok(())
    }

    fn put_template(&mut self, d: &mut Doc, c: &Ctx, font: u8, text: &[u8]) -> R<()> {
        let text = self.source_copy(text);
        self.sub_text(d, text, c, font)
    }

    fn graphic(&mut self, d: &mut Doc, c: &Ctx, font: u8) -> R<()> {
        let options = if self.peek() == b'[' { self.bracketed()? } else { b"" };
        self.loose_argument();
        let line = d.text_w as f64 * self.doc.pt as f64 / 1_000_000.0;
        let (mut w, mut h) = (0.0, 0.0);
        let mut at = 0;
        while let Some((key, value)) = figure::next_option(options, &mut at) {
            let size = relative_length(value, line);
            match key {
                b"width" => w = size,
                b"height" => h = size,
                b"scale" => w = size * 200.0,
                _ => {}
            }
        }
        if w <= 0.0 {
            w = if h > 0.0 { h * 1.5 } else { 200.0 };
        }
        if h <= 0.0 {
            h = w * 0.62;
        }
        let mut text = Vec::with_capacity(64);
        text.extend_from_slice(b"$\\boxed{\\rule{");
        push_number(&mut text, w.min(2000.0) as u32);
        text.extend_from_slice(b"pt}{0pt}\\rule{0pt}{");
        push_number(&mut text, h.min(2000.0) as u32);
        text.extend_from_slice(b"pt}}$");
        self.put_template(d, c, font, &text)
    }

    fn accented(&mut self, d: &mut Doc, c: &Ctx, font: u8, accent: u32) -> R<()> {
        self.skip_ws();
        let braced = self.peek() == b'{';
        if braced {
            self.pos += 1;
            self.skip_ws();
        }
        let mut base = self.peek() as u32;
        if base == b'\\' as u32 {
            self.pos += 1;
            base = match self.read_name() {
                b"i" => 0x131,
                b"j" => 0x237,
                _ => b'?' as u32,
            };
        } else if base >= 0x80 {
            base = self.read_utf8()?.1;
        } else {
            self.pos += 1;
        }
        if braced {
            self.skip_ws();
            if self.peek() == b'}' {
                self.pos += 1;
            }
        }
        if base == b'i' as u32 {
            base = 0x131;
        }
        let mut t = d.word.take().unwrap_or_else(|| self.close(self.end(), 0, 0, 0));
        self.overlay(c, font, base, accent, &mut t)?;
        t.e = self.end();
        d.word = Some(t);
        Ok(())
    }

    fn footnote(&mut self, d: &mut Doc, c: &Ctx, font: u8) -> R<()> {
        self.skip_options();
        let text = self.loose_argument();
        let text = self.lasting(text);
        d.notes += 1;
        let n = d.notes;
        let mut mark = Vec::with_capacity(24);
        mark.extend_from_slice(b"$^{");
        push_number(&mut mark, n);
        mark.extend_from_slice(b"}$");
        self.put_template(d, c, font, &mark)?;
        self.store.notes.push((n, text));
        Ok(())
    }

    pub(super) fn flush_notes(&mut self, d: &mut Doc, left: i32, width: i32) -> R<()> {
        if self.store.notes.is_empty() {
            return Ok(());
        }
        let notes = core::mem::take(&mut self.store.notes);
        let small = font_size(b"footnotesize", self.doc.pt).unwrap_or(800);
        let c = self.sized(d, small);
        for (n, text) in &notes {
            d.skip += 200;
            self.block_reset();
            let mut mark = Vec::with_capacity(24);
            mark.extend_from_slice(b"$^{");
            push_number(&mut mark, *n);
            mark.extend_from_slice(b"}$");
            self.put_template(d, &c, 0, &mark)?;
            self.flush_word(d, false);
            self.sub_text(d, text, &c, 0)?;
            self.flush_word(d, false);
            self.set_lines(d, &c, left, width, 0, false, false);
        }
        self.store.notes = notes;
        self.store.notes.clear();
        Ok(())
    }

    #[inline(never)]
    pub(super) fn add_contents(&mut self, d: &Doc, level: usize, title: usize) {
        let text = self.slice(title, self.pos.saturating_sub(1).max(title));
        let store = &mut self.store;
        let start = store.contents_text.len() as u32;
        store.contents_text.extend_from_slice(text);
        let end = store.contents_text.len() as u32;
        store.contents.push(Contents { level: level as u8, numbers: d.sections, page: d.page as u32, start, end });
    }

    pub(super) fn contents(&mut self, d: &mut Doc, left: i32, width: i32) -> R<()> {
        let title = self.source_copy(b"*{Contents}");
        let src = self.push_src(title.as_ptr(), title.len());
        self.heading(d, 0, left, width)?;
        self.pop_src(src);
        let c = d.ctx;
        for i in 0..self.store.shown.len() {
            let row = self.store.shown[i];
            let level = row.level as usize;
            let shift = level as i32 * CONTENTS_INDENT;
            let font = if level == 0 { FONT_MAIN_BOLD } else { 0 };
            self.items.clear();
            self.put_number(d, &c, font, row.page)?;
            let Some(page) = d.word.take() else {
                continue;
            };
            for k in 0..=level {
                if k > 0 {
                    self.put_char(d, &c, font, b'.' as u32)?;
                }
                self.put_number(d, &c, font, row.numbers[k])?;
            }
            self.add_kern(d, sc(1000, c.scale));
            let text = self.store.contents_text[row.start as usize..row.end as usize].to_vec();
            let text = self.source_copy(&text);
            self.sub_text(d, text, &c, font)?;
            self.flush_word(d, false);
            self.set_lines(d, &c, left + shift, width - shift - CONTENTS_PAGE, 0, true, false);
            self.commit(d, page.s, page.e, left + width - page.w, d.base);
        }
        d.noindent = true;
        d.skip += d.baseline;
        Ok(())
    }

    fn char_code(&mut self) -> u32 {
        self.skip_ws();
        let (radix, skip) = match self.peek() {
            b'"' => (16, 1),
            b'\'' => (8, 1),
            _ => (10, 0),
        };
        self.pos += skip;
        let mut code = 0u32;
        while let Some(digit) = (self.peek() as char).to_digit(radix) {
            code = code.saturating_mul(radix).saturating_add(digit);
            self.pos += 1;
        }
        code
    }

    fn put_keyword(&mut self, d: &mut Doc, c: &Ctx, word: &[u8]) -> R<()> {
        for ch in word {
            if *ch == b' ' {
                self.flush_word(d, false);
            } else {
                self.put_char(d, c, FONT_MAIN_BOLD, *ch as u32)?;
            }
        }
        Ok(())
    }

    pub(super) fn algorithm_line(&mut self, d: &mut Doc, left: i32, width: i32, name: &[u8]) -> R<()> {
        let Some(row) = step(name) else {
            return Ok(());
        };
        let c = d.ctx;
        d.step_level = (d.step_level + row.before).max(0);
        let shift = d.step_level * STEP_INDENT;
        self.put_keyword(d, &c, row.word)?;
        self.flush_word(d, false);
        if row.arguments > 0 {
            self.styled_group(d, &c, 0)?;
        }
        if row.arguments > 1 {
            self.put_char(d, &c, 0, b'(' as u32)?;
            self.styled_group(d, &c, 0)?;
            self.put_char(d, &c, 0, b')' as u32)?;
        }
        self.flush_word(d, false);
        self.put_keyword(d, &c, row.tail)?;
        self.flush_word(d, false);
        d.step_level += row.after;
        d.noindent = true;
        self.paragraph(d, left + shift, width - shift)
    }

    pub(super) fn text_command(&mut self, d: &mut Doc, c: &mut Ctx, font: &mut u8, name: &'static [u8], tok: usize) -> R<bool> {
        if name.len() == 1 {
            if let Some(accent) = accent_of(name[0]) {
                self.accented(d, c, *font, accent)?;
                return Ok(true);
            }
        }
        match name {
            b"," | b";" | b":" | b"!" | b" " | b"/" | b"-" | b"\n" | b"\t" => {
                if name == b" " || name == b"\n" {
                    self.flush_word(d, false);
                } else if name != b"-" && name != b"/" && name != b"!" {
                    self.add_kern(d, sc(170, c.scale));
                }
            }
            b"newline" | b"linebreak" | b"break" | b"crlf" => {
                self.skip_options();
                self.flush_word(d, true);
            }
            b"footnote" | b"thanks" => self.footnote(d, c, *font)?,
            b"footnotemark" | b"footnotetext" | b"protect" | b"strut" | b"leavevmode" | b"ignorespaces" | b"unskip" | b"null" | b"hfil"
            | b"nobreakspace" | b"sloppy" | b"noindent" | b"indent" | b"par" | b"centering" | b"raggedright" | b"raggedleft"
            | b"bigskip" | b"medskip" | b"smallskip" | b"vspace" | b"pagebreak" | b"newpage" | b"clearpage" | b"index" => {
                self.skip_arguments()
            }
            b"hfill" | b"dotfill" | b"hrulefill" | b"qquad" => self.add_kern(d, sc(2000, c.scale)),
            b"ref" | b"pageref" | b"autoref" | b"nameref" | b"cref" | b"Cref" | b"vref" | b"subref" => {
                self.skip_star();
                let key = self.loose_argument();
                self.put_reference(d, c, *font, key)?;
            }
            b"eqref" => {
                let key = self.loose_argument();
                self.put_char(d, c, *font, b'(' as u32)?;
                self.put_reference(d, c, *font, key)?;
                self.put_char(d, c, *font, b')' as u32)?;
            }
            b"cite" | b"citep" | b"citet" | b"parencite" | b"textcite" | b"autocite" | b"citeauthor" | b"citeyear" | b"footcite"
            | b"Cite" | b"Citep" | b"Citet" => {
                self.skip_star();
                self.skip_ws();
                self.skip_options();
                let keys = self.loose_argument();
                self.put_citation(d, c, *font, keys)?;
            }
            b"url" | b"nolinkurl" | b"path" => {
                let text = self.loose_argument();
                self.put_raw(d, c, text)?;
            }
            b"href" => {
                self.loose_argument();
                self.styled_group(d, c, *font)?;
            }
            b"hyperref" | b"hyperlink" | b"hypertarget" => {
                self.skip_options();
                if name != b"hyperref" {
                    self.loose_argument();
                }
                self.styled_group(d, c, *font)?;
            }
            b"verb" | b"lstinline" | b"mintinline" => {
                self.skip_star();
                self.skip_options();
                if name == b"mintinline" {
                    self.loose_argument();
                }
                let close = self.peek();
                self.pos += 1;
                let s = self.pos;
                let stop = if close == b'{' { b'}' } else { close };
                while !self.at_end() && self.peek() != stop && self.peek() != b'\n' {
                    self.pos += 1;
                }
                let text = self.slice(s, self.pos);
                self.pos += 1;
                self.put_raw(d, c, text)?;
            }
            b"textsc" | b"textmd" | b"textup" | b"textnormal" | b"textrm" | b"mbox" | b"hbox" | b"makebox" | b"uline" | b"underline"
            | b"ul" | b"sout" | b"st" | b"uwave" | b"enquote" | b"foreignlanguage" | b"MakeUppercase" | b"MakeLowercase"
            | b"MakeTextUppercase" | b"uppercase" | b"lowercase" | b"emph" | b"textls" | b"fbox" | b"framebox" => {
                if self.peek() == b'(' {
                    self.skip_pair_of_parens();
                }
                self.skip_options();
                if name == b"foreignlanguage" {
                    self.loose_argument();
                }
                self.styled_group(d, c, *font)?;
            }
            b"textsl" => self.styled_group(d, c, FONT_MAIN_ITALIC)?,
            b"onslide" => self.skip_overlay(),
            b"scshape" | b"sc" | b"upshape" | b"mdseries" => {}
            b"slshape" | b"sl" => *font = FONT_MAIN_ITALIC,
            b"@" => {}
            b"rule" => {
                self.pos = tok;
                let b = self.arg(c)?;
                self.add_box(d, b);
            }
            b"char" => {
                let cp = self.char_code();
                self.put_char(d, c, *font, cp)?;
            }
            b"Comment" | b"COMMENT" => {
                self.add_kern(d, sc(1000, c.scale));
                self.styled_group(d, c, FONT_MAIN_ITALIC)?;
            }
            b"Return" | b"RETURN" => self.put_keyword(d, c, b"return")?,
            b"True" | b"TRUE" => self.put_keyword(d, c, b"true")?,
            b"False" | b"FALSE" => self.put_keyword(d, c, b"false")?,
            b"OR" => self.put_keyword(d, c, b"or")?,
            b"NOT" => self.put_keyword(d, c, b"not")?,
            b"Call" => {
                self.styled_group(d, c, *font)?;
                self.put_char(d, c, *font, b'(' as u32)?;
                self.styled_group(d, c, *font)?;
                self.put_char(d, c, *font, b')' as u32)?;
            }
            b"centerline" | b"alert" | b"structure" | b"leftline" | b"rightline" | b"text" | b"only" | b"uncover" | b"visible"
            | b"email" | b"keywords" | b"invisible" => {
                self.skip_overlay();
                self.skip_ws();
                self.skip_options();
                if self.peek() == b'{' {
                    self.styled_group(d, c, *font)?;
                }
            }
            b"lipsum" | b"blindtext" | b"Blindtext" => {
                self.skip_options();
                let text = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.";
                self.put_text(d, c, *font, text)?;
            }
            b"begin" => {
                self.note(tok);
                let inner = self.loose_argument();
                self.skip_environment(inner);
            }
            b"parbox" | b"raisebox" => {
                self.skip_options();
                self.loose_argument();
                self.skip_options();
                self.styled_group(d, c, *font)?;
            }
            b"colorbox" => {
                self.loose_argument();
                self.styled_group(d, c, *font)?;
            }
            b"textcolor" => {
                let held = self.pos;
                let mut inner = *c;
                match self.color_arg() {
                    Ok(color) => {
                        inner.color = color;
                        inner.grp = self.grp(color, inner.scale);
                    }
                    Err(()) => {
                        self.err = 0;
                        self.pos = held;
                        self.note(tok);
                        self.loose_argument();
                    }
                }
                self.styled_group(d, &inner, *font)?;
            }
            b"color" => {
                let held = self.pos;
                match self.color_arg() {
                    Ok(color) => {
                        c.color = color;
                        c.grp = self.grp(color, c.scale);
                    }
                    Err(()) => {
                        self.err = 0;
                        self.pos = held;
                        self.note(tok);
                        self.loose_argument();
                    }
                }
            }
            b"textsuperscript" | b"textsubscript" => {
                let text = self.loose_argument();
                let mut out = Vec::with_capacity(text.len() + 16);
                out.extend_from_slice(if name == b"textsuperscript" { b"$^{\\text{" } else { b"$_{\\text{" });
                out.extend_from_slice(text);
                out.extend_from_slice(b"}}$");
                self.put_template(d, c, *font, &out)?;
            }
            b"includegraphics" => {
                self.skip_star();
                self.skip_ws();
                self.graphic(d, c, *font)?;
            }
            b"multicolumn" => {
                self.loose_argument();
                self.loose_argument();
                self.styled_group(d, c, *font)?;
            }
            b"multirow" => {
                self.skip_options();
                self.loose_argument();
                self.skip_options();
                self.loose_argument();
                self.styled_group(d, c, *font)?;
            }
            b"dots" | b"ldots" | b"textellipsis" => self.put_char(d, c, *font, 0x2026)?,
            b"LaTeXe" => self.put_text(d, c, *font, b"LaTeX2e")?,
            b"XeTeX" | b"XeLaTeX" | b"LuaTeX" | b"BibTeX" | b"pdfTeX" | b"pdfLaTeX" => self.put_text(d, c, *font, name)?,
            b"thepage" => {
                let page = d.page;
                self.put_number(d, c, *font, page)?;
            }
            b"and" | b"AND" => self.flush_word(d, true),
            _ if silent_setting(name) => self.skip_arguments(),
            _ => return Ok(false),
        }
        Ok(true)
    }

    pub(super) fn verbatim_block(&mut self, d: &mut Doc, left: i32, width: i32, name: &'static [u8]) -> R<()> {
        self.skip_options();
        if self.peek() == b'{' {
            self.skip_arguments();
        }
        let s = self.pos;
        self.skip_environment(name);
        let closing = name.len() + b"\\end{}".len();
        let e = self.pos.saturating_sub(closing).max(s);
        let text = self.slice(s, e);
        let small = font_size(b"small", self.doc.pt).unwrap_or(900);
        let c = self.sized(d, small);
        self.block_reset();
        for line in text.trim_ascii_end().split(|ch| *ch == b'\n') {
            let line = line.strip_suffix(b"\r").unwrap_or(line);
            if line.is_empty() && d.word.is_none() {
                self.add_kern(d, sc(525, c.scale));
            }
            self.put_raw(d, &c, line)?;
            self.flush_word(d, true);
        }
        d.skip += 300;
        self.set_lines(d, &c, left, width, 0, true, false);
        d.skip += 300;
        Ok(())
    }

    pub(super) fn define_environment(&mut self) -> R<()> {
        self.skip_star();
        let name = self.loose_argument();
        self.skip_ws();
        let mut count = 0;
        if self.peek() == b'[' {
            let n = self.bracketed()?;
            count = n.first().map_or(0, |c| c.wrapping_sub(b'0')).min(9) as usize;
            self.skip_ws();
        }
        let mut default = None;
        if self.peek() == b'[' {
            default = Some(self.bracketed()?);
            self.skip_ws();
        }
        let begin = self.loose_argument();
        let end = self.loose_argument();
        self.store.environments.retain(|row| row.name != name);
        self.store.environments.push(Environment { name, count, default, begin, end });
        Ok(())
    }

    pub(super) fn run_environment(&mut self, d: &mut Doc, left: i32, narrow: i32, row: Environment) -> R<()> {
        self.enter()?;
        let mut args: [&'static [u8]; 9] = [b""; 9];
        for (i, slot) in args.iter_mut().enumerate().take(row.count) {
            self.skip_ws();
            if i == 0 {
                if let Some(default) = row.default {
                    *slot = if self.peek() == b'[' { self.bracketed()? } else { default };
                    continue;
                }
            }
            *slot = self.loose_argument();
        }
        let s = self.pos;
        let mut depth = 0;
        let mut e = self.len;
        let name = row.name;
        while !self.at_end() {
            if self.peek() != b'\\' {
                self.pos += 1;
                continue;
            }
            let at = self.pos;
            self.pos += 1;
            let word = self.read_name();
            if word != b"begin" && word != b"end" {
                continue;
            }
            let inner = self.loose_argument();
            if inner == name {
                if word == b"begin" {
                    depth += 1;
                } else if depth == 0 {
                    e = at;
                    break;
                } else {
                    depth -= 1;
                }
            }
        }
        let body = self.slice(s, e);
        let mut text = Vec::with_capacity(row.begin.len() + body.len() + row.end.len() + 2);
        macros::substitute(row.begin, &args, &mut text);
        text.push(b'\n');
        text.extend_from_slice(body);
        text.push(b'\n');
        macros::substitute(row.end, &args, &mut text);
        let text = self.source_copy(&text);
        let src = self.push_src(text.as_ptr(), text.len());
        self.blocks(d, left, narrow, b"")?;
        self.pop_src(src);
        self.depth -= 1;
        Ok(())
    }

    pub(super) fn define_theorem(&mut self) -> R<()> {
        self.skip_star();
        let name = self.loose_argument();
        self.skip_ws();
        self.skip_options();
        let title = self.loose_argument();
        self.skip_ws();
        self.skip_options();
        self.store.theorems.retain(|row| row.0 != name);
        self.store.theorems.push((name, title, 0));
        Ok(())
    }
}

fn push_number(out: &mut Vec<u8>, n: u32) {
    if n >= 10 {
        push_number(out, n / 10);
    }
    out.push(b'0' + (n % 10) as u8);
}

fn relative_length(value: &[u8], line: f64) -> f64 {
    let text = value.trim_ascii();
    for unit in [&b"\\textwidth"[..], b"\\linewidth", b"\\columnwidth", b"\\paperwidth", b"\\textheight"] {
        if let Some(head) = text.strip_suffix(unit) {
            let head = head.trim_ascii();
            let factor = if head.is_empty() { Some(1.0) } else { figure::evaluate(head, 0.0, 0.0).map(|v| v.0) };
            return factor.unwrap_or(1.0) * line;
        }
    }
    length_pt(text).or_else(|| figure::evaluate(text, 0.0, 0.0).map(|v| v.0)).unwrap_or(0.0)
}
