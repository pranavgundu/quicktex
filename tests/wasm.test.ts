import { describe, expect, test } from 'bun:test'
import { readFileSync } from 'node:fs'
import { QuicktexError, usePackage } from '../wasm-core/js/quicktex.js'
import { article, documentPage, documentPdf, fixture, pages, random, render, renderDocument } from './support/engine'

const bytes = readFileSync(new URL('../wasm-core/js/quicktex.wasm', import.meta.url))

function failure(run: () => unknown) {
  try {
    run()
  } catch (error) {
    if (error instanceof QuicktexError) return error
    throw error
  }
  throw new Error('expected a QuicktexError')
}

describe('render', () => {
  test('returns an svg string', () => {
    const svg = render('x^2')
    expect(svg.startsWith('<svg xmlns="http://www.w3.org/2000/svg"')).toBe(true)
    expect(svg.endsWith('</svg>')).toBe(true)
  })

  test('display defaults to off', () => {
    expect(render('\\sum_i')).toBe(render('\\sum_i', false))
    expect(render('\\sum_i')).not.toBe(render('\\sum_i', true))
  })

  test('accepts empty input', () => {
    expect(render('')).toContain('<svg')
  })

  test('handles ascii on the fast path and unicode on the slow path', () => {
    expect(render('α+β')).toBe(render('\\alpha+\\beta'))
    expect(render('x'.repeat(600))).toBe(render('x'.repeat(600)))
    expect(render('x'.repeat(600) + 'α')).toBe(render('x'.repeat(600) + '\\alpha'))
  })

  test('grows its input buffer for long formulas', () => {
    const long = 'x+'.repeat(200000) + 'y'
    expect(render(long).length).toBeGreaterThan(100000)
    expect(render('x')).toContain('<path')
  })

  test('renders the same thing twice', () => {
    expect(render('\\frac{a}{b}')).toBe(render('\\frac{a}{b}'))
  })

  test('survives memory growth between calls', () => {
    const before = render('x')
    render('x+'.repeat(500000) + 'y')
    expect(render('x')).toBe(before)
  })
})

describe('errors', () => {
  const rows: [string, string, number][] = [
    ['\\foo', 'unknown command', 0],
    ['x\\foo', 'unknown command', 1],
    ['{', 'unexpected end of input', 1],
    ['}', 'unbalanced braces', 0],
    ['x^2^3', 'double script', 3],
    ['\\frac', 'missing argument', 5],
    ['\\begin{nope}', 'unknown or mismatched environment', 0],
    ['\\usepackage{nope}', 'unknown package', 0],
  ]
  for (const [tex, message, position] of rows) {
    test(`${tex} reports ${message}`, () => {
      const error = failure(() => render(tex))
      expect(error.message).toBe(message)
      expect(error.name).toBe('QuicktexError')
      expect(error).toBeInstanceOf(Error)
      if (position > 0) expect(error.position).toBe(position)
    })
  }

  test('position counts utf-8 bytes', () => {
    expect(failure(() => render('α\\foo')).position).toBe(2)
  })

  test('the engine keeps working after an error', () => {
    const good = render('x')
    failure(() => render('\\foo'))
    expect(render('x')).toBe(good)
  })
})

describe('usePackage', () => {
  test('refuses an unknown name', () => {
    expect(failure(() => usePackage('notapackage')).message).toBe('unknown package notapackage')
  })

  test('makes commands available without a preamble', () => {
    expect(failure(() => render('\\slashed{D}')).message).toBe('unknown command')
    usePackage('slashed')
    expect(render('\\slashed{D}')).toContain('<path')
  })
})

describe('documents', () => {
  test('counts pages', () => {
    expect(renderDocument(fixture('lab'))).toBe(2)
    expect(renderDocument(fixture('shapes'))).toBe(1)
    expect(renderDocument(article('a\\newpage b'))).toBe(2)
  })

  test('returns one svg per page', () => {
    for (const svg of pages(fixture('lab'))) {
      expect(svg.startsWith('<svg')).toBe(true)
      expect(svg.endsWith('</svg>')).toBe(true)
    }
  })

  test('missing pages are empty strings', () => {
    renderDocument(article('x'))
    expect(documentPage(1)).toBe('')
    expect(documentPage(1000)).toBe('')
  })

  test('throws with a position on errors', () => {
    const tex = article('\\foo')
    const error = failure(() => renderDocument(tex))
    expect(error.message).toBe('unknown command')
    expect(tex.slice(error.position, error.position + 4)).toBe('\\foo')
  })

  test('keeps the last good pages after an error', () => {
    const tex = fixture('lab')
    const before = pages(tex)
    failure(() => renderDocument(tex.replace('\\draw[dashed, gray]', '\\draw[wobble]')))
    expect(documentPage(0)).toBe(before[0])
    expect(documentPage(1)).toBe(before[1])
  })

  test('math between document calls leaves the pages alone', () => {
    const before = pages(fixture('lab'))
    render('\\frac{a}{b}')
    expect(documentPage(0)).toBe(before[0])
  })
})

describe('pdf', () => {
  test('is a real pdf', () => {
    renderDocument(fixture('lab'))
    const pdf = documentPdf()
    expect(new TextDecoder().decode(pdf.slice(0, 8))).toBe('%PDF-1.4')
    expect(new TextDecoder().decode(pdf.slice(-6)).trim()).toBe('%%EOF')
  })

  test('is a copy that later calls cannot change', () => {
    renderDocument(fixture('lab'))
    const pdf = documentPdf()
    const copy = pdf.slice()
    renderDocument(article('x'))
    documentPdf()
    expect(pdf).toEqual(copy)
  })

  test('is the same every time', () => {
    renderDocument(fixture('lab'))
    expect(documentPdf()).toEqual(documentPdf())
  })
})

describe('the wasm binary', () => {
  test('needs no imports', () => {
    expect(WebAssembly.Module.imports(new WebAssembly.Module(bytes))).toEqual([])
  })

  test('exports exactly the api', () => {
    const names = WebAssembly.Module.exports(new WebAssembly.Module(bytes)).map((entry) => entry.name)
    for (const name of ['memory', 'qt_input', 'qt_render', 'qt_document', 'qt_page', 'qt_pdf', 'qt_use', 'qt_output', 'qt_error_pos']) {
      expect(names).toContain(name)
    }
  })

  test('carries the font license', () => {
    const sections = WebAssembly.Module.customSections(new WebAssembly.Module(bytes), 'license')
    expect(sections.length).toBeGreaterThan(0)
    expect(new TextDecoder().decode(sections[0])).toContain('SIL Open Font License')
  })

  test('stays under the size budget', () => {
    expect(bytes.length).toBeLessThan(1_200_000)
    expect(Bun.gzipSync(bytes).length).toBeLessThan(450_000)
  })
})

describe('robustness of the real binary', () => {
  const tokens = ['{', '}', '$', '\\\\', '&', '%', '\\begin{figure}', '\\end{figure}', '\\begin{tabular}{c}', '\\end{tabular}', '\\begin{tikzpicture}', '\\end{tikzpicture}', '\\begin{axis}', '\\end{axis}', '\\draw', '\\node', '--', '(', ')', '[', ']', ';', ',', '=', '\\caption{', '\\section{', '\\newpage', 'é', '日', '\n\n', 'scale=1e9', '1e999']

  function mutate(next: (limit: number) => number, base: string) {
    let text = base
    for (let round = 0; round <= next(4); round++) {
      const at = next(text.length + 1)
      const kind = next(4)
      if (kind === 0) text = text.slice(0, at) + tokens[next(tokens.length)] + text.slice(at)
      else if (kind === 1) text = text.slice(0, at) + text.slice(at + next(24))
      else if (kind === 2) text = text.slice(0, at)
      else text = text.slice(0, at) + tokens[next(tokens.length)] + text.slice(at + 1)
    }
    return text
  }

  for (const name of ['lab', 'shapes', 'sample']) {
    test(`${name} survives 30000 mutations`, () => {
      const base = fixture(name)
      const next = random(name.length * 7919)
      let good = 0
      for (let i = 0; i < 30000; i++) {
        const text = mutate(next, base)
        try {
          const count = renderDocument(text)
          documentPage(next(count))
          if (i % 50 === 0) documentPdf()
          good++
        } catch (error) {
          if (!(error instanceof QuicktexError)) throw new Error(`trap on ${JSON.stringify(text.slice(0, 300))}: ${error}`)
        }
      }
      expect(good).toBeGreaterThan(0)
    })
  }

  test('math survives 200000 mutations', () => {
    const next = random(42)
    const seeds = ['\\frac{-b\\pm\\sqrt{b^2-4ac}}{2a}', '\\begin{pmatrix}a&b\\\\c&d\\end{pmatrix}', '\\left(\\sum_{i=1}^n\\frac{1}{i}\\right)^2', '\\newcommand{\\f}[1]{#1^2}\\f{x}']
    for (let i = 0; i < 200000; i++) {
      const text = mutate(next, seeds[i % seeds.length])
      try {
        render(text, i % 2 === 0)
      } catch (error) {
        if (!(error instanceof QuicktexError)) throw new Error(`trap on ${JSON.stringify(text)}: ${error}`)
      }
    }
  })

  test('memory stays flat over many compiles', () => {
    const exports = new WebAssembly.Instance(new WebAssembly.Module(bytes), {}).exports as Record<string, any>
    const encoded = new TextEncoder().encode(fixture('lab'))
    const input = exports.qt_input(1 << 18)
    const compile = () => {
      new Uint8Array(exports.memory.buffer).set(encoded, input)
      exports.qt_document(encoded.length)
      exports.qt_page(0)
      exports.qt_pdf()
    }
    for (let i = 0; i < 100; i++) compile()
    const settled = exports.memory.buffer.byteLength
    for (let i = 0; i < 5000; i++) compile()
    expect(exports.memory.buffer.byteLength).toBe(settled)
  })
})
