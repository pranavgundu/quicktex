use super::*;
const PATTERNS: &[u8] = include_bytes!("../../../assets/hyph-en-us.bin");
fn uint(at: usize) -> usize {
    u16::from_le_bytes(PATTERNS[at..at + 2].try_into().unwrap()) as usize
}
fn points(word: &[u8]) -> Vec<usize> {
    let mut text = Vec::with_capacity(word.len() + 2);
    text.push(b'.');
    text.extend(word.iter().map(u8::to_ascii_lowercase));
    text.push(b'.');
    let mut values = alloc::vec![0u8;text.len()+1];
    let weight_base = 4 + u32::from_le_bytes(PATTERNS[..4].try_into().unwrap()) as usize * 8;
    for start in 0..text.len() {
        let mut node = 0;
        for ch in &text[start..] {
            let mut child = uint(4 + node * 8);
            while child != 0 && PATTERNS[4 + child * 8 + 6] != *ch {
                child = uint(4 + child * 8 + 2);
            }
            if child == 0 {
                break;
            }
            node = child;
            let at = 4 + node * 8;
            let offset = weight_base + uint(at + 4);
            let len = PATTERNS[at + 7] as usize;
            for j in 0..len {
                if start + j < values.len() {
                    values[start + j] = values[start + j].max(PATTERNS[offset + j]);
                }
            }
        }
    }
    (2..word.len().saturating_sub(2)).filter(|i| values[i + 1] % 2 == 1).collect()
}
impl Engine {
    pub(super) fn hyphenation_exceptions(&mut self) -> R<()> {
        let text = self.group_text()?;
        for word in text.split(|c| c.is_ascii_whitespace()).filter(|w| !w.is_empty()) {
            let mut key = Vec::new();
            let mut breaks = Vec::new();
            for ch in word {
                if *ch == b'-' {
                    breaks.push(key.len());
                } else {
                    key.push(ch.to_ascii_lowercase());
                }
            }
            self.store.hyphenations.push((key, breaks));
        }
        Ok(())
    }

    pub(super) fn hyphenate_words(&mut self) -> bool {
        let old = core::mem::take(&mut self.words);
        let mut changed = false;
        for word in old {
            let b = word.bx;
            let items = &self.items[b.s as usize..b.e as usize];
            if items.len() < 5
                || word.hyphen.is_some()
                || items.iter().any(|it| it.kind != IT_GLYPH || !(it.b as u8).is_ascii_alphabetic() || it.b > 127 || it.y != 0)
            {
                self.words.push(word);
                continue;
            }
            let text: Vec<u8> = items.iter().map(|it| it.b as u8).collect();
            let lower: Vec<u8> = text.iter().map(u8::to_ascii_lowercase).collect();
            let exceptions = self.store.hyphenations.iter().rev().find(|(word, _)| *word == lower);
            let breaks = if let Some((_, breaks)) = exceptions { breaks.clone() } else { points(&text) };
            if breaks.is_empty() {
                self.words.push(word);
                continue;
            }
            let mut from = 0;
            let mut origin = 0;
            let group = self.items[b.s as usize].g;
            let scale = self.groups[group as usize].scale;
            let hyphen = glyph_id(FONT_MAIN_REGULAR, b'-' as u32).unwrap();
            let metric = self.glyph_metric(hyphen as usize);
            for to in breaks.into_iter().chain(core::iter::once(text.len())) {
                let start = b.s + from as u32;
                let end = b.s + to as u32;
                let width = if to == text.len() { b.w - origin } else { self.items[end as usize].x - origin };
                self.shift(start, end, -origin, 0);
                let mark = if to < text.len() {
                    let s = self.end();
                    self.push_glyph(hyphen, 0, 0, group);
                    Some(Bx { s, e: self.end(), w: sc(metric.w as i32, scale), h: sc(metric.h as i32, scale), d: 0 })
                } else {
                    None
                };
                self.words.push(Word {
                    bx: Bx { s: start, e: end, w: width, h: b.h, d: b.d },
                    forced: word.forced && to == text.len(),
                    gap: if from == 0 { word.gap } else { false },
                    hyphen: mark,
                });
                origin += width;
                from = to;
            }
            changed = true;
        }
        changed
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn english_patterns() {
        assert_eq!(points(b"hyphenation"), [2, 6]);
        assert!(points(b"extraordinary").len() >= 2);
        assert!(points(b"the").is_empty());
    }
}
