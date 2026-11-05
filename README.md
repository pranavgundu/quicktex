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

`/editor/` is a separate page for whole documents. Write on the left, read the pages on the right, and download a PDF. It runs entirely in the browser and keeps your document in local storage.

The engine runs in a Web Worker, so typing never waits for a compile. Text typed during a compile collapses to the newest version, so a slow document only delays the pages, not the keys.

Zoom the pages from 25% to 500% by pinching, by scrolling with Ctrl or Cmd held, with the buttons, or by double clicking. With the pages focused, `+`, `-` and `0` zoom too. Drag to pan.

```js
import { init, renderDocument, documentPage, documentPdf } from './wasm-core/js/quicktex.js'

await init()
const pages = renderDocument(source)
const svg = documentPage(0)
const pdf = documentPdf()
```

It handles `article`, `report`, `book`, `beamer` and `standalone` style documents: titles, abstracts, numbered sections, a table of contents, paragraphs, inline and displayed math, numbered equations, lists, quotes, centered text, theorems, verbatim blocks, footnotes, `\ref`, `\cite`, `\newenvironment`, one or two columns, page margins from `geometry`, `parskip`, tables with `booktabs` rules, `tabularx` and `tabular*` at full width, numbered figures, tables and algorithms with captions, `algorithmic` pseudocode, frames with overlays, accented letters, and drawings.

Drawings use a subset of TikZ and pgfplots: lines, curves, rectangles, circles, ellipses, arcs, arrow tips, nodes, named coordinates, `calc` points, scopes with scale, shift and rotate, styles, clipping, grids, `\foreach`, `\pgfmathsetmacro`, your own recursive macros with `\ifnum`, and `axis` plots with coordinates, functions, grids, legends and automatic ticks.

Unknown commands, options and packages never stop a document. They are skipped, and the status bar names the first one with a count of the rest. Only real structural mistakes stop it: unbalanced braces, unclosed math, and mismatched `\begin` and `\end`.

Documents now support:

- Virtual project files for `\input`, `\include`, and PNG/JPEG `\includegraphics`, with aspect-ratio sizing and PNG transparency. Use **Add files** in the editor; importing `main.tex` or `document.tex` also opens it as the main source. Imported files live in the current tab and must be added again after a reload.
- Footnotes at the bottom of their column, with reserved space and a separator rule.
- Word wrapping in `p{}`, `m{}`, `b{}`, and `X` table columns, including shrinking `X` columns to the requested table width.
- Chinese, Japanese, Korean, Cyrillic, and Greek fallback outlines. `loadFontsFor(source)` loads the optional Noto assets; the editor and CLI do this automatically. Only glyphs used in the document enter its PDF. CJK uses Noto Serif CJK SC's shared ideograph forms; language-specific shaping and bold/italic fallback faces are not implemented.
- American English pattern hyphenation, `\-`, and `\hyphenation{...}` exceptions. This improves line breaking but does not reproduce TeX's full paragraph or language machinery.
- `[h]`/`[H]` placement, deferred `[t]` floats, `[b]` floats at the bottom, and wide floats on the next page. Float queues use simpler rules than TeX; float-page fractions and all placement combinations are not implemented.
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

Implemented: `amsmath`, `mathtools`, `physics`, `braket`, `cancel`, `mhchem`, `chemformula`, `siunitx`, `nicefrac`, `xfrac`, `gensymb`, `commath`, `derivative`, `diffcoeff`, `units`, `dsfont`, `bbm`, `bbold`, `centernot`, `slashed`, `faktor`, `interval`, `stmaryrd`.

Preamble packages such as `hyperref` and `inputenc` are accepted and ignored. In full documents, `geometry`, `parskip`, `tikz` and `pgfplots` take effect, and any other package is ignored. In a single formula, an unrecognized package is an `unknown package` error.

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
