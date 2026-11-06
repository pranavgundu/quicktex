# quicktex

A LaTeX math renderer for the web. The parser, layout engine and SVG writer are Rust compiled to a single WebAssembly file, with no fonts to load for math. Document text can load bundled Unicode fallback fonts when needed.

## Run it

```sh
bun install
bun run dev
```

| Command | What it does |
| --- | --- |
| `bun run dev` | Starts the demo page |
| `bun run build` | Builds the demo page into `dist` |
| `bun run check` | Type-checks the UI |
| `bun run wasm` | Rebuilds `wasm-core/js/quicktex.wasm` (needs Rust and the `wasm32-unknown-unknown` target) |

## Deploy

`.github/workflows/build.yml` tests and builds every pull request and every push to `main`. `.github/workflows/deploy.yml` runs after a passing build of `main` and publishes that build to GitHub Pages. The build uses relative paths, so it works at any Pages URL or custom domain.

To turn it on, set **Settings > Pages > Source** to **GitHub Actions**.

## Use it

```js
import { init, render, usePackage } from './wasm-core/js/quicktex.js'

await init()

const svg = render(String.raw`\frac{a}{b}`, true)

usePackage('physics')
render(String.raw`\dv{f}{x}`)
```

`render(tex, display)` returns an SVG string and throws `QuicktexError` on invalid input. The error has a `position` into the source.

## Full editor

`/editor/` is a local-first workspace for whole documents, with the same green palette, serif controls and instant preview as the formula page. Write on the left, read the pages on the right, and download a PDF or the current source file. The root document is `main.tex`. Source is saved in local storage; supporting files and binary assets are saved asynchronously in IndexedDB. There is no account, cloud sync or collaboration; download backups of important work.

- **Project sidebar:** import files, create text files (including folder paths), filter, rename and delete. Open TeX/BibTeX files in tabs with independent undo histories and selections. Clicking an imported PNG/JPEG inserts its `\\includegraphics` command.
- **Outline:** sections in the current file update after a short idle delay and jump directly to source.
- **Editing tools:** undo/redo, bold/italic, comments, find/replace (regex, case and whole-word matching), and insert menus for sections, equations, lists, tables, figures, references and citations. Snippet fields are editable with Tab.
- **Command palette:** Cmd/Ctrl + Shift + P; **go to line:** Cmd/Ctrl + G; **save locally:** Cmd/Ctrl + S; **compile:** Cmd/Ctrl + Enter.
- **Settings:** font size, line wrapping and line numbers persist across reloads. Collapse the sidebar or choose editor-only, preview-only or split layouts. Phones show one pane at a time.
- **Preview:** automatic compilation can be switched off; page navigation, zoom, pan and PDF downloads stay available. Diagnostics jump to the root source. QuickTeX's existing LaTeX compatibility limits still apply.

The engine runs in a Web Worker, so typing never waits for a compile. Text typed during a compile collapses to the newest version, so a slow document only delays the pages, not the keys.

Zoom the pages from 25% to 500% by pinching, by scrolling with Ctrl or Cmd held, with the buttons, or by double clicking. With the pages focused, `+`, `-` and `0` zoom too. Drag to pan.

```js
import { init, renderDocument, documentPage, documentPdf } from './wasm-core/js/quicktex.js'

await init()
const pages = renderDocument(source)
const svg = documentPage(0)
const pdf = documentPdf()
```

It handles a subset of `article`, `report`, `book` and `standalone` style documents: titles, abstracts, numbered sections, a table of contents, paragraphs, inline and displayed math, numbered equations, lists, quotes, centered text, theorems, verbatim blocks, footnotes, `\ref`, `\cite`, `\newenvironment`, one or two columns, page margins from `geometry`, `parskip`, tables with `booktabs` rules, `tabularx` and `tabular*` at full width, numbered figures, tables and algorithms with captions, `algorithmic` pseudocode, frames with overlays, accented letters, and drawings.

Drawings use a subset of TikZ and pgfplots: lines, curves, rectangles, circles, ellipses, arcs, arrow tips, nodes, named coordinates, `calc` points, scopes with scale, shift and rotate, styles, clipping, grids, `\foreach`, `\pgfmathsetmacro`, your own recursive macros with `\ifnum`, and `axis` plots with coordinates, functions, grids, legends and automatic ticks.

Unsupported commands, package options and document classes produce warnings; partially implemented packages also warn when loading them could otherwise imply full support. The status bar points to the first warning and counts the rest. Structural mistakes, invalid counters, excessive loops, and missing or invalid bibliography files can stop compilation. A warning-free render is not proof of TeX compatibility; compare important output with a TeX compiler.

Documents now support:

- Virtual project files for `\input`, `\include`, `.bib` bibliographies, and PNG/JPEG `\includegraphics`, with aspect-ratio sizing and PNG transparency. Use **Add files** in the editor; importing `main.tex` or `document.tex` also opens it as the main source. Supporting files and assets persist in this browser using IndexedDB; the save indicator reports storage failures.
- Project paths resolve `.` and `..` consistently for inputs, images and bibliography resources. Literal `\graphicspath` directory lists and `\DeclareGraphicsExtensions` control image lookup, including extension order across directories; graphics settings stay local to groups, environments and titles. If that order selects a PDF figure, QuickTeX warns rather than silently substituting a PNG.
- `.bib` references from `\bibliography` with numeric and common natbib author/year citations, citation notes and `\nocite`. The built-in formatter accepts `plain`, `unsrt`, `abbrv` and their `nat` variants. It also provides partial IEEE (`IEEEtran`, `IEEEtranN`, `IEEEtranN-doi`), Springer/LNCS (`splncs04`, `splncs04nat`, `spbasic`) and ACM (`ACM-Reference-Format`) formatting, with compatibility warnings: author names, ordering and common reference fields are implemented, not arbitrary `.bst` programs or publisher-specific options. A missing database with no citations warns and renders no bibliography; a missing cited database remains an error. BibLaTeX URL counters control punctuation, letter and digit breaks in bibliography URLs and with `\biburlsetup`; this does not implement BibLaTeX styles or Biber. Classic BibTeX crossrefs inherit missing fields and list a shared parent after two cited children. Nested crossrefs and BibLaTeX inheritance remain unsupported. Macro-generated citations and conditional resource discovery are limited and produce warnings.
- Multi-page `longtable` output. Oversized `tabular` tables also preserve their rows across pages and warn about this fallback. Repeated longtable heads/feet and rows taller than a page warn.
- Custom and built-in counters (including footnote numbering), `\arabic`, alphabetic/Roman forms, count registers, `\advance` and bounded `\loop ... \ifnum ... \repeat`. This remains a subset of TeX programming; loops are limited to 1,000 iterations per render and custom `\refstepcounter` label binding warns.
- `fancyhdr` header/footer fields, odd/even selectors, `\pagestyle`, `\thispagestyle`, page numbers and header/footer rules. Automatic chapter/section marks and arbitrary page-style definitions remain unsupported.
- Clickable external PDF links from `\href` and `\url`, with separate annotations for wrapped lines/pages. Internal hyperref destinations and bookmarks remain unsupported and warn.
- Footnotes at the bottom of their column, with reserved space and a separator rule.
- Word wrapping in `p{}`, `m{}`, `b{}`, and `X` table columns, including shrinking `X` columns to the requested table width.
- Chinese, Japanese, Korean, Cyrillic, and Greek fallback outlines. `loadFontsFor(source)` loads the optional Noto assets; the editor and CLI do this automatically. Only glyphs used in the document enter its PDF. CJK uses Noto Serif CJK SC's shared ideograph forms; language-specific shaping and bold/italic fallback faces are not implemented.
- Multiple surface contour integrals (`\oiint`, `\oiiint`) with shared vector contours and normal operator script/limit placement in SVG and PDF.
- American English pattern hyphenation, `\-`, and `\hyphenation{...}` exceptions. This improves line breaking but does not reproduce TeX's full paragraph or language machinery.
- `[h]`/`[H]` placement, deferred `[t]` floats, `[b]` floats at the bottom, and wide floats on the next page. `topnumber`, `bottomnumber`, `totalnumber`, and `dbltopnumber` constrain placement; `!` overrides the count limits and `[H]` does not consume them. Zero limits still allow end-of-document float-only pages to flush. Float queues use simpler rules than TeX; float-page fractions and all placement combinations are not implemented.
- Selectable, searchable PDF text through embedded Type 3 glyph fonts and Unicode maps, plus compressed streams. Math extraction follows glyph order rather than semantic equation structure.

Drawing compatibility is a subset, with warnings for unsupported commands: `picture` supports `\put`, lines, vectors, circles, box text, and quadratic Béziers; `pspicture` supports polylines, polygons, rectangles, circles, and text; `forest` supports basic bracketed trees; `chemfig` supports linear molecules and branches; and TikZ accepts common `tkz` point definitions, segments, polygons, circles, midpoint construction, and labels. Package-specific styles, chemical rings, advanced constructions, and layout algorithms remain incomplete. Existing TikZ/pgfplots support is unchanged.

```js
import { init, loadFontsFor, renderDocument } from './wasm-core/js/quicktex.js'

await init()
await loadFontsFor(source + chapterSource) // optional for Latin documents
renderDocument(source, {
  files: {
    'chapter.tex': chapterSource,
    'plot.png': pngBytes, // Uint8Array
  },
})
```

The compiler reuses bounded caches for immutable ASCII metrics and decoded
project images, plus paragraph-layout scratch buffers. Every document edit still
runs layout; there is no whole-document result cache. Image caches clear when
project files change, and fallback-glyph preparation refreshes when files or
fonts are registered. See [edit benchmarks](bench/README.md#source-changing-edit-performance-and-caches)
for measured speedups and correctness checks.

`setFiles(files)` replaces the virtual project filesystem for subsequent compiles. Missing or unsupported image files produce warnings and a placeholder. Supported raster formats are 8-bit, non-interlaced PNG (gray, RGB, palette, and alpha) and 8-bit RGB JPEG. Other formats, including PDF figures, are not implemented. `await registerFont(bytes)` adds an OpenType fallback font. The optional bundled fonts are loaded lazily from the same origin; no external font service is used.

### Writing

The editor completes commands, environments, drawing options, colors, package names and your own `\newcommand` names as you type, with a live preview of math. Enter closes an environment and continues a list, Cmd or Ctrl with B or I toggles bold or italic, brackets and `$` pair up, sections and environments fold, and the outline jumps to any heading.

## Tests

```sh
bun run test:rust
bun run wasm
bun run build
bun run test
```

`test:rust` runs the engine tests natively. `test` runs the real wasm binary, every editor suggestion through the engine, recorded output hashes, speed budgets, and the production build in Chrome. After an intended output change, rerun with `UPDATE_GOLDEN=1 bun test tests/golden.test.ts` and review the diff.

## Benchmarks

`bench` times quicktex against `pdflatex`, `xelatex`, `lualatex`, Tectonic, KaTeX, MathJax and Temml, with no editor involved. See [bench/README.md](bench/README.md) and [bench/results.md](bench/results.md).

## Packages

Load a package with `\usepackage{...}` in the input, which lasts for that render, or with `usePackage(name)`, which lasts until the page closes.

Package subsets: `amsmath`, `mathtools`, `physics`, `braket`, `cancel`, `mhchem`, `chemformula`, `siunitx`, `nicefrac`, `xfrac`, `gensymb`, `commath`, `derivative`, `diffcoeff`, `units`, `dsfont`, `bbm`, `bbold`, `centernot`, `slashed`, `faktor`, `interval`, `stmaryrd`.

Full documents implement selected features from `geometry`, `parskip`, `tikz`, `pgfplots`, `fancyhdr` and `natbib`. Geometry currently supports margin options; other geometry options warn. Unknown packages and packages whose fonts or layout are not implemented warn. Package acceptance does not mean that all its commands, options or TeX macros work. In a single formula, an unrecognized package is an `unknown package` error.

Your own commands work too: `\newcommand`, `\renewcommand`, `\providecommand`, `\def`, `\NewDocumentCommand`, `\DeclareMathOperator`, `\DeclarePairedDelimiter` and `\definecolor`.

## Layout

| Path | Contents |
| --- | --- |
| `wasm-core/src/tex.rs` | Parser and layout |
| `wasm-core/src/tex/macros.rs` | Macros and package loading |
| `wasm-core/src/packages.rs` | Package definitions |
| `wasm-core/src/tex/document.rs` | Paragraphs, sections, lists and pages |
| `wasm-core/src/tex/document/commands.rs` | Text commands, references, footnotes, contents and algorithms |
| `wasm-core/src/tex/document/control.rs` | Warnings, conditionals and skipping what is not supported |
| `wasm-core/src/tex/pdf.rs` | PDF output |
| `wasm-core/src/emit.rs` | SVG output |
| `wasm-core/src/data.rs`, `*.bin` | Glyph outlines and metrics |
| `wasm-core/js` | JavaScript API, built wasm, and optional Unicode font assets |
| `wasm-fonts` | Optional OpenType outline parser, loaded only for Unicode fallback |
| `wasm-core/assets/build.py` | Regenerates the English trie and independent compressed glyph blocks |
| `src` | Formula page |
| `src/editor`, `editor` | Full editor page |
| `src/editor/engine.ts`, `src/editor/engine.worker.ts` | Compiling off the main thread |
