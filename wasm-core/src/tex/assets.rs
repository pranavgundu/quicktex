use super::*;
use alloc::{borrow::Cow, format, sync::Arc};

#[derive(Default)]
pub(super) struct Assets {
    pub(super) bibliography: super::bibliography::Bibliography,
    pub(super) files: Vec<(Vec<u8>, Vec<u8>)>,
    pub(super) active: Vec<Vec<u8>>,
    image_cache: Vec<(usize, Option<Image>, usize)>,
    image_cache_bytes: usize,
}

const IMAGE_CACHE_BYTES: usize = 16 * 1024 * 1024;
const IMAGE_CACHE_ENTRIES: usize = 32;

impl Assets {
    fn clear_image_cache(&mut self) {
        self.image_cache.clear();
        self.image_cache_bytes = 0;
    }

    pub(super) fn image(&mut self, index: usize) -> Option<Image> {
        if let Some(at) = self.image_cache.iter().position(|entry| entry.0 == index) {
            let entry = self.image_cache.remove(at);
            let image = entry.1.clone();
            self.image_cache.push(entry);
            return image;
        }
        let image = Image::parse(&self.files.get(index)?.1);
        let bytes = image.as_ref().map_or(0, |image| image.url.len() + image.data.len() + image.alpha.len());
        if bytes <= IMAGE_CACHE_BYTES {
            while !self.image_cache.is_empty()
                && (self.image_cache.len() >= IMAGE_CACHE_ENTRIES || self.image_cache_bytes + bytes > IMAGE_CACHE_BYTES)
            {
                let (_, _, removed) = self.image_cache.remove(0);
                self.image_cache_bytes -= removed;
            }
            self.image_cache.push((index, image.clone(), bytes));
            self.image_cache_bytes += bytes;
        }
        image
    }
}

#[derive(Clone)]
pub(crate) struct Image {
    pub(crate) url: Arc<Vec<u8>>,
    pub(crate) data: Arc<Vec<u8>>,
    pub(crate) alpha: Arc<Vec<u8>>,
    pub(crate) pixels: (u32, u32),
    pub(crate) size: (i32, i32),
    pub(crate) jpeg: bool,
}

pub(crate) fn base64(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len().div_ceil(3) * 4);
    let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    for c in bytes.chunks(3) {
        let n = ((c[0] as u32) << 16) | ((c.get(1).copied().unwrap_or(0) as u32) << 8) | c.get(2).copied().unwrap_or(0) as u32;
        out.extend_from_slice(&[
            chars[(n >> 18) as usize],
            chars[((n >> 12) & 63) as usize],
            if c.len() > 1 { chars[((n >> 6) & 63) as usize] } else { b'=' },
            if c.len() > 2 { chars[(n & 63) as usize] } else { b'=' },
        ]);
    }
    out
}
fn u32be(b: &[u8]) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(..4)?.try_into().ok()?))
}
fn png(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>, Vec<u8>)> {
    let mut at = 8;
    let (mut w, mut h, mut channels) = (0, 0, 0);
    let mut packed = Vec::new();
    let mut palette = Vec::new();
    let mut transparency = Vec::new();
    let mut mode = 0;
    while at + 12 <= bytes.len() {
        let n = u32be(&bytes[at..])? as usize;
        let end = at.checked_add(8)?.checked_add(n)?;
        let chunk = bytes.get(at + 8..end)?;
        match bytes.get(at + 4..at + 8)? {
            b"IHDR" => {
                if chunk.len() != 13 || chunk[8] != 8 || chunk[10] != 0 || chunk[11] != 0 || chunk[12] != 0 {
                    return None;
                }
                w = u32be(chunk)?;
                h = u32be(&chunk[4..])?;
                mode = chunk[9];
                channels = match mode {
                    0 | 3 => 1,
                    2 => 3,
                    4 => 2,
                    6 => 4,
                    _ => return None,
                };
            }
            b"IDAT" => packed.extend_from_slice(chunk),
            b"PLTE" => palette = chunk.to_vec(),
            b"tRNS" => transparency = chunk.to_vec(),
            b"IEND" => break,
            _ => {}
        }
        at = end.checked_add(4)?;
    }
    if w == 0 || h == 0 || w as u64 * h as u64 > 32_000_000 || channels == 0 {
        return None;
    }
    let stride = w as usize * channels;
    let raw = miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(&packed, (stride + 1) * h as usize).ok()?;
    if raw.len() != (stride + 1) * h as usize {
        return None;
    }
    let mut pixels = alloc::vec![0u8;stride*h as usize];
    for y in 0..h as usize {
        let filter = raw[y * (stride + 1)];
        if filter > 4 {
            return None;
        }
        for x in 0..stride {
            let i = y * stride + x;
            let a = if x >= channels { pixels[i - channels] } else { 0 };
            let b = if y > 0 { pixels[i - stride] } else { 0 };
            let c = if y > 0 && x >= channels { pixels[i - stride - channels] } else { 0 };
            let pred = match filter {
                1 => a,
                2 => b,
                3 => ((a as u16 + b as u16) / 2) as u8,
                4 => {
                    let p = a as i32 + b as i32 - c as i32;
                    let (pa, pb, pc) = ((p - a as i32).abs(), (p - b as i32).abs(), (p - c as i32).abs());
                    if pa <= pb && pa <= pc {
                        a
                    } else if pb <= pc {
                        b
                    } else {
                        c
                    }
                }
                _ => 0,
            };
            pixels[i] = raw[y * (stride + 1) + x + 1].wrapping_add(pred);
        }
    }
    let mut rgb = Vec::with_capacity(w as usize * h as usize * 3);
    let mut alpha = Vec::new();
    let has_alpha = matches!(mode, 4 | 6) || !transparency.is_empty();
    for p in pixels.chunks(channels) {
        let (r, g, b, a) = match mode {
            0 => (p[0], p[0], p[0], if transparency.as_slice() == [0, p[0]] { 0 } else { 255 }),
            2 => (p[0], p[1], p[2], if transparency.as_slice() == [0, p[0], 0, p[1], 0, p[2]] { 0 } else { 255 }),
            3 => {
                let i = p[0] as usize;
                (*palette.get(3 * i)?, *palette.get(3 * i + 1)?, *palette.get(3 * i + 2)?, transparency.get(i).copied().unwrap_or(255))
            }
            4 => (p[0], p[0], p[0], p[1]),
            6 => (p[0], p[1], p[2], p[3]),
            _ => return None,
        };
        rgb.extend_from_slice(&[r, g, b]);
        if has_alpha {
            alpha.push(a);
        }
    }
    Some((w, h, rgb, alpha))
}
fn jpeg_size(bytes: &[u8]) -> Option<(u32, u32)> {
    let mut at = 2;
    while at + 4 < bytes.len() {
        if bytes[at] != 255 {
            return None;
        }
        at += 1;
        while bytes.get(at) == Some(&255) {
            at += 1;
        }
        let marker = *bytes.get(at)?;
        at += 1;
        if matches!(marker, 0xd8 | 0xd9 | 0x01 | 0xd0..=0xd7) {
            continue;
        }
        let n = u16::from_be_bytes(bytes.get(at..at + 2)?.try_into().ok()?) as usize;
        if n < 2 {
            return None;
        }
        if matches!(marker, 0xc0..=0xc3) {
            let c = bytes.get(at..at + n)?;
            if c.len() < 8 || c[2] != 8 || c[7] != 3 {
                return None;
            }
            return Some((u16::from_be_bytes([c[5], c[6]]) as u32, u16::from_be_bytes([c[3], c[4]]) as u32));
        }
        at = at.checked_add(n)?;
    }
    None
}
impl Image {
    pub(super) fn parse(bytes: &[u8]) -> Option<Self> {
        let (w, h, data, alpha, jpeg, mime) = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            let (w, h, data, alpha) = png(bytes)?;
            (w, h, data, alpha, false, "png")
        } else {
            if !bytes.starts_with(&[0xff, 0xd8]) {
                return None;
            }
            let (w, h) = jpeg_size(bytes)?;
            if w == 0 || h == 0 {
                return None;
            }
            (w, h, bytes.to_vec(), Vec::new(), true, "jpeg")
        };
        let mut url = format!("data:image/{mime};base64,").into_bytes();
        url.extend(base64(bytes));
        Some(Self { url: Arc::new(url), data: Arc::new(data), alpha: Arc::new(alpha), pixels: (w, h), size: (0, 0), jpeg })
    }
}
/// Lexical virtual paths follow filesystem dot-segment rules. Leading parent
/// components in a relative path stay relative to the document directory.
#[inline(never)]
pub(super) fn file_name(path: &[u8]) -> Cow<'_, [u8]> {
    if !path.split(|&b| b == b'/').any(|part| part.is_empty() || part == b"." || part == b"..") {
        return Cow::Borrowed(path);
    }
    let absolute = path.starts_with(b"/");
    let mut parts: Vec<&[u8]> = Vec::new();
    for part in path.split(|&b| b == b'/') {
        match part {
            b"" | b"." => {}
            b".." if parts.last().is_some_and(|last| *last != b"..") => {
                parts.pop();
            }
            b".." if absolute => {}
            _ => parts.push(part),
        }
    }
    let mut out = Vec::with_capacity(path.len());
    if absolute {
        out.push(b'/');
    }
    for part in parts {
        if !out.is_empty() && out.last() != Some(&b'/') {
            out.push(b'/');
        }
        out.extend_from_slice(part);
    }
    Cow::Owned(out)
}

pub(super) fn find_file(files: &[(Vec<u8>, Vec<u8>)], name: &[u8], extensions: &[&[u8]]) -> Option<usize> {
    let name = file_name(name);
    files.iter().position(|(n, _)| file_name(n).as_ref() == name.as_ref()).or_else(|| {
        extensions.iter().find_map(|ext| {
            files.iter().position(|(n, _)| {
                let n = file_name(n);
                n.starts_with(&name) && &n[name.len()..] == *ext
            })
        })
    })
}

impl Engine {
    pub fn clear_files(&mut self) {
        self.assets.clear_image_cache();
        self.assets.files.clear();
    }
    pub fn register_file(&mut self, name: usize, len: usize) -> i32 {
        if name > len {
            return -1;
        }
        let bytes = unsafe { core::slice::from_raw_parts(self.input.as_ptr(), len) };
        self.assets.clear_image_cache();
        self.assets.files.push((bytes[..name].to_vec(), bytes[name..].to_vec()));
        0
    }
    pub(super) fn file_index(&self, name: &[u8], extensions: &[&[u8]]) -> Option<usize> {
        find_file(&self.assets.files, name, extensions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoded_images_are_shared_but_requested_sizes_remain_local() {
        let mut assets = Assets::default();
        assets.files.push((b"image.png".to_vec(), include_bytes!("../../../tests/fixtures/image.png").to_vec()));
        let mut first = assets.image(0).unwrap();
        first.size = (1200, 800);
        let second = assets.image(0).unwrap();
        assert!(Arc::ptr_eq(&first.data, &second.data));
        assert!(Arc::ptr_eq(&first.url, &second.url));
        assert_eq!(second.size, (0, 0));
    }

    #[test]
    fn image_cache_is_bounded_and_project_replacement_invalidates_it() {
        let mut engine = Engine::new();
        for index in 0..40 {
            engine.assets.files.push((format!("invalid{index}.png").into_bytes(), b"invalid".to_vec()));
            assert!(engine.assets.image(index).is_none());
        }
        assert_eq!(engine.assets.image_cache.len(), IMAGE_CACHE_ENTRIES);
        engine.clear_files();
        assert!(engine.assets.image_cache.is_empty());
        engine.assets.files.push((b"image.png".to_vec(), include_bytes!("../../../tests/fixtures/image.png").to_vec()));
        assert!(engine.assets.image(0).is_some());
        // A full cache must evict before admitting another decoded image.
        engine.assets.image_cache.clear();
        let large = Image {
            url: Arc::new(alloc::vec![0; IMAGE_CACHE_BYTES]),
            data: Arc::new(Vec::new()),
            alpha: Arc::new(Vec::new()),
            pixels: (1, 1),
            size: (0, 0),
            jpeg: false,
        };
        engine.assets.image_cache.push((1, Some(large), IMAGE_CACHE_BYTES));
        engine.assets.image_cache_bytes = IMAGE_CACHE_BYTES;
        assert!(engine.assets.image(0).is_some());
        assert_eq!(engine.assets.image_cache.len(), 1);
        assert!(engine.assets.image_cache_bytes < IMAGE_CACHE_BYTES);
        engine.clear_files();
        engine.assets.files.push((b"image.png".to_vec(), b"invalid replacement".to_vec()));
        assert!(engine.assets.image(0).is_none());
    }
    #[test]
    fn truncated_images_never_panic() {
        let bytes = include_bytes!("../../../tests/fixtures/image.png");
        for end in 0..bytes.len() {
            let _ = Image::parse(&bytes[..end]);
        }
        let mut huge = bytes.to_vec();
        huge[8..12].fill(255);
        assert!(Image::parse(&huge).is_none());
    }
}
