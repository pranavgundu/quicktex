use crate::data::*;
use alloc::vec::Vec;

pub(crate) mod assets;
mod bibliography;
mod document;
mod fast_zip;
mod fonts;
mod macros;
mod metrics;
mod pdf;
use document::{Laid, Word};
use macros::Def;

type R<T> = Result<T, ()>;

const ORD: u8 = 0;
const OP: u8 = 1;
const BIN: u8 = 2;
const REL: u8 = 3;
const OPEN: u8 = 4;
const CLOSE: u8 = 5;
const PUNCT: u8 = 6;
const INNER: u8 = 7;
const NONE: u8 = 8;

const ST_D: u8 = 0;
const ST_T: u8 = 2;
const ST_S: u8 = 4;
const ST_SS: u8 = 6;

const SUP: [u8; 8] = [4, 5, 4, 5, 6, 7, 6, 7];
const SUB: [u8; 8] = [5, 5, 5, 5, 7, 7, 7, 7];
const NUM: [u8; 8] = [2, 3, 4, 5, 6, 7, 6, 7];
const DEN: [u8; 8] = [3, 3, 5, 5, 7, 7, 7, 7];
const RATIO: [i32; 4] = [1000, 1000, 700, 500];
const SIDX: [usize; 4] = [0, 0, 1, 2];

const XHEIGHT: i32 = 431;
const AXIS: i32 = 250;
const QUAD: [i32; 3] = [1000, 1171, 1472];
const NUM1: [i32; 3] = [677, 732, 925];
const NUM2: [i32; 3] = [394, 384, 387];
const NUM3: [i32; 3] = [444, 471, 504];
const DENOM1: [i32; 3] = [686, 752, 1025];
const DENOM2: [i32; 3] = [345, 344, 532];
const SUP1: [i32; 3] = [413, 503, 504];
const SUP2: [i32; 3] = [363, 431, 404];
const SUP3: [i32; 3] = [289, 286, 294];
const SUB1: [i32; 3] = [150, 143, 200];
const SUB2: [i32; 3] = [247, 286, 400];
const SUPDROP: [i32; 3] = [386, 353, 494];
const SUBDROP: [i32; 3] = [50, 71, 100];
const DELIM1: [i32; 3] = [2390, 1700, 1980];
const DELIM2: [i32; 3] = [1010, 1157, 1420];
const RULE: [i32; 3] = [40, 49, 49];
const BIGOP1: [i32; 3] = [111, 111, 111];
const BIGOP2: [i32; 3] = [166, 166, 166];
const BIGOP3: [i32; 3] = [200, 200, 200];
const BIGOP4: [i32; 3] = [600, 611, 611];
const BIGOP5: [i32; 3] = [100, 143, 143];

const SPACE: [[u8; 8]; 8] = [
    [0, 3, 4, 5, 0, 0, 0, 3],
    [3, 3, 0, 5, 0, 0, 0, 3],
    [4, 4, 0, 0, 4, 0, 0, 4],
    [5, 5, 0, 0, 5, 0, 0, 5],
    [0, 0, 0, 0, 0, 0, 0, 0],
    [0, 3, 4, 5, 0, 0, 0, 3],
    [3, 3, 0, 5, 3, 3, 3, 3],
    [3, 3, 4, 5, 3, 0, 3, 3],
];
const TIGHT: [[u8; 8]; 8] = [
    [0, 3, 0, 0, 0, 0, 0, 0],
    [3, 3, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0],
    [0, 3, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 0, 0],
    [0, 3, 0, 0, 0, 0, 0, 0],
];
const BIN_TO_ORD: [bool; 9] = [false, true, true, true, true, false, true, false, true];

const T_BAD: u8 = 0;
const T_SYM: u8 = 1;
const T_WS: u8 = 2;
const T_LB: u8 = 3;
const T_RB: u8 = 4;
const T_CMD: u8 = 5;
const T_SCRIPT: u8 = 6;
const T_AMP: u8 = 7;
const T_NUL: u8 = 8;
const T_DOLLAR: u8 = 9;
const T_TILDE: u8 = 10;
const T_PCT: u8 = 11;
const T_UTF8: u8 = 12;

const fn char_types() -> [u8; 256] {
    let mut t = [T_BAD; 256];
    let mut i = 33;
    while i < 127 {
        t[i] = T_SYM;
        i += 1;
    }
    let mut i = 128;
    while i < 256 {
        t[i] = T_UTF8;
        i += 1;
    }
    t[0] = T_NUL;
    t[b' ' as usize] = T_WS;
    t[b'\t' as usize] = T_WS;
    t[b'\n' as usize] = T_WS;
    t[b'\r' as usize] = T_WS;
    t[b'{' as usize] = T_LB;
    t[b'}' as usize] = T_RB;
    t[b'\\' as usize] = T_CMD;
    t[b'^' as usize] = T_SCRIPT;
    t[b'_' as usize] = T_SCRIPT;
    t[b'\'' as usize] = T_SCRIPT;
    t[b'&' as usize] = T_AMP;
    t[b'$' as usize] = T_DOLLAR;
    t[b'~' as usize] = T_TILDE;
    t[b'%' as usize] = T_PCT;
    t[b'#' as usize] = T_BAD;
    t
}
static CT: [u8; 256] = char_types();

const S_EOF: u8 = 0;
const S_BRACE: u8 = 1;
const S_RIGHT: u8 = 2;
const S_CELL: u8 = 3;
const S_DOLLAR: u8 = 4;
const S_MACRO: u8 = 5;
const S_BRACKET: u8 = 6;
const S_SUBCELL: u8 = 7;

const TOK_AMP: u8 = 1;
const TOK_CR: u8 = 2;
const TOK_END: u8 = 3;

const MAX_DEPTH: u32 = 96;
const MAX_REPARSE: u32 = 256;
const MAX_DIM: i64 = 1_000_000;
const DEF_SLOTS: usize = 1024;

const THIN: i32 = 40;
const NULL_DELIM: i32 = 120;
const OVERLAP: i32 = 8;

const FRAC_STYLE: u8 = 3;
const FRAC_NO_BAR: u8 = 8;
const FRAC_PAREN: u8 = 16;

#[cfg(test)]
mod tests;

const NOT_EQUAL: u32 = 0x2260;
const E_END: u8 = 1;
const E_BRACE: u8 = 2;
const E_CMD: u8 = 3;
const E_CHAR: u8 = 4;
const E_DEPTH: u8 = 5;
const E_SCRIPT: u8 = 6;
const E_DELIM: u8 = 7;
const E_ENV: u8 = 8;
const E_ARG: u8 = 9;
const E_COLOR: u8 = 10;
const E_UNIT: u8 = 11;
const E_MISPLACED: u8 = 12;
const E_PACKAGE: u8 = 13;
const E_OPTION: u8 = 14;
const E_COUNTER: u8 = 15;
const E_LOOP: u8 = 16;

pub const IT_GLYPH: u8 = 0;
pub const IT_RULE: u8 = 1;
pub const IT_BRACE: u8 = 2;
pub const IT_SLASH: u8 = 3;
pub const IT_BACKSLASH: u8 = 4;
pub const IT_UNDERBRACE: u8 = 5;
pub const IT_IMAGE: u8 = 6;
pub const IT_ELLIPSE: u8 = 7;

pub const UNIT: u16 = 1000;

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Item {
    pub x: i32,
    pub y: i32,
    pub a: i32,
    pub b: i32,
    pub g: u16,
    pub kind: u8,
}

#[derive(Clone, Copy, Default)]
pub struct Bx {
    pub s: u32,
    pub e: u32,
    pub w: i32,
    pub h: i32,
    pub d: i32,
}

#[derive(Clone, Copy)]
pub struct Grp {
    pub scale: u16,
    pub color: u16,
    pub link: u32,
}

#[derive(Clone, Copy)]
struct Ctx {
    style: u8,
    font: u8,
    size: u16,
    scale: u16,
    color: u16,
    grp: u16,
}

#[derive(Clone, Copy)]
struct Atom {
    bx: Bx,
    class: u8,
    italic: i32,
    charbox: bool,
    limits: bool,
}

struct Run {
    x: i32,
    h: i32,
    d: i32,
    prev: u8,
    pp: u8,
    gap: i32,
    prev_s: u32,
    count: u32,
    mbase: usize,
    track: bool,
    last: Atom,
    at_x: i32,
    end_x: i32,
    x0: i32,
    prev0: u8,
    pp0: u8,
    gap0: i32,
    prev_s0: u32,
    start: usize,
    first: u32,
    ctx: Ctx,
}

struct Src {
    p: *const u8,
    pos: usize,
    len: usize,
    user: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Grid {
    Matrix,
    Align,
    Cases,
    Array,
}

#[derive(Clone, Copy)]
struct Mid {
    cp: u32,
    idx: u32,
    x: i32,
}

#[derive(Clone, Copy)]
struct Cell {
    bx: Bx,
    row: u32,
    col: u32,
}

#[derive(Clone, Copy, Default)]
struct Row {
    h: i32,
    d: i32,
    gap: i32,
    y: i32,
    lines: u8,
}

#[derive(Clone, Copy, PartialEq)]
enum Seq {
    Never,
    Large,
    Always,
}

#[derive(Clone, Copy)]
struct Stack {
    seq: Seq,
    font: u8,
    top: u32,
    mid: u32,
    bot: u32,
    bars: [(i32, i32); 2],
    nbars: u8,
}

// Direct-mapped, bounded ASCII metric cache. The full font/scale/point-size
// key is checked on every hit; document edits and colour changes can reuse it.
#[derive(Clone, Copy, Default)]
struct TextMetric {
    key: u64,
    glyph: u16,
    natural: i32,
    width: i32,
    h: i32,
    d: i32,
}
const TEXT_METRIC_SLOTS: usize = 512;

pub struct Engine {
    pair: Pair,
    text_metrics: Vec<TextMetric>,
    p: *const u8,
    pos: usize,
    len: usize,
    pub(crate) items: Vec<Item>,
    pub(crate) groups: Vec<Grp>,
    pub(crate) colors: Vec<(u32, u32)>,
    pub(crate) color_bytes: Vec<u8>,
    counters: Vec<macros::Counter>,
    loop_iterations: u32,
    has_count_registers: bool,
    at_letter: bool,
    active_link: u32,
    color_defs: Vec<(&'static [u8], u16)>,
    defs: Vec<Def>,
    def_hash: [u16; DEF_SLOTS],
    pkgs: u64,
    session: u64,
    expansions: u32,
    exp_bytes: usize,
    synth: Vec<Vec<u8>>,
    call_pos: usize,
    cells: Vec<Cell>,
    rows: Vec<Row>,
    cols: Vec<i32>,
    colx: Vec<i32>,
    mids: Vec<Mid>,
    pub(crate) out: Vec<u8>,
    doc: Laid,
    pub(crate) laid: Laid,
    scratch: pdf::Scratch,
    store: document::Store,
    pub(crate) art: Vec<u8>,
    words: Vec<Word>,
    line_costs: Vec<(i64, u32)>,
    line_ends: Vec<u32>,
    pub(crate) order: Vec<u32>,
    pub(crate) counts: Vec<u32>,
    input: Vec<u8>,
    fonts: fonts::Fonts,
    paths: Vec<core::cell::OnceCell<Vec<u8>>>,
    assets: assets::Assets,
    depth: u32,
    reparses: u32,
    over_stop: usize,
    stop_tok: u8,
    one_class: u8,
    err: u8,
    err_pos: usize,
    user_src: bool,
    lenient: bool,
    warned: Vec<u32>,
    pool: Vec<Vec<u8>>,
    pool_at: usize,
    flags: Vec<(&'static [u8], bool)>,
    labels: Vec<(u32, u32, u32)>,
    label_bytes: Vec<u8>,
    mark: (u8, [u32; 3]),
    cites: Vec<Vec<u8>>,
    unresolved: bool,
    spliced: usize,
}

#[inline(always)]
fn sc(v: i32, s: u16) -> i32 {
    if s == UNIT { v } else { (v * s as i32 + 500) / 1000 }
}

#[inline(always)]
fn sidx(c: &Ctx) -> usize {
    SIDX[(c.style >> 1) as usize]
}

#[inline(always)]
fn sg(c: &Ctx, t: &[i32; 3]) -> i32 {
    sc(t[sidx(c)], c.scale)
}

fn macro_text(s: &Sym) -> *const u8 {
    unsafe { STRS.as_ptr().add(s.cp as usize) }
}

fn env_base(name: &'static [u8]) -> &'static [u8] {
    name.strip_suffix(b"*").unwrap_or(name)
}

fn column_align(grid: Grid, spec: &[u8], col: usize) -> u8 {
    match grid {
        Grid::Align if col % 2 == 0 => b'r',
        Grid::Align | Grid::Cases => b'l',
        Grid::Array => spec.iter().copied().filter(|b| matches!(b, b'l' | b'c' | b'r')).nth(col).unwrap_or(b'c'),
        Grid::Matrix => b'c',
    }
}

#[inline(always)]
fn hash_name(n: &[u8]) -> usize {
    let mut h = 0x811c9dc5u32;
    for &b in n {
        h = (h ^ b as u32).wrapping_mul(16777619);
    }
    ((h ^ (h >> 15)) & ((1 << HASH_BITS) - 1)) as usize
}

#[inline]
fn lookup(n: &[u8]) -> Option<&'static Sym> {
    let mask = (1usize << HASH_BITS) - 1;
    let mut i = hash_name(n);
    loop {
        let e = HASH[i];
        if e == 0 {
            return None;
        }
        let en = &ENTRIES[e as usize - 1];
        if en.len as usize == n.len() {
            let o = en.off as usize;
            if &NAMES[o..o + n.len()] == n {
                return Some(&en.sym);
            }
        }
        i = (i + 1) & mask;
    }
}

/// The last character set, for kerning the next one against it.
#[derive(Clone, Copy, Default)]
struct Pair {
    font: u8,
    scale: u16,
    code: u8,
    end: usize,
}

/// The position of a character in the OT1 encoding the kern tables use.
fn kerns_code(cp: u32) -> u8 {
    match cp {
        0x2019 => 0x27,
        0x2018 => 0x60,
        0x201c => 0x5c,
        0x201d => 0x22,
        0x2013 => 0x7b,
        0x2014 => 0x7c,
        0x7b | 0x7c | 0x5c => 0,
        0x21..=0x7e => cp as u8,
        _ => 0,
    }
}

fn kern_pair(font: u8, left: u8, right: u8) -> Option<(i16, u8)> {
    let table = match font {
        FONT_MAIN_REGULAR => metrics::MAIN,
        FONT_MAIN_BOLD => metrics::BOLD,
        FONT_MAIN_ITALIC => metrics::ITALIC,
        FONT_SANSSERIF_REGULAR => metrics::SANS,
        _ => return None,
    };
    let at = table.binary_search_by_key(&(left, right), |p| (p.0, p.1)).ok()?;
    Some((table[at].2, table[at].3))
}

const FONT_MATH_ITALIC: u8 = 4;
const DOTS_WIDTH: i32 = 1167;
const THIN_MU: i32 = 3;

/// Which family of font files sets a character, and its code there.
fn family_of(font: u8, cp: u32) -> Option<(&'static [metrics::Design], u8)> {
    let symbol = match cp {
        0x2212 => Some(0),
        0x22c5 => Some(1),
        0xd7 => Some(2),
        0x2217 => Some(3),
        0xb1 => Some(6),
        0x2264 => Some(0x14),
        0x2265 => Some(0x15),
        0x2190 => Some(0x20),
        0x2192 => Some(0x21),
        0x2032 => Some(0x30),
        0x221e => Some(0x31),
        0x2208 => Some(0x32),
        _ => None,
    };
    let code = kerns_code(cp);
    if matches!(cp, 0x3c | 0x3e) {
        return Some((metrics::MATH, cp as u8));
    }
    match font {
        FONT_MAIN_REGULAR => symbol.map_or(Some((metrics::ROMAN, code)), |code| Some((metrics::SYMBOLS, code))),
        FONT_MAIN_BOLD => Some((metrics::STRONG, code)),
        FONT_MAIN_ITALIC => Some((metrics::SLANTED, code)),
        FONT_SANSSERIF_REGULAR => Some((metrics::PLAIN, code)),
        FONT_TYPEWRITER_REGULAR => Some((metrics::MONO, code)),
        FONT_MATH_ITALIC => match cp {
            0x3b1..=0x3c9 if cp != 0x3c2 => Some((metrics::MATH, (cp - 0x3b1) as u8 + 0x0b - (cp > 0x3c2) as u8)),
            _ => Some((metrics::MATH, code)),
        },
        _ => None,
    }
}

fn font_file(family: &'static [metrics::Design], size: i32) -> &'static metrics::Design {
    family.iter().rev().find(|file| file.from as i32 * 1000 <= size).unwrap_or(&family[0])
}

#[inline]
fn glyph_id(font: u8, cp: u32) -> Option<u16> {
    let k = font as u32 * 0x200000 + cp;
    let mask = (1usize << GM_BITS) - 1;
    let mut i = (k.wrapping_mul(0x9e3779b1) >> (32 - GM_BITS)) as usize;
    loop {
        let v = GM_KEYS[i];
        if v == 0 {
            return None;
        }
        if v == k + 1 {
            return Some(GM_VALS[i]);
        }
        i = (i + 1) & mask;
    }
}

fn plain_spec(spec: &[u8], out: &mut Vec<u8>, depth: u32) {
    let group = |at: usize| {
        let mut level = 0;
        for (i, ch) in spec.iter().enumerate().skip(at) {
            level += (*ch == b'{') as i32 - (*ch == b'}') as i32;
            if level == 0 {
                return i;
            }
        }
        spec.len().saturating_sub(1)
    };
    let mut i = 0;
    while i < spec.len() {
        let ch = spec[i];
        let braced = spec.get(i + 1) == Some(&b'{');
        match ch {
            b'@' | b'!' | b'>' | b'<' if braced => i = group(i + 1),
            b'p' | b'm' | b'b' if braced => {
                out.push(b'l');
                i = group(i + 1);
            }
            b'*' if braced && depth < 3 => {
                let end = group(i + 1);
                let count = spec[i + 2..end].iter().filter(|c| c.is_ascii_digit()).fold(0usize, |v, c| v * 10 + (*c - b'0') as usize);
                if spec.get(end + 1) == Some(&b'{') {
                    let close = group(end + 1);
                    for _ in 0..count.min(64) {
                        plain_spec(&spec[end + 2..close], out, depth + 1);
                    }
                    i = close;
                } else {
                    i = end;
                }
            }
            _ => out.push(ch),
        }
        i += 1;
    }
}

#[inline]
fn variant_font(fv: u8, s: &Sym) -> u8 {
    match fv {
        FV_RM => FONT_MAIN_REGULAR,
        FV_BF => FONT_MAIN_BOLD,
        FV_IT => FONT_MAIN_ITALIC,
        FV_SF => FONT_SANSSERIF_REGULAR,
        FV_TT => FONT_TYPEWRITER_REGULAR,
        FV_CAL => FONT_CALIGRAPHIC_REGULAR,
        FV_FRAK => FONT_FRAKTUR_REGULAR,
        FV_BB => FONT_AMS_REGULAR,
        FV_SCR => FONT_SCRIPT_REGULAR,
        _ => {
            if s.flags & F_MATHORD != 0 {
                FONT_MATH_BOLDITALIC
            } else {
                FONT_MAIN_BOLD
            }
        }
    }
}

fn stack_spec(cp: u32) -> Option<Stack> {
    const S1: u8 = FONT_SIZE1_REGULAR;
    const S4: u8 = FONT_SIZE4_REGULAR;
    let st = |seq, font, top, mid, bot, b: (i32, i32)| Stack { seq, font, top, mid, bot, bars: [b, (0, 0)], nbars: 1 };
    Some(match cp {
        0x28 => st(Seq::Large, S4, 0x239b, 0, 0x239d, (291, 417)),
        0x29 => st(Seq::Large, S4, 0x239e, 0, 0x23a0, (457, 583)),
        0x5b => st(Seq::Large, S4, 0x23a1, 0, 0x23a3, (319, 403)),
        0x5d => st(Seq::Large, S4, 0x23a4, 0, 0x23a6, (263, 347)),
        0x230a => st(Seq::Large, S4, 0, 0, 0x23a3, (319, 403)),
        0x2308 => st(Seq::Large, S4, 0x23a1, 0, 0, (319, 403)),
        0x230b => st(Seq::Large, S4, 0, 0, 0x23a6, (263, 347)),
        0x2309 => st(Seq::Large, S4, 0x23a4, 0, 0, (263, 347)),
        0x7b => st(Seq::Large, S4, 0x23a7, 0x23a8, 0x23a9, (384, 504)),
        0x7d => st(Seq::Large, S4, 0x23ab, 0x23ac, 0x23ad, (384, 504)),
        0x27ee => st(Seq::Always, S4, 0x23a7, 0, 0x23a9, (384, 504)),
        0x27ef => st(Seq::Always, S4, 0x23ab, 0, 0x23ad, (384, 504)),
        0x23b0 => st(Seq::Always, S4, 0x23a7, 0, 0x23ad, (384, 504)),
        0x23b1 => st(Seq::Always, S4, 0x23ab, 0, 0x23a9, (384, 504)),
        0x7c | 0x2223 => st(Seq::Always, S1, 0, 0, 0, (145, 188)),
        0x2225 | 0x2016 => Stack { seq: Seq::Always, font: S1, top: 0, mid: 0, bot: 0, bars: [(145, 188), (367, 410)], nbars: 2 },
        0x2191 => st(Seq::Always, S1, 0x2191, 0, 0, (312, 355)),
        0x2193 => st(Seq::Always, S1, 0, 0, 0x2193, (312, 355)),
        0x2195 => st(Seq::Always, S1, 0x2191, 0, 0x2193, (312, 355)),
        0x21d1 | 0x21d3 | 0x21d5 => Stack {
            seq: Seq::Always,
            font: S1,
            top: if cp == 0x21d3 { 0 } else { 0x21d1 },
            mid: 0,
            bot: if cp == 0x21d1 { 0 } else { 0x21d3 },
            bars: [(257, 300), (478, 521)],
            nbars: 2,
        },
        0x27e8 | 0x27e9 | 0x2f | 0x5c => Stack { seq: Seq::Never, font: S4, top: 0, mid: 0, bot: 0, bars: [(0, 0), (0, 0)], nbars: 0 },
        _ => return None,
    })
}

impl Engine {
    pub fn new() -> Self {
        Engine {
            pair: Pair::default(),
            text_metrics: Vec::new(),
            p: core::ptr::null(),
            pos: 0,
            len: 0,
            items: Vec::with_capacity(4096),
            groups: Vec::with_capacity(16),
            colors: Vec::with_capacity(16),
            color_bytes: Vec::with_capacity(256),
            counters: Vec::new(),
            loop_iterations: 0,
            has_count_registers: false,
            at_letter: false,
            active_link: 0,
            color_defs: Vec::new(),
            defs: Vec::new(),
            def_hash: [0; DEF_SLOTS],
            pkgs: 0,
            session: 0,
            expansions: 0,
            exp_bytes: 0,
            synth: Vec::new(),
            call_pos: 0,
            cells: Vec::with_capacity(256),
            rows: Vec::with_capacity(64),
            cols: Vec::with_capacity(32),
            colx: Vec::with_capacity(32),
            mids: Vec::with_capacity(8),
            out: Vec::with_capacity(1 << 16),
            doc: Laid::default(),
            laid: Laid::default(),
            scratch: pdf::Scratch::default(),
            store: document::Store::default(),
            art: Vec::new(),
            words: Vec::new(),
            line_costs: Vec::new(),
            line_ends: Vec::new(),
            order: Vec::with_capacity(4096),
            counts: Vec::with_capacity(16),
            input: Vec::with_capacity(4096),
            fonts: fonts::Fonts::default(),
            paths: (0..crate::glyph_data::PATH_CHUNKS.len()).map(|_| core::cell::OnceCell::new()).collect(),
            assets: assets::Assets::default(),
            depth: 0,
            reparses: 0,
            over_stop: usize::MAX,
            stop_tok: 0,
            one_class: ORD,
            err: 0,
            err_pos: 0,
            user_src: true,
            lenient: false,
            warned: Vec::new(),
            pool: Vec::new(),
            pool_at: 0,
            flags: Vec::new(),
            labels: Vec::new(),
            label_bytes: Vec::new(),
            mark: (0, [0; 3]),
            cites: Vec::new(),
            unresolved: false,
            spliced: 0,
        }
    }

    pub fn input_buffer(&mut self, len: usize) -> *mut u8 {
        if self.input.capacity() < len + 1 {
            self.input.reserve(len + 1 - self.input.len());
        }
        self.input.as_mut_ptr()
    }

    fn input_ptr(&self) -> *const u8 {
        self.input.as_ptr()
    }

    pub fn output_ptr(&self) -> *const u8 {
        self.out.as_ptr()
    }

    pub fn error_pos(&self) -> usize {
        self.err_pos
    }

    #[inline(always)]
    fn reset(&mut self, len: usize) {
        unsafe { *self.input.as_mut_ptr().add(len) = 0 };
        self.p = self.input.as_ptr();
        self.pos = 0;
        self.len = len;
        self.items.clear();
        self.groups.clear();
        self.active_link = 0;
        self.colors.clear();
        self.cells.clear();
        self.rows.clear();
        self.cols.clear();
        self.colx.clear();
        self.mids.clear();
        self.color_bytes.clear();
        self.colors.push((0, 0));
        self.groups.push(Grp { scale: UNIT, color: 0, link: 0 });
        self.depth = 0;
        self.reparses = 0;
        self.spliced = 0;
        self.over_stop = usize::MAX;
        self.stop_tok = 0;
        self.err = 0;
        self.err_pos = 0;
        self.user_src = true;
        self.lenient = false;
        self.warned.clear();
        self.pool_at = 0;
        self.flags.clear();
        if let Some(first) = self.pool.first_mut() {
            first.clear();
        }
        self.call_pos = 0;
        self.reset_macros();
    }

    pub fn render(&mut self, len: usize, display: bool) -> i32 {
        self.reset(len);
        let style = if display { ST_D } else { ST_T };
        let ctx = Ctx { style, font: 0, size: UNIT, scale: UNIT, color: 0, grp: 0 };
        match self.list(ctx, S_EOF, NONE) {
            Ok(b) => {
                self.emit(b);
                self.out.len() as i32
            }
            Err(()) => {
                self.write_error();
                -(self.out.len() as i32)
            }
        }
    }

    fn write_error(&mut self) {
        let m: &[u8] = match self.err {
            E_END => b"unexpected end of input",
            E_BRACE => b"unbalanced braces",
            E_CMD => b"unknown command",
            E_CHAR => b"unsupported character",
            E_DEPTH => b"nesting too deep",
            E_SCRIPT => b"double script",
            E_DELIM => b"invalid delimiter",
            E_ENV => b"unknown or mismatched environment",
            E_ARG => b"missing argument",
            E_COLOR => b"invalid color",
            E_UNIT => b"invalid dimension",
            E_MISPLACED => b"misplaced token",
            E_PACKAGE => b"unknown package",
            E_OPTION => b"unsupported drawing option",
            E_COUNTER => b"invalid or undefined counter",
            E_LOOP => b"invalid loop or iteration limit exceeded",
            _ => b"parse error",
        };
        self.out.clear();
        self.out.extend_from_slice(m);
    }

    #[cold]
    #[inline(never)]
    fn fail<T>(&mut self, code: u8) -> R<T> {
        if self.err == 0 {
            self.err = code;
            self.err_pos = if self.user_src { self.pos.min(self.len) } else { self.call_pos };
        }
        Err(())
    }

    #[inline(always)]
    fn peek(&self) -> u8 {
        unsafe { *self.p.add(self.pos) }
    }

    #[inline(always)]
    fn peek_at(&self, i: usize) -> u8 {
        if self.pos + i <= self.len { unsafe { *self.p.add(self.pos + i) } } else { 0 }
    }

    #[inline(always)]
    fn at_end(&self) -> bool {
        self.pos >= self.len
    }

    #[inline(always)]
    fn skip_ws(&mut self) {
        loop {
            let c = self.peek();
            if c == b' ' || c == b'\n' || c == b'\t' || c == b'\r' {
                self.pos += 1;
            } else if c == b'%' && !self.at_end() {
                while !self.at_end() && self.peek() != b'\n' {
                    self.pos += 1;
                }
            } else {
                return;
            }
        }
    }

    fn slice(&self, a: usize, b: usize) -> &'static [u8] {
        unsafe { core::slice::from_raw_parts(self.p.add(a), b - a) }
    }

    #[inline]
    fn read_name(&mut self) -> &'static [u8] {
        let s = self.pos;
        let c = self.peek();
        // Internal control-word tokens stored in macro definitions retain their
        // identity after makeatother, just as TeX's tokenized replacement text.
        if c == 0x1f {
            self.pos += 1;
            let start = self.pos;
            while !self.at_end() && self.peek() != 0x1f {
                self.pos += 1;
            }
            let name = self.slice(start, self.pos);
            if !self.at_end() { self.pos += 1; }
            return name;
        }
        if c.is_ascii_alphabetic() || (self.at_letter && c == b'@') {
            while self.peek().is_ascii_alphabetic() || (self.at_letter && self.peek() == b'@') {
                self.pos += 1;
            }
            if self.peek() == b'*' && self.slice(s, self.pos) == b"operatorname" {
                self.pos += 1;
            }
        } else if !self.at_end() {
            self.pos += 1;
        }
        self.slice(s, self.pos)
    }

    #[inline]
    fn utf8_len(c: u8) -> usize {
        if c >= 0xf0 {
            4
        } else if c >= 0xe0 {
            3
        } else {
            2
        }
    }

    fn read_utf8(&mut self) -> R<(&'static [u8], u32)> {
        let c = self.peek();
        let n = Self::utf8_len(c);
        if self.pos + n > self.len {
            return self.fail(E_CHAR);
        }
        let s = self.slice(self.pos, self.pos + n);
        let mut cp = (c as u32) & (0xff >> (n + 1));
        for &b in &s[1..] {
            cp = (cp << 6) | (b as u32 & 0x3f);
        }
        self.pos += n;
        Ok((s, cp))
    }

    fn grp(&mut self, color: u16, scale: u16) -> u16 {
        for (i, g) in self.groups.iter().enumerate() {
            if g.scale == scale && g.color == color && g.link == self.active_link {
                return i as u16;
            }
        }
        self.groups.push(Grp { scale, color, link: self.active_link });
        (self.groups.len() - 1) as u16
    }

    fn restyle(&mut self, c: &Ctx, style: u8) -> Ctx {
        let scale = sc(RATIO[(style >> 1) as usize], c.size) as u16;
        let grp = if scale == c.scale { c.grp } else { self.grp(c.color, scale) };
        Ctx { style, scale, grp, ..*c }
    }

    #[inline(always)]
    fn push_glyph(&mut self, g: u16, x: i32, y: i32, grp: u16) {
        let grp = if self.active_link != 0 && self.groups[grp as usize].link != self.active_link {
            let style = self.groups[grp as usize];
            self.grp(style.color, style.scale)
        } else {
            grp
        };
        if self.glyph_metric(g as usize).len != 0 {
            self.items.push(Item { x, y, a: g as i32, b: 0, g: grp, kind: IT_GLYPH });
        }
    }

    fn push_item(&mut self, c: &Ctx, x: i32, y: i32, a: i32, b: i32, kind: u8) {
        let g = self.grp(c.color, UNIT);
        self.items.push(Item { x, y, a, b, g, kind });
    }

    fn push_rule(&mut self, c: &Ctx, x: i32, y: i32, w: i32, h: i32) {
        self.push_item(c, x, y, w, h, IT_RULE);
    }

    #[inline(always)]
    fn end(&self) -> u32 {
        self.items.len() as u32
    }

    #[inline(always)]
    fn close(&self, s: u32, w: i32, h: i32, d: i32) -> Bx {
        let e = self.end();
        Bx { s, e, w, h, d }
    }

    #[inline(always)]
    fn atom(&self, s: u32, w: i32, h: i32, d: i32, class: u8) -> Atom {
        Self::plain(self.close(s, w, h, d), class)
    }

    #[inline(always)]
    fn push_src(&mut self, p: *const u8, len: usize) -> Src {
        let src = Src { p: self.p, pos: self.pos, len: self.len, user: self.user_src };
        if src.user {
            self.call_pos = src.pos;
        }
        self.p = p;
        self.pos = 0;
        self.len = len;
        self.user_src = false;
        src
    }

    #[inline(always)]
    fn pop_src(&mut self, src: Src) {
        self.p = src.p;
        self.pos = src.pos;
        self.len = src.len;
        self.user_src = src.user;
    }

    fn list_in(&mut self, p: *const u8, len: usize, c: Ctx) -> R<Bx> {
        let src = self.push_src(p, len);
        let b = self.list(c, S_MACRO, NONE)?;
        self.pop_src(src);
        Ok(b)
    }

    #[inline(always)]
    fn shift(&mut self, s: u32, e: u32, dx: i32, dy: i32) {
        if dx == 0 && dy == 0 {
            return;
        }
        for it in &mut self.items[s as usize..e as usize] {
            it.x += dx;
            it.y += dy;
        }
    }

    #[inline(always)]
    fn move_box(&mut self, b: &Bx, dx: i32, dy: i32) {
        self.shift(b.s, b.e, dx, dy);
    }

    fn mu(c: &Ctx, n: i32) -> i32 {
        let v = n * QUAD[sidx(c)] * c.scale as i32;
        if v >= 0 { (v + 9000) / 18000 } else { (v - 9000) / 18000 }
    }

    #[inline(always)]
    fn gap(c: &Ctx, a: u8, b: u8) -> i32 {
        if a >= NONE {
            return 0;
        }
        let t = if c.style >= ST_S { &TIGHT } else { &SPACE };
        let m = t[a as usize][b as usize];
        if m == 0 { 0 } else { Self::mu(c, m as i32) }
    }

    fn unbin(&mut self, r: &mut Run, c: &Ctx, end: u32) {
        let ng = Self::gap(c, r.pp, ORD);
        let delta = ng - r.gap;
        self.shift(r.prev_s, end, delta, 0);
        r.x += delta;
        r.gap = ng;
        r.prev = ORD;
    }

    #[inline(always)]
    fn place(&mut self, r: &mut Run, c: &Ctx, a: Atom) {
        let mut cl = a.class;
        if cl == BIN && BIN_TO_ORD[r.prev as usize] {
            cl = ORD;
        }
        if r.prev == BIN && (cl == REL || cl == CLOSE || cl == PUNCT) {
            self.unbin(r, c, a.bx.s);
        }
        if r.track {
            r.last = a;
            r.x0 = r.x;
            r.prev0 = r.prev;
            r.pp0 = r.pp;
            r.gap0 = r.gap;
            r.prev_s0 = r.prev_s;
        }
        let g = Self::gap(c, r.prev, cl);
        let x = r.x + g;
        self.shift(a.bx.s, a.bx.e, x, 0);
        r.at_x = x;
        r.end_x = x + a.bx.w;
        r.pp = r.prev;
        r.prev = cl;
        r.gap = g;
        r.prev_s = a.bx.s;
        r.x = x + a.bx.w;
        if a.bx.h > r.h {
            r.h = a.bx.h;
        }
        if a.bx.d > r.d {
            r.d = a.bx.d;
        }
        r.count += 1;
    }

    fn enter(&mut self) -> R<()> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return self.fail(E_DEPTH);
        }
        Ok(())
    }

    #[inline(always)]
    fn new_run(&self, s: u32, init: u8, ctx: Ctx) -> Run {
        Run {
            x: 0,
            h: i32::MIN / 2,
            d: i32::MIN / 2,
            prev: init,
            pp: NONE,
            gap: 0,
            prev_s: s,
            count: 0,
            mbase: self.mids.len(),
            track: false,
            last: Self::plain(Bx::default(), ORD),
            at_x: 0,
            end_x: 0,
            x0: 0,
            prev0: NONE,
            pp0: NONE,
            gap0: 0,
            prev_s0: s,
            start: self.pos,
            first: s,
            ctx,
        }
    }

    fn list(&mut self, ctx: Ctx, stop: u8, init: u8) -> R<Bx> {
        self.enter()?;
        let s = self.end();
        let mut r = self.new_run(s, init, ctx);
        let mut c = ctx;
        let done = self.run(&mut r, &mut c, stop)?;
        self.depth -= 1;
        if let Some(b) = done {
            self.one_class = ORD;
            return Ok(b);
        }
        if r.prev == BIN {
            let e = self.end();
            self.unbin(&mut r, &c, e);
        }
        self.one_class = if r.count == 1 { r.prev } else { ORD };
        if r.count == 0 {
            r.h = 0;
            r.d = 0;
        }
        Ok(self.close(s, r.x, r.h, r.d))
    }

    #[inline(always)]
    fn sym_atom(&mut self, c: &Ctx, s: &Sym) -> Atom {
        let mut g = s.glyph;
        if c.font != 0 && s.flags & F_ORD != 0 {
            if let Some(v) = glyph_id(variant_font(c.font, s), s.cp) {
                g = v;
            }
        }
        let m = &self.glyph_metric(g as usize);
        let st = self.end();
        let natural = sc(m.w as i32, c.scale);
        let mut width = natural;
        if matches!(s.cp, 0x2026 | 0x22ef) {
            width = sc(DOTS_WIDTH, c.scale);
        } else if self.point_size(c.scale) != 10000 {
            let font = [FONT_MATH_ITALIC, FONT_MAIN_REGULAR, FONT_MAIN_BOLD].into_iter().find(|font| glyph_id(*font, s.cp) == Some(g));
            width = font.map_or(natural, |font| self.advance(font, s.cp, natural, c.scale));
        }
        self.push_glyph(g, (width - natural) / 2, 0, c.grp);
        let it = if c.font >= FV_RM && c.font <= FV_TT { 0 } else { sc(m.it as i32, c.scale) };
        Atom {
            bx: self.close(st, width + it, sc(m.h as i32, c.scale), sc(m.d as i32, c.scale)),
            class: s.class,
            italic: it,
            charbox: true,
            limits: false,
        }
    }

    #[inline(always)]
    fn plain(bx: Bx, class: u8) -> Atom {
        Atom { bx, class, italic: 0, charbox: false, limits: false }
    }

    fn single_glyph(&self, b: &Bx) -> Option<u16> {
        if b.e == b.s + 1 && self.items[b.s as usize].kind == IT_GLYPH { Some(self.items[b.s as usize].a as u16) } else { None }
    }

    fn run(&mut self, r: &mut Run, c: &mut Ctx, stop: u8) -> R<Option<Bx>> {
        loop {
            let ch = self.peek();
            match CT[ch as usize] {
                T_SYM => {
                    let s = &ASCII[ch as usize];
                    if ch == b']' && stop == S_BRACKET {
                        self.pos += 1;
                        return Ok(None);
                    }
                    if s.kind == K_NONE {
                        return self.fail(E_CHAR);
                    }
                    self.pos += 1;
                    let a = self.sym_atom(c, s);
                    let a = self.scripts(c, a)?;
                    self.place(r, c, a);
                }
                T_WS => self.pos += 1,
                T_LB => {
                    let a = self.group(c)?;
                    let a = self.scripts(c, a)?;
                    self.place(r, c, a);
                }
                T_RB => {
                    if stop == S_BRACE || stop == S_SUBCELL {
                        self.pos += 1;
                        return Ok(None);
                    }
                    return self.fail(E_BRACE);
                }
                T_CMD => {
                    let tok = self.pos;
                    self.pos += 1;
                    let name = self.read_name();
                    if self.control_command(name)? {
                        continue;
                    }
                    if !self.defs.is_empty() {
                        if let Some(d) = self.find_def(name) {
                            self.expand_into(r, c, &d)?;
                            continue;
                        }
                    }
                    // amsmath leaves a thin space after dots that end a formula.
                    let closing = matches!(name, b"dots" | b"cdots") && self.peek() == b'$' && self.package_loaded(b"amsmath");
                    let name: &[u8] = if closing && name == b"dots" { b"ldots" } else { name };
                    let Some(s) = lookup(name) else {
                        if self.core(name, r, c)? {
                            continue;
                        }
                        if self.lenient {
                            self.ignore_command(tok);
                            continue;
                        }
                        self.pos = tok;
                        return self.fail(E_CMD);
                    };
                    match s.kind {
                        K_SYM => {
                            if s.flags & F_TEXTONLY != 0 {
                                self.pos = tok;
                                return self.fail(E_CMD);
                            }
                            let mut a = self.sym_atom(c, s);
                            if closing {
                                a.bx.w += Self::mu(c, THIN_MU);
                            }
                            let a = self.scripts(c, a)?;
                            self.place(r, c, a);
                        }
                        K_STYLE => *c = self.restyle(c, s.arg as u8),
                        K_OLDFONT => c.font = s.arg as u8,
                        K_SIZE => {
                            c.size = s.arg;
                            *c = self.restyle(c, c.style);
                        }
                        K_COLOR => {
                            c.color = self.color_arg()?;
                            c.grp = self.grp(c.color, c.scale);
                        }
                        K_MUKERN => r.x += Self::mu(c, s.arg as i16 as i32),
                        K_EMKERN => r.x += sc(s.arg as i32, c.scale),
                        K_HSPACE => r.x += self.length_arg(c)?,
                        K_NOOP => {}
                        K_INFIX => {
                            if self.over_stop == self.p as usize + tok {
                                return Ok(None);
                            }
                            if stop == S_MACRO {
                                return self.fail(E_MISPLACED);
                            }
                            self.mids.truncate(r.mbase);
                            return self.infix(s.arg as u8, tok, stop, r).map(Some);
                        }
                        K_RIGHT => {
                            if stop == S_RIGHT {
                                return Ok(None);
                            }
                            return self.fail(E_MISPLACED);
                        }
                        K_MIDDLE => {
                            if stop != S_RIGHT {
                                return self.fail(E_MISPLACED);
                            }
                            let cp = self.delim_token()?;
                            self.mids.push(Mid { cp, idx: self.end(), x: r.x });
                            r.prev = OPEN;
                        }
                        K_CR => {
                            if stop == S_CELL || stop == S_SUBCELL {
                                self.stop_tok = TOK_CR;
                                return Ok(None);
                            }
                            return self.fail(E_MISPLACED);
                        }
                        K_END => {
                            if stop == S_CELL {
                                self.stop_tok = TOK_END;
                                return Ok(None);
                            }
                            return self.fail(E_MISPLACED);
                        }
                        K_MACRO => {
                            self.run_bytes(macro_text(s), s.arg as usize, r, c)?;
                        }
                        _ => {
                            let a = self.fn_atom(c, s)?;
                            let a = self.scripts(c, a)?;
                            self.place(r, c, a);
                        }
                    }
                }
                T_SCRIPT => {
                    let st = self.end();
                    let a = Self::plain(Bx { s: st, e: st, ..Bx::default() }, ORD);
                    let a = self.scripts(c, a)?;
                    self.place(r, c, a);
                }
                T_AMP => {
                    if stop == S_CELL {
                        self.pos += 1;
                        self.stop_tok = TOK_AMP;
                        return Ok(None);
                    }
                    return self.fail(E_MISPLACED);
                }
                T_NUL => {
                    if !self.at_end() {
                        return self.fail(E_CHAR);
                    }
                    if stop == S_EOF || stop == S_MACRO {
                        return Ok(None);
                    }
                    return self.fail(E_END);
                }
                T_DOLLAR => {
                    if stop == S_DOLLAR {
                        self.pos += 1;
                        return Ok(None);
                    }
                    return self.fail(E_MISPLACED);
                }
                T_TILDE => {
                    self.pos += 1;
                    r.x += sc(250, c.scale);
                }
                T_PCT => self.skip_ws(),
                T_UTF8 => {
                    let a = self.utf8_atom(c)?;
                    let a = self.scripts(c, a)?;
                    self.place(r, c, a);
                }
                _ => return self.fail(E_CHAR),
            }
        }
    }

    fn group(&mut self, c: &Ctx) -> R<Atom> {
        self.pos += 1;
        let b = self.list(*c, S_BRACE, NONE)?;
        let mut a = Self::plain(b, ORD);
        if self.single_glyph(&b).is_some() {
            a.charbox = true;
        }
        Ok(a)
    }

    fn utf8_atom(&mut self, c: &Ctx) -> R<Atom> {
        let tok = self.pos;
        let (b, cp) = self.read_utf8()?;
        let Some(s) = lookup(b) else {
            if cp == NOT_EQUAL {
                if let Some(s) = lookup(b"neq") {
                    let bx = self.list_in(macro_text(s), s.arg as usize, *c)?;
                    return Ok(Self::plain(bx, REL));
                }
            }
            self.pos = tok;
            return self.fail(E_CHAR);
        };
        self.fn_atom(c, s)
    }

    fn infix(&mut self, arg: u8, tok: usize, stop: u8, r: &Run) -> R<Bx> {
        self.reparses += 1;
        if self.reparses > MAX_REPARSE {
            return self.fail(E_DEPTH);
        }
        let after = self.pos;
        let orig = &r.ctx;
        self.items.truncate(r.first as usize);
        self.pos = r.start;
        let nc = self.restyle(orig, NUM[orig.style as usize]);
        let dc = self.restyle(orig, DEN[orig.style as usize]);
        let saved = self.over_stop;
        self.over_stop = self.p as usize + tok;
        let num = self.list(nc, stop, NONE)?;
        self.over_stop = saved;
        self.pos = after;
        let den = self.list(dc, stop, NONE)?;
        Ok(self.frac_layout(orig, num, den, arg & (FRAC_NO_BAR | FRAC_PAREN) == 0, arg & FRAC_PAREN != 0))
    }

    fn arg(&mut self, c: &Ctx) -> R<Bx> {
        self.skip_ws();
        let ch = self.peek();
        match CT[ch as usize] {
            T_LB => {
                self.pos += 1;
                self.list(*c, S_BRACE, NONE)
            }
            T_SYM => {
                let s = &ASCII[ch as usize];
                if s.kind == K_NONE {
                    return self.fail(E_CHAR);
                }
                self.pos += 1;
                Ok(self.sym_atom(c, s).bx)
            }
            T_CMD => {
                self.enter()?;
                let tok = self.pos;
                self.pos += 1;
                let name = self.read_name();
                let mut def: Option<Def> = None;
                if !self.defs.is_empty() {
                    def = self.find_def(name);
                }
                if def.is_none() && matches!(name, b"oiint" | b"oiiint") {
                    self.depth -= 1;
                    return Ok(self.contour_op(c, if name == b"oiint" { 2 } else { 3 }).bx);
                }
                let sym = lookup(name);
                if def.is_none() && sym.is_none() {
                    def = self.autoload(name)?;
                }
                if let Some(d) = def {
                    let b = self.def_box(&d, c)?;
                    self.depth -= 1;
                    return Ok(b);
                }
                let Some(s) = sym else {
                    if self.lenient {
                        self.ignore_command(tok);
                        return Ok(self.close(self.end(), 0, 0, 0));
                    }
                    self.pos = tok;
                    return self.fail(E_CMD);
                };
                let b = match s.kind {
                    K_SYM => self.sym_atom(c, s).bx,
                    K_MACRO => self.list_in(macro_text(s), s.arg as usize, *c)?,
                    K_STYLE | K_COLOR | K_SIZE | K_OLDFONT | K_MUKERN | K_EMKERN | K_HSPACE | K_NOOP | K_INFIX | K_RIGHT | K_MIDDLE
                    | K_CR | K_END | K_LIMITS | K_HLINE => {
                        self.pos = tok;
                        return self.fail(E_ARG);
                    }
                    _ => self.fn_atom(c, s)?.bx,
                };
                self.depth -= 1;
                Ok(b)
            }
            T_UTF8 => Ok(self.utf8_atom(c)?.bx),
            _ => self.fail(E_ARG),
        }
    }

    fn raw_group(&mut self) -> R<(usize, usize)> {
        self.skip_ws();
        if self.peek() != b'{' {
            return self.fail(E_ARG);
        }
        self.pos += 1;
        let s = self.pos;
        while !self.at_end() && self.peek() != b'}' {
            self.pos += 1;
        }
        if self.at_end() {
            return self.fail(E_END);
        }
        let e = self.pos;
        self.pos += 1;
        Ok((s, e))
    }

    fn color_arg(&mut self) -> R<u16> {
        let (s, e) = self.raw_group()?;
        let b = self.slice(s, e);
        for &(name, id) in self.color_defs.iter().rev() {
            if name == b {
                return Ok(id);
            }
        }
        if b.is_empty() || b.len() > 32 || !b.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'#') {
            return self.fail(E_COLOR);
        }
        self.intern_color(b)
    }

    fn intern_color(&mut self, b: &[u8]) -> R<u16> {
        for (i, c) in self.colors.iter().enumerate().skip(1) {
            if &self.color_bytes[c.0 as usize..(c.0 + c.1) as usize] == b {
                return Ok(i as u16);
            }
        }
        if self.colors.len() >= 0xfff0 {
            return self.fail(E_COLOR);
        }
        let o = self.color_bytes.len() as u32;
        self.color_bytes.extend_from_slice(b);
        self.colors.push((o, b.len() as u32));
        Ok((self.colors.len() - 1) as u16)
    }

    fn length(&mut self, c: &Ctx) -> R<i32> {
        self.skip_ws();
        let mut neg = false;
        loop {
            match self.peek() {
                b'-' => {
                    neg = !neg;
                    self.pos += 1;
                }
                b'+' => self.pos += 1,
                _ => break,
            }
        }
        let mut v: i64 = 0;
        let mut frac: i64 = 0;
        let mut scale: i64 = 1;
        let mut any = false;
        while self.peek().is_ascii_digit() {
            if v < MAX_DIM {
                v = v * 10 + (self.peek() - b'0') as i64;
            }
            any = true;
            self.pos += 1;
        }
        if self.peek() == b'.' {
            self.pos += 1;
            while self.peek().is_ascii_digit() {
                if scale < 1_000_000 {
                    frac = frac * 10 + (self.peek() - b'0') as i64;
                    scale *= 10;
                }
                any = true;
                self.pos += 1;
            }
        }
        if !any {
            return self.fail(E_UNIT);
        }
        self.skip_ws();
        let u = [self.peek(), self.peek_at(1)];
        let thou = v * 1000 + frac * 1000 / scale;
        let s = c.scale as i64;
        let r = match &u {
            b"em" => thou * s / 1000,
            b"ex" => thou * s * XHEIGHT as i64 / 1_000_000,
            b"mu" => thou * s * QUAD[sidx(c)] as i64 / 18_000_000,
            b"pt" => thou / 10,
            b"bp" => thou * 10038 / 100000,
            b"px" => thou / 10,
            b"mm" => thou * 28453 / 100000,
            b"cm" => thou * 28453 / 10000,
            b"in" => thou * 72270 / 10000,
            _ => return self.fail(E_UNIT),
        };
        self.pos += 2;
        let r = r.clamp(0, MAX_DIM) as i32;
        Ok(if neg { -r } else { r })
    }

    fn bracket_length(&mut self, c: &Ctx) -> R<i32> {
        self.skip_ws();
        if self.peek() != b'[' {
            return Ok(0);
        }
        self.pos += 1;
        let v = self.length(c)?;
        self.skip_ws();
        if self.peek() != b']' {
            return self.fail(E_UNIT);
        }
        self.pos += 1;
        Ok(v)
    }

    fn length_arg(&mut self, c: &Ctx) -> R<i32> {
        if !self.lenient {
            return self.exact_length_arg(c);
        }
        let held = self.pos;
        match self.exact_length_arg(c) {
            Ok(v) => Ok(v),
            Err(()) if self.err == E_UNIT => {
                self.err = 0;
                self.pos = held;
                self.loose_argument();
                Ok(0)
            }
            Err(()) => Err(()),
        }
    }

    fn exact_length_arg(&mut self, c: &Ctx) -> R<i32> {
        self.skip_ws();
        if self.peek() == b'{' {
            self.pos += 1;
            let v = self.length(c)?;
            self.skip_ws();
            if self.peek() != b'}' {
                return self.fail(E_UNIT);
            }
            self.pos += 1;
            Ok(v)
        } else {
            self.length(c)
        }
    }

    fn fn_atom(&mut self, c: &Ctx, s: &'static Sym) -> R<Atom> {
        match s.kind {
            K_ACCENT => self.accent(c, s),
            K_WIDEACCENT => self.wide_accent(c, s),
            K_OP => Ok(self.big_op(c, s)),
            K_NAMEDOP => self.named_op(c, s),
            K_OPERATORNAME => {
                let mut oc = *c;
                oc.font = FV_RM;
                let bx = self.arg(&oc)?;
                let mut a = Self::plain(bx, OP);
                a.limits = s.arg != 0 && c.style < ST_T;
                Ok(a)
            }
            K_FRAC => self.frac(c, s.arg as u8),
            K_SQRT => self.sqrt(c),
            K_LEFT => self.left_right(c),
            K_BIG => {
                let cp = self.delim_token()?;
                self.check_delim(cp)?;
                let bx = self.delim_sized(c, cp, s.arg as usize);
                Ok(Self::plain(bx, s.class))
            }
            K_FONT => {
                let mut fc = *c;
                fc.font = s.arg as u8;
                let bx = self.arg(&fc)?;
                Ok(Self::plain(bx, ORD))
            }
            K_TEXT => {
                let bx = self.text_arg(c, s.arg as u8)?;
                Ok(Self::plain(bx, ORD))
            }
            K_TEXTCOLOR => {
                let mut cc = *c;
                cc.color = self.color_arg()?;
                cc.grp = self.grp(cc.color, cc.scale);
                let bx = self.arg(&cc)?;
                Ok(Self::plain(bx, ORD))
            }
            K_BEGIN => self.environment(c),
            K_PHANTOM => {
                let mut b = self.arg(c)?;
                self.items.truncate(b.s as usize);
                b.e = b.s;
                match s.arg {
                    1 => {
                        b.h = 0;
                        b.d = 0;
                    }
                    2 => b.w = 0,
                    _ => {}
                }
                Ok(Self::plain(b, ORD))
            }
            K_SMASH => {
                let mut b = self.arg(c)?;
                b.h = 0;
                b.d = 0;
                Ok(Self::plain(b, ORD))
            }
            K_LAP => {
                let mut b = self.arg(c)?;
                let dx = match s.arg {
                    1 => -b.w,
                    2 => -b.w / 2,
                    _ => 0,
                };
                self.move_box(&b, dx, 0);
                b.w = 0;
                Ok(Self::plain(b, ORD))
            }
            K_NOT => {
                let st = self.end();
                let g = glyph_id(FONT_MAIN_REGULAR, 0xe020).unwrap_or(0);
                self.push_glyph(g, 0, 0, c.grp);
                let m = &self.glyph_metric(g as usize);
                Ok(self.atom(st, 0, sc(m.h as i32, c.scale), sc(m.d as i32, c.scale), REL))
            }
            K_BOXED => self.boxed(c),
            K_CLASS => {
                let b = self.arg(c)?;
                let mut a = Self::plain(b, s.arg as u8);
                a.limits = s.arg as u8 == OP && c.style < ST_T;
                Ok(a)
            }
            K_STACK => self.stack(c, s.arg as u8),
            K_LINE => self.line(c, s.arg as u8),
            K_ARROW => self.over_arrow(c, s.arg as u8),
            K_XARROW => self.x_arrow(c, s.arg as u8),
            K_BRACE => self.brace(c, s.arg as u8),
            K_PMOD => self.pmod(c, s.arg as u8),
            K_SUBSTACK => self.substack(c),
            K_RULE => self.rule(c),
            K_SYM => Ok(self.sym_atom(c, s)),
            _ => self.fail(E_MISPLACED),
        }
    }

    fn scripts(&mut self, c: &Ctx, mut a: Atom) -> R<Atom> {
        let mut ch = self.peek();
        if ch == b' ' || ch == b'\n' || ch == b'\t' || ch == b'\r' {
            self.skip_ws();
            ch = self.peek();
        }
        if a.class == OP && ch == b'\\' {
            let save = self.pos;
            self.pos += 1;
            let n = self.read_name();
            if n == b"limits" {
                a.limits = true;
            } else if n == b"nolimits" {
                a.limits = false;
            } else if n == b"displaylimits" {
            } else {
                self.pos = save;
                return Ok(a);
            }
            self.skip_ws();
            ch = self.peek();
        }
        if ch != b'^' && ch != b'_' && ch != b'\'' {
            return Ok(a);
        }
        let upc = self.restyle(c, SUP[c.style as usize]);
        let dnc = self.restyle(c, SUB[c.style as usize]);
        let mut sup: Option<Bx> = None;
        let mut sub: Option<Bx> = None;
        let mut explicit_sup = false;
        loop {
            self.skip_ws();
            match self.peek() {
                b'^' => {
                    if explicit_sup {
                        return self.fail(E_SCRIPT);
                    }
                    explicit_sup = true;
                    self.pos += 1;
                    let b = self.arg(&upc)?;
                    sup = Some(self.join(sup, b));
                }
                b'_' => {
                    if sub.is_some() {
                        return self.fail(E_SCRIPT);
                    }
                    self.pos += 1;
                    sub = Some(self.arg(&dnc)?);
                }
                b'\'' => {
                    let g = glyph_id(FONT_MAIN_REGULAR, 0x2032).unwrap_or(0);
                    while self.peek() == b'\'' {
                        self.pos += 1;
                        let st = self.end();
                        self.push_glyph(g, 0, 0, upc.grp);
                        let m = &self.glyph_metric(g as usize);
                        let width = self.advance(FONT_MAIN_REGULAR, 0x2032, sc(m.w as i32, upc.scale), upc.scale);
                        let b = self.close(st, width, sc(m.h as i32, upc.scale), sc(m.d as i32, upc.scale));
                        sup = Some(self.join(sup, b));
                    }
                }
                _ => break,
            }
        }
        if a.limits {
            return Ok(self.limits_layout(c, a, sup, sub));
        }
        let (mut us, mut ds) = (0, 0);
        if !a.charbox {
            us = a.bx.h - sg(&upc, &SUPDROP);
            ds = a.bx.d + sg(&dnc, &SUBDROP);
        }
        let min_sup = if c.style == ST_D {
            sg(c, &SUP1)
        } else if c.style & 1 != 0 {
            sg(c, &SUP3)
        } else {
            sg(c, &SUP2)
        };
        let xh = sc(XHEIGHT, c.scale);
        let sw = sc(50, c.size);
        let base_w = a.bx.w;
        let mut w = base_w;
        let mut h = a.bx.h;
        let mut d = a.bx.d;
        match (sup, sub) {
            (Some(u), Some(l)) => {
                us = us.max(min_sup).max(u.d + xh / 4);
                ds = ds.max(sg(c, &SUB2));
                let rw = sg(c, &RULE);
                let mw = 4 * rw;
                if (us - u.d) - (l.h - ds) < mw {
                    ds = mw - (us - u.d) + l.h;
                    let psi = xh * 4 / 5 - (us - u.d);
                    if psi > 0 {
                        us += psi;
                        ds -= psi;
                    }
                }
                let lx = base_w - a.italic;
                self.move_box(&u, base_w, -us);
                self.move_box(&l, lx, ds);
                w = (base_w + u.w).max(lx + l.w) + sw;
                h = h.max(us + u.h);
                d = d.max(ds + l.d);
            }
            (Some(u), None) => {
                us = us.max(min_sup).max(u.d + xh / 4);
                self.move_box(&u, base_w, -us);
                w = base_w + u.w + sw;
                h = h.max(us + u.h);
                d = d.max(u.d - us);
            }
            (None, Some(l)) => {
                ds = ds.max(sg(c, &SUB1)).max(l.h - xh * 4 / 5);
                let lx = base_w - a.italic;
                self.move_box(&l, lx, ds);
                w = (lx + l.w).max(base_w) + sw;
                h = h.max(l.h - ds);
                d = d.max(ds + l.d);
            }
            _ => {}
        }
        a.bx = self.close(a.bx.s, w, h, d);
        Ok(a)
    }

    fn join(&mut self, a: Option<Bx>, b: Bx) -> Bx {
        match a {
            None => b,
            Some(p) => {
                self.move_box(&b, p.w, 0);
                Bx { s: p.s, e: b.e, w: p.w + b.w, h: p.h.max(b.h), d: p.d.max(b.d) }
            }
        }
    }

    fn limits_layout(&mut self, c: &Ctx, mut a: Atom, sup: Option<Bx>, sub: Option<Bx>) -> Atom {
        let slant = a.italic;
        let base_w = a.bx.w;
        let mut w = base_w;
        if let Some(u) = &sup {
            w = w.max(u.w + slant);
        }
        if let Some(l) = &sub {
            w = w.max(l.w + slant);
        }
        let bx = a.bx;
        self.move_box(&bx, (w - base_w) / 2, 0);
        let mut h = a.bx.h;
        let mut d = a.bx.d;
        if let Some(u) = &sup {
            let k = sg(c, &BIGOP1).max(sg(c, &BIGOP3) - u.d);
            let y = -(h + k + u.d);
            self.move_box(u, (w - u.w + slant) / 2, y);
            h = -y + u.h + sg(c, &BIGOP5);
        }
        if let Some(l) = &sub {
            let k = sg(c, &BIGOP2).max(sg(c, &BIGOP4) - l.h);
            let y = d + k + l.h;
            self.move_box(l, (w - l.w - slant) / 2, y);
            d = y + l.d + sg(c, &BIGOP5);
        }
        a.bx = self.close(a.bx.s, w, h, d);
        a.italic = 0;
        a
    }

    fn contour_op(&mut self, c: &Ctx, count: i32) -> Atom {
        let mut atom = self.big_op(c, lookup(b"int").unwrap());
        let first = self.items[atom.bx.s as usize];
        let step = atom.bx.w - sc(300, c.scale);
        for i in 1..count {
            self.push_glyph(first.a as u16, first.x + i * step, first.y, c.grp);
        }
        atom.bx.w += (count - 1) * step;
        let radius = sc(220, c.scale);
        self.push_item(c, (atom.bx.w - atom.italic) / 2, -sc(AXIS, c.scale),
            (count - 1) * step / 2 + radius, radius, IT_ELLIPSE);
        atom.bx.e = self.end();
        atom
    }

    fn big_op(&mut self, c: &Ctx, s: &Sym) -> Atom {
        let big = c.style < ST_T;
        let g = if big { s.arg } else { s.glyph };
        let m = &self.glyph_metric(g as usize);
        let st = self.end();
        let (h, d) = (sc(m.h as i32, c.scale), sc(m.d as i32, c.scale));
        let shift = (h - d) / 2 - sc(AXIS, c.scale);
        self.push_glyph(g, 0, shift, c.grp);
        let it = sc(m.it as i32, c.scale);
        Atom {
            bx: self.close(st, sc(m.w as i32, c.scale) + it, h - shift, d + shift),
            class: OP,
            italic: it,
            charbox: false,
            limits: s.flags & F_LIMITS != 0 && big,
        }
    }

    fn named_op(&mut self, c: &Ctx, s: &Sym) -> R<Atom> {
        let mut oc = *c;
        oc.font = FV_RM;
        let b = self.list_in(macro_text(s), s.arg as usize, oc)?;
        let mut a = Self::plain(b, OP);
        a.limits = s.flags & F_LIMITS != 0 && c.style < ST_T;
        Ok(a)
    }

    fn frac(&mut self, c: &Ctx, arg: u8) -> R<Atom> {
        let v = arg & FRAC_STYLE;
        let fc = match v {
            1 | 3 => self.restyle(c, ST_D),
            2 => self.restyle(c, ST_T),
            _ => *c,
        };
        let (ns, ds) = if v == 3 { (ST_D, ST_D) } else { (NUM[fc.style as usize], DEN[fc.style as usize]) };
        let nc = self.restyle(&fc, ns);
        let dc = self.restyle(&fc, ds);
        let mut num = self.arg(&nc)?;
        let mut den = self.arg(&dc)?;
        if v == 3 {
            num.h = num.h.max(sc(700, fc.scale));
            num.d = num.d.max(sc(300, fc.scale));
            den.h = den.h.max(sc(700, fc.scale));
            den.d = den.d.max(sc(300, fc.scale));
        }
        let binom = arg & FRAC_PAREN != 0;
        let mut b = self.frac_layout(&fc, num, den, !binom, binom);
        if v == 3 {
            b.w -= sc(NULL_DELIM, fc.size);
        }
        Ok(Self::plain(b, ORD))
    }

    fn frac_layout(&mut self, c: &Ctx, num: Bx, den: Bx, bar: bool, paren: bool) -> Bx {
        let rule = sg(c, &RULE);
        let axis = sc(AXIS, c.scale);
        let disp = c.style < ST_T;
        let (mut ns, mut ds, clear);
        if disp {
            ns = sg(c, &NUM1);
            ds = sg(c, &DENOM1);
            clear = if bar { 3 * rule } else { 7 * rule };
        } else {
            ds = sg(c, &DENOM2);
            if bar {
                ns = sg(c, &NUM2);
                clear = rule;
            } else {
                ns = sg(c, &NUM3);
                clear = 3 * rule;
            }
        }
        if bar {
            let t = (ns - num.d) - (axis + rule / 2);
            if t < clear {
                ns += clear - t;
            }
            let t = (axis - rule / 2) - (den.h - ds);
            if t < clear {
                ds += clear - t;
            }
        } else {
            let t = (ns - num.d) - (den.h - ds);
            if t < clear {
                ns += (clear - t) / 2;
                ds += (clear - t) / 2;
            }
        }
        let w = num.w.max(den.w);
        self.move_box(&num, (w - num.w) / 2, -ns);
        self.move_box(&den, (w - den.w) / 2, ds);
        let s = num.s;
        let mut h = ns + num.h;
        let mut d = ds + den.d;
        if bar {
            self.push_rule(c, 0, -(axis + rule / 2), w, rule);
            h = h.max(axis + rule / 2);
            d = d.max(rule / 2 - axis);
        }
        if paren {
            let total = if disp { sg(c, &DELIM1) } else { sg(c, &DELIM2) };
            let inner = self.close(s, w, h, d);
            return self.fence(c, inner, 0x28, 0x29, total);
        }
        let n = sc(NULL_DELIM, c.size);
        self.shift(s, self.end(), n, 0);
        self.close(s, w + 2 * n, h, d)
    }

    fn fence(&mut self, c: &Ctx, b: Bx, left: u32, right: u32, total: i32) -> Bx {
        let l = self.delim(c, left, total);
        self.move_box(&b, l.w, 0);
        let r = self.delim(c, right, total);
        self.move_box(&r, l.w + b.w, 0);
        self.close(b.s, l.w + b.w + r.w, b.h.max(l.h).max(r.h), b.d.max(l.d).max(r.d))
    }

    fn size_ladder(c: &Ctx) -> ([(u8, u16); 7], usize) {
        let size = c.size;
        let ladder = [
            (FONT_MAIN_REGULAR, sc(500, size) as u16),
            (FONT_MAIN_REGULAR, sc(700, size) as u16),
            (FONT_MAIN_REGULAR, size),
            (FONT_SIZE1_REGULAR, size),
            (FONT_SIZE2_REGULAR, size),
            (FONT_SIZE3_REGULAR, size),
            (FONT_SIZE4_REGULAR, size),
        ];
        let start = match c.style >> 1 {
            3 => 0,
            2 => 1,
            _ => 2,
        };
        (ladder, start)
    }

    fn sqrt(&mut self, c: &Ctx) -> R<Atom> {
        self.skip_ws();
        let s = self.end();
        let mut index: Option<Bx> = None;
        if self.peek() == b'[' {
            self.pos += 1;
            let ic = self.restyle(c, ST_SS);
            index = Some(self.list(ic, S_BRACKET, NONE)?);
        }
        let cc = self.restyle(c, c.style | 1);
        let mut inner = self.arg(&cc)?;
        if inner.h + inner.d == 0 {
            inner.h = sc(XHEIGHT, c.scale);
        }
        let theta = sg(c, &RULE);
        let phi = if c.style < ST_T { sc(XHEIGHT, c.scale) } else { theta };
        let mut clearance = theta + phi / 4;
        let need = inner.h + inner.d + clearance + theta;

        let size = c.size;
        let (ladder, start) = Self::size_ladder(c);
        let mut pick: Option<(u16, u16)> = None;
        for &(f, scl) in &ladder[start..] {
            let g = glyph_id(f, 0x221a).unwrap_or(0);
            let m = &self.glyph_metric(g as usize);
            if sc(m.h as i32 + m.d as i32, scl) > need {
                pick = Some((g, scl));
                break;
            }
        }
        let (rule_w, adv, img_total) = match pick {
            Some((g, scl)) => {
                let m = &self.glyph_metric(g as usize);
                (sc(THIN, scl), sc(m.w as i32, scl), sc(m.h as i32 + m.d as i32, scl))
            }
            None => (sc(THIN, size), sc(1056, size), need),
        };
        let delim_depth = img_total - rule_w;
        if delim_depth > inner.h + inner.d + clearance {
            clearance = (clearance + delim_depth - inner.h - inner.d) / 2;
        }
        let y_top = -(inner.h + clearance + rule_w);
        self.move_box(&inner, adv, 0);
        self.push_rule(c, adv, y_top, inner.w, rule_w);
        if let Some((g, scl)) = pick {
            let m = &self.glyph_metric(g as usize);
            let gg = self.grp(c.color, scl);
            self.push_glyph(g, 0, y_top + sc(m.h as i32, scl), gg);
        } else {
            let gg = self.grp(c.color, size);
            let top = glyph_id(FONT_SIZE4_REGULAR, 0xe001).unwrap_or(0);
            let bot = glyph_id(FONT_SIZE4_REGULAR, 0x23b7).unwrap_or(0);
            let tm = &self.glyph_metric(top as usize);
            let bm = &self.glyph_metric(bot as usize);
            let th = sc(tm.h as i32, size);
            self.push_glyph(top, 0, y_top + th, gg);
            let by = y_top + img_total - sc(bm.d as i32, size);
            self.push_glyph(bot, 0, by, gg);
            let bx0 = sc(702, size);
            let bw = sc(THIN, size);
            let t0 = y_top + th - OVERLAP;
            let t1 = by - sc(bm.h as i32, size) + OVERLAP;
            if t1 > t0 {
                self.push_rule(c, bx0, t0, bw, t1 - t0);
            }
        }
        let mut w = adv + inner.w;
        let mut h = -y_top + theta;
        let d = inner.d.max(y_top + img_total);
        if let Some(ix) = index {
            let lead = sc(278, c.scale);
            let tuck = sc(556, c.scale);
            let e = self.end();
            let rx = (lead + ix.w - tuck).max(0);
            self.shift(ix.e, e, rx, 0);
            let up = (h - d) * 3 / 5;
            self.move_box(&ix, lead, -up);
            h = h.max(up + ix.h);
            w += rx;
        }
        Ok(self.atom(s, w, h, d, ORD))
    }

    fn delim_token(&mut self) -> R<u32> {
        self.skip_ws();
        let ch = self.peek();
        match CT[ch as usize] {
            T_CMD => {
                self.pos += 1;
                let n = self.read_name();
                match lookup(n) {
                    Some(s) if s.kind == K_SYM => Ok(s.cp),
                    Some(s) if s.kind == K_MACRO => {
                        let b = &STRS[s.cp as usize..s.cp as usize + s.arg as usize];
                        if b.len() > 1 && b[0] == b'\\' {
                            match lookup(&b[1..]) {
                                Some(t) if t.kind == K_SYM => Ok(t.cp),
                                _ => self.fail(E_DELIM),
                            }
                        } else if b.len() == 1 {
                            Ok(Self::norm_delim(b[0] as u32))
                        } else {
                            self.fail(E_DELIM)
                        }
                    }
                    _ => self.fail(E_DELIM),
                }
            }
            T_UTF8 => {
                let (_, cp) = self.read_utf8()?;
                Ok(cp)
            }
            _ if ch == b'.' => {
                self.pos += 1;
                Ok(0)
            }
            _ if ch > 32 && ch < 127 => {
                self.pos += 1;
                Ok(Self::norm_delim(ch as u32))
            }
            _ => self.fail(E_DELIM),
        }
    }

    #[inline]
    fn norm_delim(cp: u32) -> u32 {
        match cp {
            0x3c => 0x27e8,
            0x3e => 0x27e9,
            _ => cp,
        }
    }

    fn check_delim(&mut self, cp: u32) -> R<()> {
        if cp != 0 && stack_spec(Self::norm_delim(cp)).is_none() {
            return self.fail(E_DELIM);
        }
        Ok(())
    }

    fn left_right(&mut self, c: &Ctx) -> R<Atom> {
        let lcp = self.delim_token()?;
        self.check_delim(lcp)?;
        let mbase = self.mids.len();
        let inner = self.list(*c, S_RIGHT, OPEN)?;
        let rcp = self.delim_token()?;
        self.check_delim(rcp)?;
        let total = Self::lr_total(c, inner.h, inner.d);
        let mut extra = 0;
        let mut h = inner.h;
        let mut d = inner.d;
        let inner_end = self.end();
        for i in mbase..self.mids.len() {
            let m = self.mids[i];
            self.check_delim(m.cp)?;
            let b = self.delim(c, m.cp, total);
            self.shift(m.idx, inner_end, b.w, 0);
            self.move_box(&b, m.x + extra, 0);
            extra += b.w;
            h = h.max(b.h);
            d = d.max(b.d);
        }
        self.mids.truncate(mbase);
        let body = self.close(inner.s, inner.w + extra, h, d);
        Ok(Self::plain(self.fence(c, body, lcp, rcp, total), INNER))
    }

    fn lr_total(c: &Ctx, h: i32, d: i32) -> i32 {
        let axis = sc(AXIS, c.scale);
        let m = (h - axis).max(d + axis);
        (m / 500 * 901 + (m % 500) * 901 / 500).max(2 * m - sc(500, c.scale))
    }

    fn null_delim(&self, c: &Ctx) -> Bx {
        Bx { w: sc(NULL_DELIM, c.size), ..self.close(self.end(), 0, 0, 0) }
    }

    fn glyph_box(&mut self, c: &Ctx, g: u16, scale: u16) -> Bx {
        let m = &self.glyph_metric(g as usize);
        let (h, d) = (sc(m.h as i32, scale), sc(m.d as i32, scale));
        let axis = sc(AXIS, c.size);
        let y = (h - d) / 2 - axis;
        let gg = self.grp(c.color, scale);
        let s = self.end();
        self.push_glyph(g, 0, y, gg);
        self.close(s, sc(m.w as i32, scale), h - y, d + y)
    }

    fn delim(&mut self, c: &Ctx, cp: u32, total: i32) -> Bx {
        let cp = Self::norm_delim(cp);
        let Some(spec) = stack_spec(cp) else {
            return self.null_delim(c);
        };
        let (ladder, start) = Self::size_ladder(c);
        let mut last: Option<(u16, u16)> = None;
        let n = if spec.seq == Seq::Always { 3 } else { 7 };
        for &(f, scl) in &ladder[start..n] {
            let Some(g) = glyph_id(f, cp) else { continue };
            let m = &self.glyph_metric(g as usize);
            last = Some((g, scl));
            if sc(m.h as i32 + m.d as i32, scl) > total {
                return self.glyph_box(c, g, scl);
            }
        }
        if spec.seq == Seq::Never {
            if let Some((g, scl)) = last {
                return self.glyph_box(c, g, scl);
            }
        }
        self.stacked(c, &spec, total)
    }

    fn delim_sized(&mut self, c: &Ctx, cp: u32, size: usize) -> Bx {
        let cp = Self::norm_delim(cp);
        let Some(spec) = stack_spec(cp) else {
            return self.null_delim(c);
        };
        if spec.seq == Seq::Always {
            const H: [i32; 5] = [0, 1200, 1800, 2400, 3000];
            return self.stacked(c, &spec, sc(H[size], c.size));
        }
        let f = [FONT_MAIN_REGULAR, FONT_SIZE1_REGULAR, FONT_SIZE2_REGULAR, FONT_SIZE3_REGULAR, FONT_SIZE4_REGULAR][size];
        match glyph_id(f, cp) {
            Some(g) => self.glyph_box(c, g, c.size),
            None => self.null_delim(c),
        }
    }

    fn stacked(&mut self, c: &Ctx, sp: &Stack, total: i32) -> Bx {
        let size = c.size;
        let gg = self.grp(c.color, size);
        let s = self.end();
        let piece = |cp: u32, font: u8| -> Option<u16> { if cp == 0 { None } else { glyph_id(font, cp) } };
        let top = piece(sp.top, sp.font);
        let mid = piece(sp.mid, sp.font);
        let bot = piece(sp.bot, sp.font);
        let dim = |g: Option<u16>| -> i32 {
            g.map_or(0, |g| {
                let m = &GLYPHS[g as usize];
                sc(m.h as i32 + m.d as i32, size)
            })
        };
        let min = dim(top) + dim(mid) + dim(bot);
        let total = total.max(min + sc(100, size));
        let axis = sc(AXIS, size);
        let y0 = -(total / 2 + axis);
        let y1 = y0 + total;
        let mut w = sc(sp.bars[(sp.nbars.max(1) - 1) as usize].1 + sp.bars[0].0, size);
        let mut seg_top = y0;
        let mut seg_bot = y1;
        if let Some(g) = top {
            let m = &self.glyph_metric(g as usize);
            self.push_glyph(g, 0, y0 + sc(m.h as i32, size), gg);
            seg_top = y0 + dim(top) - OVERLAP;
            w = sc(m.w as i32, size);
        }
        if let Some(g) = bot {
            let m = &self.glyph_metric(g as usize);
            self.push_glyph(g, 0, y1 - sc(m.d as i32, size), gg);
            seg_bot = y1 - dim(bot) + OVERLAP;
            w = sc(m.w as i32, size);
        }
        let mut segs = [(seg_top, seg_bot), (0, 0)];
        let mut nseg = 1;
        if let Some(g) = mid {
            let m = &self.glyph_metric(g as usize);
            let mh = dim(mid);
            let my = (y0 + y1) / 2 - mh / 2;
            self.push_glyph(g, 0, my + sc(m.h as i32, size), gg);
            segs = [(seg_top, my + OVERLAP), (my + mh - OVERLAP, seg_bot)];
            nseg = 2;
        }
        for i in 0..nseg {
            let (a, b) = segs[i];
            if b > a {
                for j in 0..sp.nbars as usize {
                    let (x0, x1) = sp.bars[j];
                    self.push_rule(c, sc(x0, size), a, sc(x1 - x0, size), b - a);
                }
            }
        }
        self.close(s, w, -y0, y1)
    }

    fn accent(&mut self, c: &Ctx, s: &Sym) -> R<Atom> {
        let cc = self.restyle(c, c.style | 1);
        let b = self.arg(&cc)?;
        let skew = self.single_glyph(&b).map_or(0, |g| sc(self.glyph_metric(g as usize).sk as i32, c.scale));
        let ink = sc(s.arg as i32 - 2000, c.scale);
        Ok(self.over_glyph(c, b, s.glyph, b.w / 2 + skew - ink))
    }

    fn over_glyph(&mut self, c: &Ctx, b: Bx, g: u16, x: i32) -> Atom {
        let clear = b.h.min(sc(XHEIGHT, c.scale));
        let y = -(b.h - clear);
        self.push_glyph(g, x, y, c.grp);
        let h = b.h.max(sc(self.glyph_metric(g as usize).h as i32, c.scale) - y);
        self.atom(b.s, b.w, h, b.d, ORD)
    }

    fn wide_accent(&mut self, c: &Ctx, s: &Sym) -> R<Atom> {
        let cc = self.restyle(c, c.style | 1);
        let b = self.arg(&cc)?;
        let fonts = [FONT_SIZE1_REGULAR, FONT_SIZE2_REGULAR, FONT_SIZE3_REGULAR, FONT_SIZE4_REGULAR];
        let mut g = glyph_id(FONT_SIZE4_REGULAR, s.cp).unwrap_or(0);
        for f in fonts {
            if let Some(v) = glyph_id(f, s.cp) {
                if sc(self.glyph_metric(v as usize).w as i32, c.scale) >= b.w {
                    g = v;
                    break;
                }
            }
        }
        let w = sc(self.glyph_metric(g as usize).w as i32, c.scale);
        Ok(self.over_glyph(c, b, g, (b.w - w) / 2))
    }

    fn line(&mut self, c: &Ctx, under: u8) -> R<Atom> {
        let cc = if under == 0 { self.restyle(c, c.style | 1) } else { *c };
        let b = self.arg(&cc)?;
        let r = sg(c, &RULE);
        let (mut h, mut d) = (b.h, b.d);
        if under == 0 {
            self.push_rule(c, 0, -(b.h + 3 * r + r), b.w, r);
            h = b.h + 5 * r;
        } else {
            self.push_rule(c, 0, b.d + 3 * r, b.w, r);
            d = b.d + 5 * r;
        }
        Ok(self.atom(b.s, b.w, h, d, ORD))
    }

    fn arrow_at(&mut self, c: &Ctx, left: bool, y_axis: i32, w: i32) {
        let cp = if left { 0x2190 } else { 0x2192 };
        let g = glyph_id(FONT_MAIN_REGULAR, cp).unwrap_or(0);
        let aw = sc(self.glyph_metric(g as usize).w as i32, c.scale);
        let shaft = sc(THIN, c.scale);
        let tip = sc(56, c.scale);
        self.push_glyph(g, if left { 0 } else { w - aw }, y_axis + sc(AXIS, c.scale), c.grp);
        if w > aw {
            let x = if left { aw / 2 } else { tip };
            self.push_rule(c, x, y_axis - shaft / 2, w - aw / 2 - tip, shaft);
        }
    }

    fn over_arrow(&mut self, c: &Ctx, left: u8) -> R<Atom> {
        let cc = self.restyle(c, c.style | 1);
        let b = self.arg(&cc)?;
        let w = b.w.max(sc(1000, c.scale));
        let dx = (w - b.w) / 2;
        self.move_box(&b, dx, 0);
        let y = -(b.h + sc(220, c.scale));
        self.arrow_at(c, left != 0, y, w);
        Ok(self.atom(b.s, w, -y + sc(130, c.scale), b.d, ORD))
    }

    fn x_arrow(&mut self, c: &Ctx, left: u8) -> R<Atom> {
        self.skip_ws();
        let s = self.end();
        let uc = self.restyle(c, SUP[c.style as usize]);
        let dc = self.restyle(c, SUB[c.style as usize]);
        let mut under: Option<Bx> = None;
        if self.peek() == b'[' {
            self.pos += 1;
            under = Some(self.list(dc, S_BRACKET, NONE)?);
        }
        let over = self.arg(&uc)?;
        let pad = sc(700, c.scale);
        let mut w = (over.w + pad).max(sc(1469, c.scale));
        if let Some(u) = &under {
            w = w.max(u.w + pad);
        }
        let ay = -sc(AXIS, c.scale);
        let gap = sc(110, c.scale);
        self.move_box(&over, (w - over.w) / 2, ay - gap - over.d);
        let mut h = -ay + gap + over.d + over.h;
        let mut d = sc(100, c.scale) + ay;
        if let Some(u) = &under {
            let y = ay + gap + u.h;
            self.move_box(u, (w - u.w) / 2, y);
            d = d.max(y + u.d);
        }
        h = h.max(sc(600, c.scale));
        self.arrow_at(c, left != 0, ay, w);
        Ok(self.atom(s, w, h, d.max(0), REL))
    }

    fn brace(&mut self, c: &Ctx, under: u8) -> R<Atom> {
        let b = self.arg(c)?;
        let bh = sc(400, c.scale);
        let gap = sc(100, c.scale);
        let y = if under == 0 { -(b.h + gap + bh) } else { b.d + gap };
        let kind = if under == 0 { IT_BRACE } else { IT_UNDERBRACE };
        self.push_item(c, 0, y, b.w, bh, kind);
        let (mut h, mut d) = (b.h, b.d);
        if under == 0 {
            h = b.h + gap + bh;
        } else {
            d = b.d + gap + bh;
        }
        let mut a = self.atom(b.s, b.w, h, d, OP);
        a.limits = true;
        Ok(a)
    }

    fn stack(&mut self, c: &Ctx, kind: u8) -> R<Atom> {
        let sc_ = self.restyle(c, SUP[c.style as usize]);
        let top = self.arg(&sc_)?;
        let base = self.arg(c)?;
        let class = self.one_class;
        let s = top.s;
        let w = top.w.max(base.w);
        let rule = sg(c, &BIGOP1);
        self.move_box(&base, (w - base.w) / 2, 0);
        let (mut h, mut d) = (base.h, base.d);
        if kind == 1 {
            let y = base.d + rule + top.h;
            self.move_box(&top, (w - top.w) / 2, y);
            d = y + top.d;
        } else {
            let y = -(base.h + rule + top.d);
            self.move_box(&top, (w - top.w) / 2, y);
            h = -y + top.h;
        }
        let cl = if kind == 2 { REL } else { class };
        Ok(self.atom(s, w, h, d, cl))
    }

    fn boxed(&mut self, c: &Ctx) -> R<Atom> {
        let b = self.arg(c)?;
        let pad = sc(300, c.scale);
        let t = sc(THIN, c.scale);
        let off = pad + t;
        self.move_box(&b, off, 0);
        let w = b.w + 2 * off;
        let h = b.h + off;
        let d = b.d + off;
        self.push_rule(c, 0, -h, w, t);
        self.push_rule(c, 0, d - t, w, t);
        self.push_rule(c, 0, -h, t, h + d);
        self.push_rule(c, w - t, -h, t, h + d);
        Ok(self.atom(b.s, w, h, d, ORD))
    }

    fn rule(&mut self, c: &Ctx) -> R<Atom> {
        let raise = self.bracket_length(c)?;
        let w = self.length_arg(c)?;
        let h = self.length_arg(c)?;
        let s = self.end();
        self.push_rule(c, 0, -(raise + h), w, h);
        Ok(self.atom(s, w, h + raise, -raise, ORD))
    }

    fn pmod(&mut self, c: &Ctx, pod: u8) -> R<Atom> {
        let s = self.end();
        let mut x = sc(1000, c.scale);
        let lp = self.sym_atom(c, &ASCII[b'(' as usize]);
        self.move_box(&lp.bx, x, 0);
        x += lp.bx.w;
        let (mut h, mut d) = (lp.bx.h, lp.bx.d);
        if pod == 0 {
            let mut rc = *c;
            rc.font = FV_RM;
            for ch in b"mod" {
                let a = self.sym_atom(&rc, &ASCII[*ch as usize]);
                self.move_box(&a.bx, x, 0);
                x += a.bx.w;
            }
            x += Self::mu(c, 6);
        }
        let b = self.arg(c)?;
        self.move_box(&b, x, 0);
        x += b.w;
        h = h.max(b.h);
        d = d.max(b.d);
        let rp = self.sym_atom(c, &ASCII[b')' as usize]);
        self.move_box(&rp.bx, x, 0);
        x += rp.bx.w;
        Ok(self.atom(s, x, h, d, ORD))
    }

    fn text_arg(&mut self, c: &Ctx, font: u8) -> R<Bx> {
        self.skip_ws();
        if self.peek() != b'{' {
            return self.fail(E_ARG);
        }
        self.pos += 1;
        self.enter()?;
        let b = self.text_run(c, font)?;
        self.depth -= 1;
        Ok(b)
    }

    fn text_metric(&mut self, font: u8, cp: u32, scale: u16) -> Option<TextMetric> {
        let key = ((font as u64) << 48) | ((scale as u64) << 32) | ((self.doc.pt as u64) << 24) | (cp as u64 + 1);
        let slot = (font as usize * 131 + cp as usize + (scale as usize & 31) * 17) & (TEXT_METRIC_SLOTS - 1);
        if cp < 128 {
            if self.text_metrics.is_empty() { self.text_metrics.resize(TEXT_METRIC_SLOTS, TextMetric::default()); }
            let metric = self.text_metrics[slot];
            if metric.key == key { return Some(metric); }
        }
        let builtin = glyph_id(font, cp).or_else(|| glyph_id(FONT_MAIN_REGULAR, cp));
        let glyph = builtin.or_else(|| self.fallback_glyph(cp))?;
        let m = self.glyph_metric(glyph as usize);
        let natural = sc(m.w as i32, scale);
        let metric = TextMetric { key, glyph, natural, width: self.advance(font, cp, natural, scale),
            h: sc(m.h as i32, scale), d: sc(m.d as i32, scale) };
        // Fallback outlines can change after registerFont. Only immutable
        // built-in ASCII metrics are cached, never dynamic Unicode glyphs.
        if cp < 128 && builtin.is_some() { self.text_metrics[slot] = metric; }
        Some(metric)
    }

    fn text_glyph(&mut self, c: &Ctx, font: u8, cp: u32, t: &mut Bx) -> R<()> {
        let Some(metric) = self.text_metric(font, cp, c.scale) else { return self.missing_glyph(c, font, cp, t); };
        t.w += self.kern_before(c, font, cp, t.w);
        self.push_glyph(metric.glyph, t.w + (metric.width - metric.natural) / 2, 0, c.grp);
        if let Some(it) = self.items.last_mut() {
            it.b = cp as i32;
        }
        self.pair.end = self.items.len();
        t.w += metric.width;
        t.h = t.h.max(metric.h);
        t.d = t.d.max(metric.d);
        Ok(())
    }

    /// How far a character moves on at this size: LaTeX loads a font file
    /// drawn for the size, whose letters are not the 10pt ones scaled.
    fn advance(&self, font: u8, cp: u32, natural: i32, scale: u16) -> i32 {
        let size = self.point_size(scale);
        if size == 10000 {
            return natural;
        }
        let Some((family, code)) = family_of(font, cp) else {
            return natural;
        };
        match font_file(family, size).widths[(code & 127) as usize] {
            0 => natural,
            width => sc(width as i32, scale),
        }
    }

    /// The size of type at this scale, in thousandths of a point.
    fn point_size(&self, scale: u16) -> i32 {
        scale as i32 * if self.doc.pt > 0 { self.doc.pt } else { 10 }
    }

    /// The interword space of a font at this size, as its width, stretch,
    /// shrink and extra space.
    pub(crate) fn space_at(&self, font: u8, scale: u16) -> (i32, i32, i32, i32) {
        let family = family_of(font, b' ' as u32).map_or(metrics::ROMAN, |(family, _)| family);
        let (width, stretch, shrink, extra) = font_file(family, self.point_size(scale)).space;
        (sc(width, scale), sc(stretch, scale), sc(shrink, scale), sc(extra, scale))
    }

    /// Anything between two letters stops TeX kerning them.
    pub(crate) fn break_pair(&mut self) {
        self.pair.end = 0;
    }

    /// The kern or ligature adjustment TeX makes between this character and
    /// the one just set, which only joins letters of one font and size.
    fn kern_before(&mut self, c: &Ctx, font: u8, cp: u32, x: i32) -> i32 {
        let code = kerns_code(cp);
        let last = self.pair;
        self.pair = Pair { font, scale: c.scale, code, end: 0 };
        if x == 0 || last.end != self.items.len() || last.font != font || last.scale != c.scale {
            return 0;
        }
        let Some((adjust, state)) = kern_pair(font, last.code, code) else {
            return 0;
        };
        self.pair.code = state;
        sc(adjust as i32, c.scale)
    }

    #[cold]
    #[inline(never)]
    fn missing_glyph(&mut self, c: &Ctx, font: u8, cp: u32, t: &mut Bx) -> R<()> {
        if let Some((base, accent)) = document::composed(cp) {
            return self.overlay(c, font, base, accent, t);
        }
        if !self.lenient {
            return self.fail(E_CHAR);
        }
        self.note(self.pos);
        self.bare_glyph(c, font, b'?' as u32, t);
        Ok(())
    }

    fn bare_glyph(&mut self, c: &Ctx, font: u8, cp: u32, t: &mut Bx) {
        if let Some(g) = glyph_id(font, cp).or_else(|| glyph_id(FONT_MAIN_REGULAR, cp)) {
            let m = &self.glyph_metric(g as usize);
            self.push_glyph(g, t.w, 0, c.grp);
            t.w += sc(m.w as i32, c.scale);
            t.h = t.h.max(sc(m.h as i32, c.scale));
            t.d = t.d.max(sc(m.d as i32, c.scale));
        }
    }

    fn overlay(&mut self, c: &Ctx, font: u8, base: u32, accent: u32, t: &mut Bx) -> R<()> {
        let start = t.w;
        self.bare_glyph(c, font, base, t);
        let Some(g) = document::accent_glyph(accent) else {
            return Ok(());
        };
        let m = &self.glyph_metric(g as usize);
        let width = sc(m.w as i32, c.scale);
        let tall = base < 0x80 && (base as u8).is_ascii_uppercase();
        let rise = if tall { sc(240, c.scale) } else { 0 };
        self.push_glyph(g, start + (t.w - start - width) / 2, -rise, c.grp);
        t.h = t.h.max(sc(m.h as i32, c.scale) + rise);
        Ok(())
    }

    fn text_run(&mut self, c: &Ctx, font: u8) -> R<Bx> {
        let mut t = self.close(self.end(), 0, 0, 0);
        let space = self.space_at(font, c.scale).0;
        loop {
            let ch = self.peek();
            match ch {
                0 => {
                    return self.fail(E_END);
                }
                b'}' => {
                    self.pos += 1;
                    break;
                }
                b'{' => {
                    let b = self.text_arg(c, font)?;
                    t = self.join(Some(t), b);
                }
                b'$' => {
                    self.pos += 1;
                    let mc = self.restyle(c, ST_T.max(c.style & !1));
                    let b = self.list(mc, S_DOLLAR, NONE)?;
                    t = self.join(Some(t), b);
                }
                b' ' | b'\n' | b'\t' | b'\r' => {
                    self.skip_ws();
                    self.break_pair();
                    t.w += space;
                }
                b'~' => {
                    self.pos += 1;
                    self.break_pair();
                    t.w += space;
                }
                b'\\' => {
                    let tok = self.pos;
                    self.pos += 1;
                    let name = self.read_name();
                    if self.control_command(name)? {
                        continue;
                    }
                    if !self.defs.is_empty() {
                        if let Some(d) = self.find_def(name) {
                            let b = self.text_def(&d, c, font)?;
                            t = self.join(Some(t), b);
                            continue;
                        }
                    }
                    let Some(sy) = lookup(name) else {
                        if self.lenient {
                            self.ignore_command(tok);
                            continue;
                        }
                        self.pos = tok;
                        return self.fail(E_CMD);
                    };
                    match sy.kind {
                        K_SYM if sy.flags & F_TEXT != 0 => self.text_glyph(c, font, sy.cp, &mut t)?,
                        K_TEXT => {
                            let b = self.text_arg(c, sy.arg as u8)?;
                            t = self.join(Some(t), b);
                        }
                        K_MUKERN => t.w += Self::mu(c, sy.arg as i16 as i32),
                        K_EMKERN => t.w += sc(sy.arg as i32, c.scale),
                        K_HSPACE => t.w += self.length_arg(c)?,
                        K_NOOP => {}
                        _ => {
                            self.pos = tok;
                            return self.fail(E_CMD);
                        }
                    }
                }
                _ if ch < 0x80 => {
                    if ch < 32 || ch == 127 {
                        return self.fail(E_CHAR);
                    }
                    self.pos += 1;
                    self.text_glyph(c, font, ch as u32, &mut t)?;
                }
                _ => {
                    let (_, cp) = self.read_utf8()?;
                    self.text_glyph(c, font, cp, &mut t)?;
                }
            }
        }
        Ok(self.close(t.s, t.w, t.h, t.d))
    }

    fn substack(&mut self, c: &Ctx) -> R<Atom> {
        self.skip_ws();
        if self.peek() != b'{' {
            return self.fail(E_ARG);
        }
        self.pos += 1;
        let s = self.end();
        let cbase = self.cells.len();
        loop {
            self.stop_tok = 0;
            let b = self.list(*c, S_SUBCELL, NONE)?;
            let tok = self.stop_tok;
            self.cells.push(Cell { bx: b, row: 0, col: 0 });
            if tok != TOK_CR {
                break;
            }
        }
        let n = self.cells.len();
        let mut w = 0;
        for ce in &self.cells[cbase..n] {
            w = w.max(ce.bx.w);
        }
        let lead = sc(60, c.scale);
        let mut y = 0;
        let h = self.cells[cbase].bx.h;
        let mut d = 0;
        for i in cbase..n {
            let b = self.cells[i].bx;
            if i > cbase {
                y += b.h;
            }
            self.move_box(&b, (w - b.w) / 2, y);
            d = y + b.d;
            y = d + lead;
        }
        self.cells.truncate(cbase);
        Ok(self.atom(s, w, h, d, ORD))
    }

    fn environment(&mut self, c: &Ctx) -> R<Atom> {
        let (ns, ne) = self.raw_group()?;
        let name = env_base(self.slice(ns, ne));
        let (style, grid, left, right) = match name {
            b"matrix" => (ST_T, Grid::Matrix, 0, 0),
            b"smallmatrix" => (ST_S, Grid::Matrix, 0, 0),
            b"pmatrix" => (ST_T, Grid::Matrix, 0x28, 0x29),
            b"bmatrix" => (ST_T, Grid::Matrix, 0x5b, 0x5d),
            b"Bmatrix" => (ST_T, Grid::Matrix, 0x7b, 0x7d),
            b"vmatrix" => (ST_T, Grid::Matrix, 0x2223, 0x2223),
            b"Vmatrix" => (ST_T, Grid::Matrix, 0x2225, 0x2225),
            b"cases" => (ST_T, Grid::Cases, 0x7b, 0),
            b"dcases" => (ST_D, Grid::Cases, 0x7b, 0),
            b"rcases" => (ST_T, Grid::Cases, 0, 0x7d),
            b"aligned" | b"align" | b"split" | b"flalign" => (ST_D, Grid::Align, 0, 0),
            b"alignat" | b"alignedat" => {
                self.raw_group()?;
                (ST_D, Grid::Align, 0, 0)
            }
            b"gathered" | b"gather" | b"equation" => (ST_D, Grid::Matrix, 0, 0),
            b"array" => (ST_T, Grid::Array, 0, 0),
            _ => {
                self.pos = ns;
                return self.fail(E_ENV);
            }
        };
        let array = grid == Grid::Array;
        let cbase = self.cells.len();
        let rbase = self.rows.len();
        let colbase = self.cols.len();
        let xbase = self.colx.len();
        let mut plain = Vec::new();
        let spec_b: &[u8] = if array {
            self.skip_ws();
            if self.peek() != b'{' {
                return self.fail(E_ARG);
            }
            let spec = self.balanced()?;
            if spec.contains(&b'{') {
                plain_spec(spec, &mut plain, 0);
                &plain
            } else {
                spec
            }
        } else {
            b""
        };
        let s = self.end();
        let cc = self.restyle(c, style);
        let mut row = 0u32;
        let mut col = 0u32;
        let mut pending_lines = 0u8;
        self.rows.push(Row::default());
        loop {
            self.stop_tok = 0;
            self.skip_ws();
            while self.peek() == b'\\' && self.slice(self.pos, (self.pos + 6).min(self.len)) == b"\\hline" {
                self.pos += 6;
                pending_lines = pending_lines.saturating_add(1);
                self.skip_ws();
            }
            if pending_lines != 0 && col == 0 {
                let i = self.rows.len() - 1;
                self.rows[i].lines = pending_lines;
                pending_lines = 0;
            }
            let init = if grid == Grid::Align && col % 2 == 1 { ORD } else { NONE };
            let b = self.list(cc, S_CELL, init)?;
            let tok = self.stop_tok;
            self.cells.push(Cell { bx: b, row, col });
            match tok {
                TOK_AMP => col += 1,
                TOK_CR => {
                    let gap = self.bracket_length(&cc)?;
                    let i = self.rows.len() - 1;
                    self.rows[i].gap = gap;
                    row += 1;
                    col = 0;
                    self.rows.push(Row::default());
                }
                _ => break,
            }
        }
        let (es, ee) = self.raw_group()?;
        if env_base(self.slice(es, ee)) != name {
            return self.fail(E_ENV);
        }
        let n = self.cells.len();
        if n > cbase {
            let last = self.cells[n - 1];
            if last.col == 0 && last.bx.s == last.bx.e && last.bx.w == 0 && row > 0 {
                self.cells.pop();
                self.rows.pop();
            }
        }
        let scale = cc.scale;
        let skip = sc(if grid == Grid::Cases { 1440 } else { 1200 }, scale);
        let strut_h = skip * 7 / 10;
        let strut_d = skip * 3 / 10;
        for i in rbase..self.rows.len() {
            self.rows[i].h = strut_h;
            self.rows[i].d = strut_d;
        }
        let mut ncols = 0usize;
        for i in cbase..self.cells.len() {
            let ce = self.cells[i];
            let ri = rbase + ce.row as usize;
            let r = &mut self.rows[ri];
            r.h = r.h.max(ce.bx.h);
            r.d = r.d.max(ce.bx.d);
            let ci = ce.col as usize;
            if ci >= ncols {
                ncols = ci + 1;
                self.cols.resize(colbase + ncols, 0);
            }
            let cw = &mut self.cols[colbase + ci];
            *cw = (*cw).max(ce.bx.w);
        }
        let mut total = 0;
        let line_w = sc(THIN, scale);
        let line_gap = sc(200, scale) / 2;
        for i in rbase..self.rows.len() {
            let r = self.rows[i];
            total += r.lines as i32 * (line_w + line_gap);
            total += r.h;
            self.rows[i].y = total;
            total += r.d + r.gap;
        }
        let trailing = pending_lines as i32 * (line_w + line_gap);
        total += trailing;
        let offset = total / 2 + sc(AXIS, scale);
        let colsep = sc(500, scale);
        let mut x = 0;
        let mut vlines = [0i32; 8];
        let mut nlines = 0usize;
        let mut sp = 0usize;
        if array {
            x = colsep + Self::spec_lines(spec_b, &mut sp, 0, line_w, &mut vlines, &mut nlines);
        }
        for ci in 0..ncols {
            let cw = self.cols[colbase + ci];
            self.colx.push(x);
            x += cw;
            let last = ci + 1 == ncols;
            if array {
                if sp < spec_b.len() {
                    sp += 1;
                }
                x += colsep;
                x += Self::spec_lines(spec_b, &mut sp, x, line_w, &mut vlines, &mut nlines);
                if !last {
                    x += colsep;
                }
            } else if !last {
                x += match grid {
                    Grid::Align if ci % 2 == 0 => 0,
                    Grid::Align | Grid::Cases => sc(1000, scale),
                    _ => 2 * colsep,
                };
            }
        }
        let width = x;
        for i in cbase..self.cells.len() {
            let ce = self.cells[i];
            let ci = ce.col as usize;
            let x0 = self.colx[xbase + ci];
            let cw = self.cols[colbase + ci];
            let dx = match column_align(grid, spec_b, ci) {
                b'l' => 0,
                b'r' => cw - ce.bx.w,
                _ => (cw - ce.bx.w) / 2,
            };
            let y = self.rows[rbase + ce.row as usize].y - offset;
            self.move_box(&ce.bx, x0 + dx, y);
        }
        for i in rbase..self.rows.len() {
            let r = self.rows[i];
            let mut yy = r.y - r.h - offset;
            for _ in 0..r.lines {
                yy -= line_w + line_gap;
                self.push_rule(&cc, 0, yy, width, line_w);
            }
        }
        let mut yy = total - trailing - offset;
        for _ in 0..pending_lines {
            self.push_rule(&cc, 0, yy, width, line_w);
            yy += line_w + line_gap;
        }
        for &vx in &vlines[..nlines] {
            self.push_rule(&cc, vx.max(0), -offset, line_w, total);
        }
        self.cells.truncate(cbase);
        self.rows.truncate(rbase);
        self.cols.truncate(colbase);
        self.colx.truncate(xbase);
        let b = self.close(s, width, offset, total - offset);
        if left | right == 0 {
            return Ok(Self::plain(b, ORD));
        }
        let t = Self::lr_total(c, b.h, b.d);
        Ok(Self::plain(self.fence(c, b, left, right, t), INNER))
    }

    fn spec_lines(spec: &[u8], sp: &mut usize, x: i32, lw: i32, out: &mut [i32; 8], n: &mut usize) -> i32 {
        let mut adv = 0;
        while *sp < spec.len() {
            match spec[*sp] {
                b'|' => {
                    if *n < 8 {
                        out[*n] = x + adv;
                        *n += 1;
                    }
                    adv += lw;
                }
                b' ' => {}
                _ => break,
            }
            *sp += 1;
        }
        adv
    }
}
