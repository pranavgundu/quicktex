use super::*;
use alloc::collections::BTreeMap;
#[derive(Default)]
pub(super) struct Fonts {
    map: BTreeMap<u32, u16>,
    glyphs: Vec<Glyph>,
    paths: Vec<Vec<u8>>,
    pub(super) unicode: Vec<u32>,
}
impl Engine {
    pub fn register_glyph(&mut self, len: usize) -> i32 {
        if len < 16 || self.glyph_count() >= u16::MAX as usize {
            return -1;
        }
        let data = unsafe { core::slice::from_raw_parts(self.input.as_ptr(), len) };
        let int = |i: usize| i32::from_le_bytes(data[i..i + 4].try_into().unwrap());
        let cp = int(0) as u32;
        if self.fonts.map.contains_key(&cp) {
            return 0;
        }
        let id = self.glyph_count() as u16;
        self.fonts.glyphs.push(Glyph {
            off: 0,
            len: (len - 16).min(u16::MAX as usize) as u16,
            sx: 0,
            sy: 0,
            w: int(4) as i16,
            h: int(8) as i16,
            d: int(12) as i16,
            it: 0,
            sk: 0,
        });
        self.fonts.paths.push(data[16..].to_vec());
        self.fonts.unicode.push(cp);
        self.fonts.map.insert(cp, id);
        0
    }
    pub(super) fn fallback_glyph(&self, cp: u32) -> Option<u16> {
        self.fonts.map.get(&cp).copied()
    }
    pub(crate) fn glyph_count(&self) -> usize {
        GLYPHS.len() + self.fonts.glyphs.len()
    }
    pub(crate) fn glyph_metric(&self, id: usize) -> Glyph {
        if id < GLYPHS.len() { GLYPHS[id] } else { self.fonts.glyphs[id - GLYPHS.len()] }
    }
    pub(crate) fn glyph_path(&self, id: usize) -> &[u8] {
        if id < GLYPHS.len() {
            let g = GLYPHS[id];
            let offset = g.off as usize;
            let mut block = crate::glyph_data::PATH_INDEX[offset / 1024] as usize;
            let chunks = crate::glyph_data::PATH_CHUNKS;
            if block + 1 < chunks.len() && offset >= chunks[block + 1].0 {
                block += 1;
            }
            let (start, packed, len) = chunks[block];
            let path = self.paths[block]
                .get_or_init(|| miniz_oxide::inflate::decompress_to_vec_zlib(&PATHS_Z[packed..packed + len]).expect("bundled outlines"));
            &path[offset - start..offset - start + g.len as usize]
        } else {
            &self.fonts.paths[id - GLYPHS.len()]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compressed_blocks_preserve_every_builtin_outline() {
        let engine = Engine::new();
        for (id, glyph) in GLYPHS.iter().enumerate() {
            assert_eq!(engine.glyph_path(id), &PATHS[glyph.off as usize..glyph.off as usize + glyph.len as usize], "glyph {id}");
        }
    }
}
