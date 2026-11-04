# quicktex

A LaTeX math renderer for the web. The parser, layout engine and SVG writer are Rust compiled to a single WebAssembly file, with no runtime dependencies and no fonts to load.

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

```js
import { init, renderDocument, documentPage, documentPdf } from './wasm-core/js/quicktex.js'

await init()
const pages = renderDocument(source)
const svg = documentPage(0)
const pdf = documentPdf()
```

It handles `article` style documents: titles, abstracts, numbered sections, paragraphs, inline and displayed math, numbered equations, lists, quotes, centered text, one or two columns, page margins from `geometry`, `parskip`, tables with `booktabs` rules, numbered figures and tables with captions, and drawings.

Drawings use a subset of TikZ and pgfplots: lines, curves, rectangles, circles, ellipses, arcs, arrow tips, nodes, named coordinates, `calc` points, scopes with scale, shift and rotate, styles, and `axis` plots with coordinates, functions, grids, legends and automatic ticks. An option outside that subset is an `unsupported drawing option` error on its line.

It does not yet handle `\ref`, `\cite`, footnotes, images, `\foreach`, `\multicolumn`, `\textcolor` in body text, accented letters, row numbers in `align`, a table of contents, or hyphenation, so line breaks differ from `pdflatex`. Floats stay where they are written, except wide `figure*` floats, which move to the top of the next page. Text in the PDF is drawn as shapes, so it cannot be selected or searched.

## Tests

```sh
bun run test:rust
bun run wasm
bun run build
bun run test
```

`test:rust` runs the engine tests natively. `test` runs the real wasm binary, recorded output hashes, speed budgets, and the production build in Chrome. After an intended output change, rerun with `UPDATE_GOLDEN=1 bun test tests/golden.test.ts` and review the diff.

## Packages

Load a package with `\usepackage{...}` in the input, which lasts for that render, or with `usePackage(name)`, which lasts until the page closes.

Implemented: `amsmath`, `mathtools`, `physics`, `braket`, `cancel`, `mhchem`, `chemformula`, `siunitx`, `nicefrac`, `xfrac`, `gensymb`, `commath`, `derivative`, `diffcoeff`, `units`, `dsfont`, `bbm`, `bbold`, `centernot`, `slashed`, `faktor`, `interval`, `stmaryrd`.

Preamble packages such as `hyperref` and `inputenc` are accepted and ignored. In full documents, `geometry`, `parskip`, `tikz` and `pgfplots` take effect. Anything else is an `unknown package` error.

Your own commands work too: `\newcommand`, `\renewcommand`, `\providecommand`, `\def`, `\NewDocumentCommand`, `\DeclareMathOperator`, `\DeclarePairedDelimiter` and `\definecolor`.

## Layout

| Path | Contents |
| --- | --- |
| `wasm-core/src/tex.rs` | Parser and layout |
| `wasm-core/src/tex/macros.rs` | Macros and package loading |
| `wasm-core/src/packages.rs` | Package definitions |
| `wasm-core/src/tex/document.rs` | Paragraphs, sections, lists and pages |
| `wasm-core/src/tex/pdf.rs` | PDF output |
| `wasm-core/src/emit.rs` | SVG output |
| `wasm-core/src/data.rs`, `*.bin` | Glyph outlines and metrics |
| `wasm-core/js` | JavaScript API and the built wasm |
| `src` | Formula page |
| `src/editor`, `editor` | Full editor page |
