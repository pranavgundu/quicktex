# Results

Run on 2026-11-05. Apple M1, 8 cores, 16 GB, darwin arm64. Regenerate with `bun run bench`.

## Source to PDF on disk

Median wall time from a fresh process to a finished PDF. Lower is better. The last column is the speedup over whichever other compiler was fastest on that document.

| Document | quicktex | pdflatex | xelatex | lualatex | tectonic | vs fastest other |
| --- | --- | --- | --- | --- | --- | --- |
| note | 19 ms | 280 ms | 405 ms | 458 ms | 213 ms | 11x |
| paper | 27 ms | 766 ms | 742 ms | 1.4 s | 281 ms | 10x |
| formulas | 25 ms | 326 ms | 385 ms | 485 ms | 277 ms | 11x |
| book | 76 ms | 1.5 s | 1.5 s | 5.1 s | 927 ms | 12x |
| drawing | 19 ms | 621 ms | 730 ms | 751 ms | 751 ms | 32x |
| plots | 21 ms | 1.3 s | 1.1 s | 1.1 s | 1.9 s | 53x |
| tree | 46 ms | 5.7 s | 6.2 s | 5.6 s | 10.4 s | 120x |

## How many times faster quicktex is

| Document | pdflatex | xelatex | lualatex | tectonic |
| --- | --- | --- | --- | --- |
| note | 14x | 21x | 24x | 11x |
| paper | 28x | 28x | 50x | 10x |
| formulas | 13x | 15x | 19x | 11x |
| book | 19x | 20x | 66x | 12x |
| drawing | 32x | 38x | 39x | 39x |
| plots | 62x | 55x | 53x | 91x |
| tree | 122x | 134x | 120x | 224x |

## Peak memory

| Document | quicktex | pdflatex | xelatex | lualatex | tectonic |
| --- | --- | --- | --- | --- | --- |
| note | 47 MB | 49 MB | 179 MB | 155 MB | 227 MB |
| paper | 60 MB | 49 MB | 181 MB | 175 MB | 230 MB |
| formulas | 54 MB | 49 MB | 179 MB | 159 MB | 226 MB |
| book | 121 MB | 49 MB | 181 MB | 187 MB | 242 MB |
| drawing | 47 MB | 51 MB | 182 MB | 157 MB | 231 MB |
| plots | 50 MB | 55 MB | 186 MB | 173 MB | 232 MB |
| tree | 75 MB | 54 MB | 196 MB | 200 MB | 247 MB |

## PDF size

quicktex embeds reusable Type 3 glyph fonts with Unicode maps and compresses page streams. TeX uses different font formats; compare the resulting sizes below.

| Document | quicktex | pdflatex | xelatex | lualatex | tectonic |
| --- | --- | --- | --- | --- | --- |
| note | 40 KB | 19 KB | 7 KB | 7 KB | 7 KB |
| paper | 220 KB | 169 KB | 60 KB | 125 KB | 61 KB |
| formulas | 312 KB | 125 KB | 60 KB | 140 KB | 64 KB |
| book | 2.0 MB | 398 KB | 343 KB | 431 KB | 345 KB |
| drawing | 72 KB | 24 KB | 18 KB | 18 KB | 18 KB |
| plots | 44 KB | 38 KB | 17 KB | 37 KB | 17 KB |
| tree | 47 KB | 51 KB | 50 KB | 51 KB | 50 KB |

## Page counts

Layout is not identical, so page counts can differ. See [the README](README.md) for what quicktex does not do.

| Document | quicktex | pdflatex | xelatex | lualatex | tectonic |
| --- | --- | --- | --- | --- | --- |
| note | 1 | 1 | 1 | 1 | 1 |
| paper | 9 | 8 | 8 | 8 | 8 |
| formulas | 31 | 22 | 22 | 22 | 22 |
| book | 106 | 107 | 107 | 107 | 107 |
| drawing | 1 | 1 | 1 | 1 | 1 |
| plots | 2 | 2 | 2 | 2 | 2 |
| tree | 1 | 1 | 1 | 1 | 1 |

## quicktex already loaded

This is what an open editor tab pays per compile. No other compiler here has a mode like this, so there is no column to compare it with.

| Document | Already loaded | Fresh process |
| --- | --- | --- |
| note | 44 µs | 19 ms |
| paper | 1.4 ms | 27 ms |
| formulas | 692 µs | 25 ms |
| book | 36 ms | 76 ms |
| drawing | 302 µs | 19 ms |
| plots | 186 µs | 21 ms |
| tree | 14 ms | 46 ms |

## One formula

Mean time per formula over the 222 formulas that every library accepted, after warming up, in Node. Output formats differ, and none of this includes the browser drawing the result.

| Library | Per formula | vs quicktex | Output | Mean output | Version |
| --- | --- | --- | --- | --- | --- |
| quicktex | 1.0 µs | 1.0x | SVG | 1.3 KB | this repo |
| temml | 10.6 µs | 11x | MathML | 0.2 KB | 0.14.0 |
| katex | 23.6 µs | 24x | HTML and MathML | 1.2 KB | 0.19.1 |
| mathjax | 178.3 µs | 181x | SVG | 2.1 KB | 3.2.2 |

## Versions

- quicktex: b7062eb-dirty
- pdflatex: pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026)
- xelatex: XeTeX 3.141592653-2.6-0.999998 (TeX Live 2026)
- lualatex: This is LuaHBTeX, Version 1.24.0 (TeX Live 2026)
- tectonic: Tectonic 0.17.0
