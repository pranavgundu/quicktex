use super::*;

impl Engine {
    pub(super) fn page_setting(&mut self, d: &mut Doc, name: &[u8]) -> R<bool> {
        if matches!(name, b"pagestyle" | b"thispagestyle") {
            let style = self.loose_argument();
            let value = match style {
                b"empty" => 0,
                b"plain" => 1,
                b"fancy" => 2,
                _ => {
                    self.note(self.pos);
                    return Ok(true);
                }
            };
            if name == b"thispagestyle" {
                d.this_page_style = Some(value);
            } else {
                d.page_style = value;
            }
            return Ok(true);
        }
        let target = match name {
            b"fancyhf" => 0,
            b"fancyhead" => 1,
            b"fancyfoot" => 2,
            b"lhead" | b"chead" | b"rhead" => 3,
            b"lfoot" | b"cfoot" | b"rfoot" => 4,
            _ => return Ok(false),
        };
        self.skip_ws();
        let option = if self.peek() == b'[' { Some(self.bracketed()?) } else { None };
        let text = self.loose_argument();
        if target >= 3 {
            let col = match name[0] {
                b'l' => 0,
                b'c' => 1,
                _ => 2,
            };
            let area = if target == 3 { 0 } else { 1 };
            d.page_fields[0][area][col] = text;
            d.page_fields[1][area][col] = option.unwrap_or(text);
            return Ok(true);
        }
        let selectors = option.unwrap_or(b"");
        for selector in selectors.split(|ch| *ch == b',') {
            let selector = selector.trim_ascii();
            let has = |ch: u8| selector.iter().any(|s| s.to_ascii_uppercase() == ch);
            if selector.iter().any(|s| !b"LCRHEOFlcrheof ".contains(s)) {
                self.note(self.pos);
                continue;
            }
            for parity in 0..2 {
                if (has(b'O') || has(b'E')) && !has(if parity == 0 { b'O' } else { b'E' }) {
                    continue;
                }
                for area in 0..2 {
                    if target == 1 && area != 0 || target == 2 && area != 1 {
                        continue;
                    }
                    if (has(b'H') || has(b'F')) && !has(if area == 0 { b'H' } else { b'F' }) {
                        continue;
                    }
                    for col in 0..3 {
                        if (has(b'L') || has(b'C') || has(b'R')) && !has(b"LCR"[col]) {
                            continue;
                        }
                        d.page_fields[parity][area][col] = text;
                    }
                }
            }
        }
        Ok(true)
    }

    fn page_rule_width(&self, name: &[u8], fallback: f64) -> i32 {
        let value = self.defs.iter().rev().find(|def| def.name == name).and_then(|def| figure::length_pt(def.body)).unwrap_or(fallback);
        self.pt_units(value).max(0)
    }

    pub(super) fn page_furniture(&mut self, d: &mut Doc) -> R<()> {
        let style = d.this_page_style.take().unwrap_or(d.page_style);
        if d.standalone.is_some() || style == 0 {
            return Ok(());
        }
        if style == 1 {
            let c = d.ctx;
            let saved = d.word.take();
            let mark = self.end();
            let (left, top) = (d.left, d.top);
            d.left = d.frame_left;
            d.top = d.frame_top;
            let result = self.put_number(d, &c, 0, d.page);
            if let Some(t) = d.word.take() {
                self.commit(d, t.s, t.e, (d.frame_w - t.w) / 2, d.frame_h + d.foot_skip);
            }
            d.left = left;
            d.top = top;
            self.items.truncate(mark as usize);
            d.word = saved;
            return result;
        }
        let mut furniture = d.clone();
        furniture.left = d.frame_left;
        furniture.top = d.frame_top;
        furniture.word = None;
        furniture.glue_next = false;
        furniture.flowing = false;
        let c = self.sized(d, UNIT);
        let saved_words = core::mem::take(&mut self.words);
        let mark = self.end();
        let fields = d.page_fields[((d.page + 1) % 2) as usize];
        let result = (|| {
            for area in 0..2 {
                let y = if area == 0 { -d.head_sep } else { d.frame_h + d.foot_skip };
                for col in 0..3 {
                    let text = fields[area][col];
                    if text.is_empty() {
                        continue;
                    }
                    let text = self.source_copy(text);
                    self.sub_text(&mut furniture, text, &c, 0)?;
                    let b = self.line_box(&mut furniture, &c);
                    let x = match col {
                        0 => 0,
                        1 => (d.frame_w - b.w) / 2,
                        _ => d.frame_w - b.w,
                    };
                    self.commit(&furniture, b.s, b.e, x, y);
                }
                if style == 2 && fields[area].iter().any(|text| !text.is_empty()) {
                    let thick = self
                        .page_rule_width(if area == 0 { b"headrulewidth" } else { b"footrulewidth" }, if area == 0 { 0.4 } else { 0.0 });
                    if thick > 0 {
                        let start = self.end();
                        self.push_rule(&c, 0, if area == 0 { -d.head_sep / 2 } else { d.frame_h + d.foot_skip / 2 }, d.frame_w, thick);
                        self.commit(&furniture, start, self.end(), 0, 0);
                    }
                }
            }
            Ok(())
        })();
        self.items.truncate(mark as usize);
        self.words = saved_words;
        result
    }
}
