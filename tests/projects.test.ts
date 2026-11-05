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
