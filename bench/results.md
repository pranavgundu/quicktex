# Results

Run on 2026-11-05. Apple M1, 8 cores, 16 GB, darwin arm64. Regenerate with `bun run bench`.

## Source to PDF on disk

Median wall time from a fresh process to a finished PDF. Lower is better. The last column is the speedup over whichever other compiler was fastest on that document.

| Document | quicktex | pdflatex | xelatex | lualatex | tectonic | vs fastest other |
| --- | --- | --- | --- | --- | --- | --- |
| note | 17 ms | 263 ms | 374 ms | 410 ms | 214 ms | 13x |
| paper | 23 ms | 772 ms | 721 ms | 1.3 s | 270 ms | 12x |
| formulas | 21 ms | 320 ms | 378 ms | 473 ms | 269 ms | 13x |
| book | 80 ms | 1.4 s | 1.5 s | 4.7 s | 868 ms | 11x |
| drawing | 19 ms | 591 ms | 690 ms | 745 ms | 733 ms | 32x |
| plots | 19 ms | 1.3 s | 1.1 s | 1.1 s | 1.8 s | 57x |
| tree | 48 ms | 5.9 s | 6.2 s | 5.5 s | 10.3 s | 116x |

## How many times faster quicktex is

| Document | pdflatex | xelatex | lualatex | tectonic |
| --- | --- | --- | --- | --- |
| note | 16x | 22x | 24x | 13x |
| paper | 33x | 31x | 57x | 12x |
| formulas | 15x | 18x | 23x | 13x |
| book | 18x | 19x | 58x | 11x |
| drawing | 32x | 37x | 40x | 39x |
| plots | 69x | 59x | 57x | 97x |
| tree | 123x | 130x | 116x | 216x |

## Peak memory

| Document | quicktex | pdflatex | xelatex | lualatex | tectonic |
| --- | --- | --- | --- | --- | --- |
| note | 45 MB | 48 MB | 178 MB | 154 MB | 227 MB |
| paper | 56 MB | 49 MB | 181 MB | 164 MB | 230 MB |
| formulas | 50 MB | 48 MB | 178 MB | 148 MB | 226 MB |
| book | 153 MB | 49 MB | 181 MB | 181 MB | 242 MB |
| drawing | 44 MB | 51 MB | 182 MB | 156 MB | 231 MB |
| plots | 47 MB | 55 MB | 186 MB | 171 MB | 232 MB |
| tree | 73 MB | 44 MB | 196 MB | 187 MB | 247 MB |

## PDF size

quicktex loses here. It draws every letter as a shape, while TeX embeds each font once.

| Document | quicktex | pdflatex | xelatex | lualatex | tectonic |
| --- | --- | --- | --- | --- | --- |
| note | 67 KB | 19 KB | 7 KB | 7 KB | 7 KB |
| paper | 729 KB | 169 KB | 60 KB | 125 KB | 61 KB |
| formulas | 397 KB | 125 KB | 60 KB | 140 KB | 64 KB |
| book | 11.4 MB | 398 KB | 343 KB | 431 KB | 345 KB |
| drawing | 71 KB | 24 KB | 18 KB | 18 KB | 18 KB |
| plots | 44 KB | 38 KB | 17 KB | 37 KB | 17 KB |
| tree | 230 KB | 51 KB | 50 KB | 51 KB | 50 KB |

## Page counts

Layout is not identical, so page counts can differ. See [the README](README.md) for what quicktex does not do.

| Document | quicktex | pdflatex | xelatex | lualatex | tectonic |
| --- | --- | --- | --- | --- | --- |
| note | 1 | 1 | 1 | 1 | 1 |
| paper | 8 | 8 | 8 | 8 | 8 |
| formulas | 31 | 22 | 22 | 22 | 22 |
| book | 106 | 107 | 107 | 107 | 107 |
| drawing | 1 | 1 | 1 | 1 | 1 |
| plots | 2 | 2 | 2 | 2 | 2 |
| tree | 1 | 1 | 1 | 1 | 1 |

## quicktex already loaded

This is what an open editor tab pays per compile. No other compiler here has a mode like this, so there is no column to compare it with.

| Document | Already loaded | Fresh process |
| --- | --- | --- |
| note | 0.1 ms | 17 ms |
| paper | 1.8 ms | 23 ms |
| formulas | 0.8 ms | 21 ms |
| book | 49 ms | 80 ms |
| drawing | 0.3 ms | 19 ms |
| plots | 0.2 ms | 19 ms |
| tree | 12 ms | 48 ms |

## One formula

Mean time per formula over the 222 formulas that every library accepted, after warming up, in Node. Output formats differ, and none of this includes the browser drawing the result.

| Library | Per formula | vs quicktex | Output | Mean output | Version |
| --- | --- | --- | --- | --- | --- |
| quicktex | 0.8 µs | 1.0x | SVG | 1.3 KB | this repo |
| temml | 9.3 µs | 11x | MathML | 0.2 KB | 0.14.0 |
| katex | 21.1 µs | 25x | HTML and MathML | 1.2 KB | 0.19.1 |
| mathjax | 161.5 µs | 193x | SVG | 2.1 KB | 3.2.2 |

## Versions

- quicktex: 9f0ce88
- pdflatex: pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026)
- xelatex: XeTeX 3.141592653-2.6-0.999998 (TeX Live 2026)
- lualatex: This is LuaHBTeX, Version 1.24.0 (TeX Live 2026)
- tectonic: Tectonic 0.17.0
