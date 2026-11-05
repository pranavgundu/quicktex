#![no_std]
extern crate alloc;
use alloc::{format, vec::Vec};
use core::mem::MaybeUninit;
#[global_allocator]
static ALLOC: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}
struct Fonts {
    input: Vec<u8>,
    data: Vec<Vec<u8>>,
    out: Vec<u8>,
}
static mut FONTS: MaybeUninit<Fonts> = MaybeUninit::uninit();
static mut READY: bool = false;
fn fonts() -> &'static mut Fonts {
    unsafe {
        if !READY {
            (&raw mut FONTS).write(MaybeUninit::new(Fonts {
                input: Vec::new(),
                data: Vec::new(),
                out: Vec::new(),
            }));
            READY = true;
        }
        (*(&raw mut FONTS)).assume_init_mut()
    }
}
#[unsafe(no_mangle)]
extern "C" fn font_input(len: usize) -> *mut u8 {
    let f = fonts();
    f.input.resize(len, 0);
    f.input.as_mut_ptr()
}
#[unsafe(no_mangle)]
extern "C" fn font_load(len: usize) -> i32 {
    let f = fonts();
    let bytes = &f.input[..len];
    if ttf_parser::Face::parse(bytes, 0).is_err() {
        return -1;
    }
    if !f.data.iter().any(|v| v == bytes) {
        f.data.push(bytes.to_vec());
    }
    0
}
#[unsafe(no_mangle)]
extern "C" fn font_output() -> *const u8 {
    fonts().out.as_ptr()
}
#[unsafe(no_mangle)]
extern "C" fn font_glyph(cp: u32) -> usize {
    let Some(ch) = char::from_u32(cp) else {
        return 0;
    };
    let f = fonts();
    for data in &f.data {
        let Ok(face) = ttf_parser::Face::parse(data, 0) else {
            continue;
        };
        let Some(id) = face.glyph_index(ch) else {
            continue;
        };
        let scale = 1000.0 / face.units_per_em() as f32;
        let mut outline = Outline {
            path: Vec::new(),
            scale,
            x: 0,
            y: 0,
            start: (0, 0),
        };
        let bounds = face.outline_glyph(id, &mut outline);
        let w = (face.glyph_hor_advance(id).unwrap_or(1000) as f32 * scale) as i32;
        let (h, d) = bounds.map_or((0, 0), |b| {
            (
                (b.y_max as f32 * scale) as i32,
                (-b.y_min as f32 * scale) as i32,
            )
        });
        f.out.clear();
        for value in [cp as i32, w, h, d] {
            f.out.extend_from_slice(&value.to_le_bytes());
        }
        f.out.extend_from_slice(&outline.path);
        return f.out.len();
    }
    0
}
struct Outline {
    path: Vec<u8>,
    scale: f32,
    x: i32,
    y: i32,
    start: (i32, i32),
}
impl Outline {
    fn point(&self, x: f32, y: f32) -> (i32, i32) {
        ((x * self.scale) as i32, (-y * self.scale) as i32)
    }
}
impl ttf_parser::OutlineBuilder for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.point(x, y);
        self.path
            .extend_from_slice(format!("m{} {}", x - self.x, y - self.y).as_bytes());
        self.x = x;
        self.y = y;
        self.start = (x, y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.point(x, y);
        self.path
            .extend_from_slice(format!("l{} {}", x - self.x, y - self.y).as_bytes());
        self.x = x;
        self.y = y;
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let (cx, cy) = self.point(cx, cy);
        let (x, y) = self.point(x, y);
        self.path.extend_from_slice(
            format!(
                "q{} {} {} {}",
                cx - self.x,
                cy - self.y,
                x - self.x,
                y - self.y
            )
            .as_bytes(),
        );
        self.x = x;
        self.y = y;
    }
    fn curve_to(&mut self, a: f32, b: f32, c: f32, d: f32, x: f32, y: f32) {
        let (a, b) = self.point(a, b);
        let (c, d) = self.point(c, d);
        let (x, y) = self.point(x, y);
        self.path.extend_from_slice(
            format!(
                "c{} {} {} {} {} {}",
                a - self.x,
                b - self.y,
                c - self.x,
                d - self.y,
                x - self.x,
                y - self.y
            )
            .as_bytes(),
        );
        self.x = x;
        self.y = y;
    }
    fn close(&mut self) {
        self.path.push(b'z');
        (self.x, self.y) = self.start;
    }
}
