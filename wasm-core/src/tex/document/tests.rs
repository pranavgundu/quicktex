use super::*;
use crate::tex::tests::{Rng, feed, mutate, output, well_formed};

mod bibliography;
mod compat;
mod drawing;
mod floats;
mod headers;
mod improvements;
mod layout;
mod output_pdf;
mod pages;
mod plots;
mod robust;
mod setup;

pub(crate) const LAB: &str = include_str!("../../../../tests/fixtures/lab.tex");
pub(crate) const SHAPES: &str = include_str!("../../../../tests/fixtures/shapes.tex");
pub(crate) const SAMPLE: &str = include_str!("../../../../tests/fixtures/sample.tex");

pub(crate) struct Done {
    pub(crate) e: Engine,
    pub(crate) pages: usize,
}

pub(crate) fn run(e: &mut Engine, s: &str) -> Result<usize, (String, usize)> {
    let n = feed(e, s);
    let size = e.render_document(n);
    if size >= 0 {
        Ok(size as usize)
    } else {
        let message = {
            let bytes = unsafe { core::slice::from_raw_parts(e.output_ptr(), (-size) as usize) };
            String::from_utf8_lossy(bytes).into_owned()
        };
        Err((message, e.error_pos()))
    }
}

pub(crate) fn compile(s: &str) -> Result<Done, (String, usize)> {
    let mut e = Engine::new();
    let pages = run(&mut e, s)?;
    Ok(Done { e, pages })
}

pub(crate) fn good(s: &str) -> Done {
    compile(s).unwrap_or_else(|why| panic!("{why:?}\n{s}"))
}

pub(crate) fn article(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

pub(crate) fn with(preamble: &str, body: &str) -> String {
    format!("\\documentclass{{article}}\n{preamble}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

pub(crate) fn body(s: &str) -> Done {
    good(&article(s))
}

pub(crate) fn words(n: usize) -> String {
    let list = ["alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta"];
    (0..n).map(|i| list[i % list.len()]).collect::<Vec<_>>().join(" ")
}

impl Done {
    pub(crate) fn laid(&self) -> &Laid {
        &self.e.laid
    }

    pub(crate) fn raw_items(&self, page: usize) -> &[Item] {
        let l = self.laid();
        let s = if page == 0 { 0 } else { l.pages[page - 1] as usize };
        &l.items[s..l.pages[page] as usize]
    }

    pub(crate) fn items(&self, page: usize) -> &[Item] {
        let all = self.raw_items(page);
        &all[..all.len() - (page + 1).to_string().len()]
    }

    pub(crate) fn all_items(&self) -> &'static [Item] {
        (0..self.pages).flat_map(|p| self.items(p).iter().copied()).collect::<Vec<_>>().leak()
    }

    pub(crate) fn art(&self, page: usize) -> &[Art] {
        let l = self.laid();
        let s = if page == 0 { 0 } else { l.art_pages[page - 1] as usize };
        &l.art[s..l.art_pages[page] as usize]
    }

    pub(crate) fn all_art(&self) -> &[Art] {
        &self.laid().art
    }

    pub(crate) fn segs(&self, a: &Art) -> Vec<(u8, i32, i32)> {
        self.laid().segs[a.s as usize..a.e as usize].iter().map(|g| (g.op, g.x + a.x, g.y + a.y)).collect()
    }

    pub(crate) fn page(&mut self, n: usize) -> String {
        let size = self.e.document_page(n);
        output(&self.e, size as usize)
    }

    pub(crate) fn pdf(&mut self) -> Vec<u8> {
        let size = self.e.document_pdf() as usize;
        unsafe { core::slice::from_raw_parts(self.e.output_ptr(), size) }.to_vec()
    }

    pub(crate) fn glyphs(&self) -> usize {
        self.all_items().iter().filter(|i| i.kind == IT_GLYPH).count()
    }

    pub(crate) fn baselines(&self, page: usize) -> Vec<i32> {
        let mut ys: Vec<i32> = self.items(page).iter().filter(|i| i.kind == IT_GLYPH).map(|i| i.y).collect();
        ys.sort();
        ys.dedup();
        ys
    }

    pub(crate) fn left_edge(&self, page: usize) -> i32 {
        self.items(page).iter().map(|i| i.x).min().unwrap()
    }

    pub(crate) fn right_edge(&self, page: usize) -> i32 {
        self.items(page).iter().map(|i| i.x).max().unwrap()
    }

    pub(crate) fn top_edge(&self, page: usize) -> i32 {
        self.items(page).iter().map(|i| i.y).min().unwrap()
    }

    pub(crate) fn bottom_edge(&self, page: usize) -> i32 {
        self.items(page).iter().map(|i| i.y).max().unwrap()
    }
}

mod counters;
