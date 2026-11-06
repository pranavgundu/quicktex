import { afterEach, describe, expect, test } from 'bun:test'
import { readFileSync, mkdtempSync, writeFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { spawnSync } from 'node:child_process'
import { documentPage, documentPdf, documentWarnings, loadFontsFor, renderDocument, setFiles } from '../wasm-core/js/quicktex.js'
import { article } from './support/engine'

const png = readFileSync(new URL('./fixtures/image.png', import.meta.url))
afterEach(() => setFiles({}))

function extracted(bytes: Uint8Array) {
  const directory = mkdtempSync(join(tmpdir(), 'quicktex-text-'))
  try {
    const file = join(directory, 'document.pdf')
    writeFileSync(file, bytes)
    const output = spawnSync('pdftotext', ['-layout', file, '-'], { encoding: 'utf8' })
    if (output.error) throw output.error
    expect(output.status).toBe(0)
    return output.stdout
  } finally { rmSync(directory, { recursive: true, force: true }) }
}

describe('project files and PDF text', () => {
  test('publisher bibliography families render searchable data with approximation warnings', () => {
    for (const style of ['IEEEtran', 'IEEEtranN-doi', 'splncs04', 'splncs04nat', 'spbasic', 'ACM-Reference-Format']) {
      renderDocument(article(String.raw`\cite{x}\bibliographystyle{${style}}\bibliography{refs}`), { files: {
        'refs.bib': '@article{x,author={Doe, Jane},title={Searchable paper},journal={Example Journal},year=2024,doi={10.1/test}}',
      } })
      const text = extracted(documentPdf())
      for (const value of ['Doe', 'Searchable paper', 'Example Journal', '2024', '10.1/test']) expect(text).toContain(value)
      expect(documentWarnings().length).toBeGreaterThan(0)
    }
  })

  test('surface contour integrals accept explicit limits and emit valid SVG and PDF', () => {
    renderDocument(article(String.raw`\[\oiint\limits_{S}\vec E\cdot\mathrm{d}\vec S=\frac{Q}{\varepsilon_0}\]`))
    expect(documentWarnings()).toEqual([])
    expect(documentPage(0)).toContain('c')
    expect(extracted(documentPdf())).toContain('S')
  })

  test('unused missing bibliography databases warn, but referenced ones fail', () => {
    renderDocument(article(String.raw`Text\bibliography{missing}`))
    expect(documentWarnings().length).toBeGreaterThan(0)
    const text = extracted(documentPdf())
    expect(text).toContain('Text')
    expect(text).not.toContain('References')
    expect(() => renderDocument(article(String.raw`\cite{x}\bibliography{missing}`))).toThrow()
  })
  test('nested inputs share definitions and appear in the PDF', () => {
    renderDocument(article(String.raw`Before. \input{parts/one} \greeting{} After.`), { files: {
      'parts/one.tex': String.raw`\newcommand{\greeting}{Hello} First included paragraph.\input{parts/two}`,
      'parts/two.tex': 'Second included paragraph.',
    } })
    expect(documentWarnings()).toEqual([])
    const text = extracted(documentPdf())
    expect(text).toContain('First included paragraph.')
    expect(text).toContain('Second included paragraph.')
    expect(text).toContain('Hello After.')
  })

  test('classic crossrefs inherit fields and promote a shared parent in the PDF', () => {
    renderDocument(article(String.raw`\cite{a,b}\bibliographystyle{unsrt}\bibliography{refs}`), { files: {
      'refs.bib': String.raw`@inproceedings{a,title={First paper},crossref={parent}}
        @inproceedings{b,title={Second paper},crossref={parent}}
        @proceedings{parent,title={Collected papers},publisher={Example Press},year=2024}`,
    } })
    expect(documentWarnings()).toEqual([])
    const text = extracted(documentPdf())
    expect(text).toContain('First paper')
    expect(text).toContain('Second paper')
    expect(text).toContain('Collected papers')
    expect(text.match(/Example Press/g)?.length).toBe(3)
    expect(text.match(/2024/g)?.length).toBe(3)
  })

  test('footnote counter assignments control subsequent marks', () => {
    renderDocument(article(String.raw`\setcounter{footnote}{2}Text\footnote{Third note.}
      \addtocounter{footnote}{4}More\footnote{Eighth note.} Counter: \arabic{footnote}.`))
    expect(documentWarnings()).toEqual([])
    const text = extracted(documentPdf())
    expect(text).toMatch(/3\s+Third note/)
    expect(text).toMatch(/8\s+Eighth note/)
    expect(text).toContain('Counter: 8.')
    renderDocument(article(String.raw`Reset\footnote{First note.}`))
    expect(extracted(documentPdf())).toMatch(/1\s+First note/)
  })

  test('BibLaTeX URL penalties wrap links without losing searchable characters', () => {
    const url = 'https://example.org/' + 'abcdefghij'.repeat(8)
    renderDocument(String.raw`\documentclass{article}
      \usepackage[left=250pt,right=250pt]{geometry}\usepackage{biblatex}
      \setlength{\parindent}{0pt}\setcounter{biburllcpenalty}{7000}
      \begin{document}\biburlsetup\url{${url}}\end{document}`)
    const text = extracted(documentPdf()).replace(/\s/g, '')
    expect(text).toContain(url)
    const pdf = new TextDecoder().decode(documentPdf())
    expect(pdf.match(/\/Subtype \/Link/g)!.length).toBeGreaterThan(2)
    // The package's unimplemented bibliography styles/options remain visible.
    expect(documentWarnings().length).toBeGreaterThan(0)
  })

  test('generated bibliographies activate URL wrapping locally', () => {
    const url = 'https://example.org/' + 'abcdefghij'.repeat(8)
    renderDocument(String.raw`\documentclass{article}
      \usepackage[left=250pt,right=250pt]{geometry}\usepackage{biblatex}
      \setcounter{biburllcpenalty}{7000}\addbibresource{refs.bib}
      \begin{document}\nocite{a}\printbibliography\end{document}`, { files: {
      'refs.bib': `@online{a,title={Example},url={${url}}}`,
    } })
    const text = extracted(documentPdf())
    expect(text).not.toContain(url)
    expect(text.replace(/\s/g, '')).toContain(url)
  })

  test('include starts and ends a page', () => {
    expect(renderDocument(article(String.raw`Before.\include{chapter}After.`), { files: { 'chapter.tex': 'Chapter text.' } })).toBe(3)
    expect(extracted(documentPdf())).toContain('Chapter text.')
  })

  test('recursive inputs stop with a structural error', () => {
    expect(() => renderDocument(article(String.raw`\input{loop}`), { files: { 'loop.tex': String.raw`\input{loop}` } })).toThrow()
  })

  test('PNG pixels and alpha reach both outputs with the requested aspect ratio', () => {
    renderDocument(article(String.raw`\includegraphics[width=2cm]{image}`), { files: { 'image.png': png } })
    expect(documentWarnings()).toEqual([])
    const svg = documentPage(0)
    expect(svg).toContain('<image ')
    expect(svg).toContain('data:image/png;base64,')
    const size = /<image[^>]*width="(\d+)" height="(\d+)"/.exec(svg)!
    expect(Number(size[1]) / Number(size[2])).toBeCloseTo(2, 2)
    const pdf = new TextDecoder().decode(documentPdf())
    expect(pdf).toContain('/Subtype /Image')
    expect(pdf).toContain('/SMask')
  })

  test('graphic scale uses the image intrinsic dimensions', () => {
    renderDocument(article(String.raw`\includegraphics{image}`), { files: { 'image.png': png } })
    const width = Number(/<image[^>]*width="(\d+)"/.exec(documentPage(0))![1])
    renderDocument(article(String.raw`\includegraphics[scale=0.5]{image}`))
    expect(Number(/<image[^>]*width="(\d+)"/.exec(documentPage(0))![1])).toBe(width / 2)
  })

  test('missing images warn instead of silently claiming success', () => {
    renderDocument(article(String.raw`\includegraphics{missing}`))
    expect(documentWarnings().length).toBeGreaterThan(0)
  })

  test('ordinary text can be selected and searched', () => {
    renderDocument(article('Searchable text with spaces and numbers 123.'))
    expect(extracted(documentPdf())).toContain('Searchable text with spaces and numbers 123.')
  })

  test('Unicode outlines are embedded and extract to their original scripts', async () => {
    const text = 'Ελληνικά Русский 中文 日本語 한국어'
    await loadFontsFor(text, readFileSync)
    renderDocument(article(text))
    expect(documentWarnings()).toEqual([])
    expect(documentPage(0)).not.toContain('?')
    const output = extracted(documentPdf())
    for (const word of text.split(' ')) expect(output).toContain(word)
  })

  test('image caches survive edits, keep sizing local and invalidate on replacement', () => {
    setFiles({ 'image.png': png })
    renderDocument(article(String.raw`First \includegraphics[width=1cm]{image}`))
    const before = Number(/<image[^>]*width="(\d+)"/.exec(documentPage(0))![1])
    renderDocument(article(String.raw`Edited \includegraphics[width=2cm]{image}`))
    expect(documentWarnings()).toEqual([])
    expect(Number(/<image[^>]*width="(\d+)"/.exec(documentPage(0))![1])).toBe(before * 2)
    renderDocument(article(String.raw`Replaced \includegraphics{image}`), { files: { 'image.png': new Uint8Array([1, 2, 3]) } })
    expect(documentWarnings().length).toBeGreaterThan(0)
    expect(documentPage(0)).not.toContain('<image ')
    renderDocument(article(String.raw`Restored \includegraphics{image}`), { files: { 'image.png': png } })
    expect(documentWarnings()).toEqual([])
    expect(documentPage(0)).toContain('<image ')
  })

  test('project Unicode glyphs refresh after font loading and project edits', async () => {
    const path = `../wasm-core/js/quicktex.js?project-glyph-cache=${crypto.randomUUID()}`
    const fresh: typeof import('../wasm-core/js/quicktex.js') = await import(path)
    await fresh.init(readFileSync(new URL('../wasm-core/js/quicktex.wasm', import.meta.url)))
    fresh.setFiles({ 'chapter.tex': '中文' })
    fresh.renderDocument(article(String.raw`Before fonts \input{chapter}`))
    expect(fresh.documentWarnings().length).toBeGreaterThan(0)
    await fresh.loadFontsFor('中文 Ελληνικά', readFileSync)
    for (const edit of ['First', 'Edited']) {
      fresh.renderDocument(article(String.raw`${edit} \input{chapter}`))
      expect(extracted(fresh.documentPdf())).toContain('中文')
      expect(fresh.documentWarnings()).toEqual([])
    }
    fresh.setFiles({ 'chapter.tex': 'Ελληνικά' })
    fresh.renderDocument(article(String.raw`Changed \input{chapter}`))
    expect(extracted(fresh.documentPdf())).toContain('Ελληνικά')
    expect(fresh.documentWarnings()).toEqual([])
  })

  test('a failed compile preserves the previous image PDF', () => {
    renderDocument(article(String.raw`\includegraphics{image}`), { files: { 'image.png': png } })
    const before = documentPdf()
    expect(() => renderDocument(String.raw`\begin{document}{`, { files: {} })).toThrow()
    expect(documentPdf()).toEqual(before)
  })
})

const bibliography = String.raw`@string{press = "A Press"}
@article{doe, author={Jane Doe and John Roe}, title={A {Nested} Title}, journal={A Journal}, year=2024}
@book{smith, author={Alice Smith}, title={The Book}, publisher=press, year=2023}`

describe('bibliography project assets', () => {
  test('numeric natbib citations and a single references heading reach the PDF', () => {
    renderDocument(String.raw`\documentclass{article}\usepackage[numbers]{natbib}\begin{document}
\citet{doe}. \citep[see][p.~4]{smith,doe}. \citeauthor*{doe}; \citeyear{smith}.
\bibliographystyle{plainnat}\bibliography{refs}\end{document}`, { files: { 'refs.bib': bibliography } })
    expect(documentWarnings()).toEqual([])
    const text = extracted(documentPdf())
    expect(text).toContain('Doe and Roe [1]')
    expect(text).toContain('[see 2, 1, p. 4]')
    expect(text).toContain('Doe and Roe; 2023')
    expect(text).toContain('A Nested Title')
    expect(text).toContain('A Press')
    expect(text.match(/References/g)).toHaveLength(1)
  })

  test('author-year natbib citations support notes and bare forms', () => {
    renderDocument(String.raw`\documentclass{article}\usepackage{natbib}\begin{document}
\citet{doe}; \citep[see][p.~4]{smith}; \citealp{doe}; \citeyearpar{smith}.
\bibliographystyle{plainnat}\bibliography{refs}\end{document}`, { files: { 'refs.bib': bibliography } })
    expect(documentWarnings()).toEqual([])
    const text = extracted(documentPdf())
    expect(text).toContain('Doe and Roe (2024)')
    expect(text).toContain('(see Smith, 2023, p. 4)')
    expect(text).toContain('Doe and Roe, 2024')
    expect(text).toContain('(2023)')
  })

  test('nested bibliography resources and nocite include uncited entries', () => {
    renderDocument(article(String.raw`\input{chapter}`), { files: {
      'chapter.tex': String.raw`\cite{doe}\nocite{*}\bibliographystyle{unsrt}\bibliography{data/refs}`,
      'data/refs.bib': bibliography,
    } })
    expect(documentWarnings()).toEqual([])
    const text = extracted(documentPdf())
    expect(text).toContain('A Nested Title')
    expect(text).toContain('The Book')
  })

  test('missing resources and unsupported styles identify the cause', () => {
    expect(() => renderDocument(article(String.raw`\cite{needed}\bibliography{missing}`))).toThrow("bibliography file 'missing' was not found")
    expect(() => renderDocument(article(String.raw`\bibliographystyle{custom}`))).toThrow("unsupported bibliography style 'custom'")
    expect(() => renderDocument(article(String.raw`\bibliography{bad}`), { files: { 'bad.bib': '@article{broken, title={unclosed}' } })).toThrow("invalid bibliography 'bad.bib'")
  })

  test('missing citation keys produce a visible placeholder and warning', () => {
    renderDocument(article(String.raw`\cite{absent,doe}\bibliography{refs}`), { files: { 'refs.bib': bibliography } })
    expect(documentWarnings().length).toBeGreaterThan(0)
    expect(extracted(documentPdf())).toContain('[??, 1]')
  })
})

describe('virtual filesystem path aliases', () => {
  test('inputs and image files resolve dot segments', () => {
    renderDocument(article(String.raw`\input{./parts/../chapter}\includegraphics{img/../figure}`), { files: {
      'chapter.tex': 'An included paragraph.', 'figure.png': png,
    } })
    expect(documentWarnings()).toEqual([])
    expect(extracted(documentPdf())).toContain('An included paragraph.')
    expect(documentPage(0)).toContain('<image ')
  })

  test('parent-relative names remain relative to the main directory', () => {
    renderDocument(article(String.raw`\input{../shared/./chapter}`), { files: {
      '../shared/chapter.tex': 'Shared parent content.',
    } })
    expect(documentWarnings()).toEqual([])
    expect(extracted(documentPdf())).toContain('Shared parent content.')
  })

  test('aliases cannot bypass recursive input detection', () => {
    expect(() => renderDocument(article(String.raw`\input{parts/../loop}`), { files: {
      'loop.tex': String.raw`\input{./other/../loop}`,
    } })).toThrow()
  })
})

describe('graphics search paths and extension selection', () => {
  test('graphicspath searches declared directories in project files', () => {
    renderDocument(String.raw`\documentclass{article}\graphicspath{ {missing/}{figures/../photos/} }
\begin{document}\includegraphics[width=2cm]{plot}\end{document}`, { files: { 'photos/plot.png': png } })
    expect(documentWarnings()).toEqual([])
    expect(documentPage(0)).toContain('<image ')
  })

  test('extensions are tried across all directories before the next extension', () => {
    renderDocument(article('A control PDF image.'))
    const pdf = documentPdf()
    const files = { 'plot.png': png, 'figures/plot.pdf': pdf }
    const source = (extensions = '') => String.raw`\documentclass{article}\graphicspath{{figures/}}` + extensions + String.raw`\begin{document}\includegraphics{plot}\end{document}`
    renderDocument(source(), { files })
    // PDF inclusion is still unsupported: do not silently substitute the PNG
    // when graphics.sty's default order chooses the PDF in figures/ first.
    expect(documentWarnings().length).toBeGreaterThan(0)
    expect(documentPage(0)).not.toContain('<image ')
    renderDocument(source(String.raw`\DeclareGraphicsExtensions{ .png, .pdf }`), { files })
    expect(documentWarnings()).toEqual([])
    expect(documentPage(0)).toContain('<image ')
  })

  test('search-path changes respect text, block and environment scopes', () => {
    const files = { 'inner/inside.png': png, 'outer/restored.png': png }
    for (const [open, close] of [['{', '}'], [String.raw`\textbf{`, '}'], [String.raw`\begin{center}`, String.raw`\end{center}`], ['{', String.raw`\begin{center}\end{center}}`]]) {
      renderDocument(String.raw`\documentclass{article}\graphicspath{{outer/}}\begin{document}` + open + String.raw`\graphicspath{{inner/}}\includegraphics[width=1cm]{inside}` + close + String.raw`\includegraphics[width=1cm]{restored}\end{document}`, { files })
      expect(documentWarnings()).toEqual([])
      expect(documentPage(0).match(/<image /g)?.length).toBe(2)
    }
  })

  test('search paths reset between renders while uploaded assets stay available', () => {
    renderDocument(String.raw`\documentclass{article}\graphicspath{{inner/}}\begin{document}\includegraphics{plot}\end{document}`, { files: { 'inner/plot.png': png } })
    expect(documentWarnings()).toEqual([])
    renderDocument(article(String.raw`\includegraphics{plot}`))
    expect(documentWarnings().length).toBeGreaterThan(0)
  })
})

test('graphics declarations ignore comments, including commented braces', () => {
  renderDocument(String.raw`\documentclass{article}
\graphicspath{%
{missing/}% } this brace is a comment
{figures/}%
}
\DeclareGraphicsExtensions{.png,% ] } these delimiters are comments
.jpg}
\begin{document}\includegraphics{plot}\end{document}`, { files: { 'figures/plot.png': png } })
  expect(documentWarnings()).toEqual([])
  expect(documentPage(0)).toContain('<image ')
})

test('graphics settings inside a title do not leak into the document', () => {
  renderDocument(String.raw`\documentclass{article}\graphicspath{{outer/}}
\title{\graphicspath{{inner/}}\includegraphics[width=1cm]{inside}}\date{}
\begin{document}\maketitle\includegraphics[width=1cm]{restored}\end{document}`, { files: {
    'inner/inside.png': png, 'outer/restored.png': png,
  } })
  expect(documentWarnings()).toEqual([])
  expect(documentPage(0).match(/<image /g)?.length).toBe(2)
})
