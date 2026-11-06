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
  test('nested inputs share definitions and appear in the PDF', () => {
    renderDocument(article(String.raw`Before. \input{parts/one} \greeting After.`), { files: {
      'parts/one.tex': String.raw`\newcommand{\greeting}{Hello} First included paragraph.\input{parts/two}`,
      'parts/two.tex': 'Second included paragraph.',
    } })
    expect(documentWarnings()).toEqual([])
    const text = extracted(documentPdf())
    expect(text).toContain('First included paragraph.')
    expect(text).toContain('Second included paragraph.')
    expect(text).toContain('Hello After.')
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
    expect(() => renderDocument(article(String.raw`\bibliography{missing}`))).toThrow("bibliography file 'missing' was not found")
    expect(() => renderDocument(article(String.raw`\bibliographystyle{custom}`))).toThrow("unsupported bibliography style 'custom'")
    expect(() => renderDocument(article(String.raw`\bibliography{bad}`), { files: { 'bad.bib': '@article{broken, title={unclosed}' } })).toThrow("invalid bibliography 'bad.bib'")
  })

  test('missing citation keys produce a visible placeholder and warning', () => {
    renderDocument(article(String.raw`\cite{absent,doe}\bibliography{refs}`), { files: { 'refs.bib': bibliography } })
    expect(documentWarnings().length).toBeGreaterThan(0)
    expect(extracted(documentPdf())).toContain('[??, 1]')
  })
})
