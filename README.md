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

Not handled yet:

- `\includegraphics` draws a placeholder box, because a page has no files to read
- `\input` and `\include` are skipped, because there is one source file
- footnotes print under their paragraph, not at the bottom of the page
- table cells stay on one line, so long `p{}` and `X` cells run past the column
- there is no font for Chinese, Japanese, Korean, Cyrillic or Greek text, so those letters show as `?`
- `picture`, `pspicture`, `tkz-euclide`, `forest` and `chemfig` drawings are skipped
- there is no hyphenation, so line breaks differ from `pdflatex`
- floats stay where they are written, except wide `figure*` floats, which move to the top of the next page
- text in the PDF is drawn as shapes, so it cannot be selected or searched

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
| `wasm-core/js` | JavaScript API and the built wasm |
| `src` | Formula page |
| `src/editor`, `editor` | Full editor page |
