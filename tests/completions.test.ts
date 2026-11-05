import { describe, expect, test } from 'bun:test'
import {
  colors,
  commandsFor,
  environmentsFor,
  optionsFor,
  packageOptions,
  previewOf,
  userColors,
  userCommands,
  type Option,
} from '../src/editor/completions'
import { countWords, currentHeading, outline, packageLine, placeAt, withoutComment, type Place } from '../src/editor/scan'
import { documentWarnings } from '../wasm-core/js/quicktex.js'
import { article, documentPage, documentPdf, render, renderDocument } from './support/engine'

const place = (extra: Partial<Place> = {}): Place => ({ math: false, preamble: false, picture: false, plot: false, open: [], ...extra })

function filled(template: string) {
  return template
    .replace(/#\{\}$/, '')
    .replace(/\\([{}])/g, '$1')
    .replace(/#\{((?:[^{}]|\{[^{}]*\})*)\}/g, (_, value) => value || 'a')
}

function failures(options: Option[], wrap: (tex: string) => string) {
  const bad: string[] = []
  for (const entry of options) {
    const tex = filled(entry.template)
    const body = /^\\(no)?limits/.test(tex) ? `\\sum${tex}` : tex
    let doc = wrap(body)
    if (entry.needs) doc = doc.replace('\\begin{document}', `\\usepackage{${entry.needs}}\n\\begin{document}`)
    try {
      renderDocument(doc)
      if (documentWarnings().length) bad.push(`${entry.label}: ignored`)
    } catch (error) {
      bad.push(`${entry.label}: ${(error as Error).message}`)
    }
  }
  return bad
}

const inPicture = (tex: string) => article(`\\begin{tikzpicture}\n${tex}\n\\end{tikzpicture}`)
const inAxis = (tex: string) => inPicture(`\\begin{axis}\n${tex}\n\\end{axis}`)
const preamble = (tex: string) => `\\documentclass{article}\n${tex}\n\\begin{document}\nx\n\\end{document}\n`

describe('every suggestion compiles', () => {
  const cases: [string, Partial<Place>, (tex: string) => string][] = [
    ['display math', { math: true }, (tex) => article(`\\[ ${tex} \\]`)],
    ['array', { math: true, open: ['document', 'array'] }, (tex) => article(`\\[\\begin{array}{cc}\n${tex} a & b\n\\end{array}\\]`)],
    ['body text', { open: ['document'] }, (tex) => article(`x ${tex} y`)],
    ['preamble', { preamble: true }, preamble],
    ['list', { open: ['document', 'itemize'] }, (tex) => article(`\\begin{itemize}\n\\item x ${tex}\n\\end{itemize}`)],
    ['figure', { open: ['document', 'figure'] }, (tex) => article(`\\begin{figure}\nx ${tex}\n\\end{figure}`)],
    ['table', { open: ['document', 'tabular'] }, (tex) => article(`\\begin{tabular}{ll}\n${tex} a & b \\\\\n\\end{tabular}`)],
    ['picture', { picture: true, open: ['document', 'tikzpicture'] }, inPicture],
    ['plot', { picture: true, plot: true, open: ['document', 'tikzpicture', 'axis'] }, inAxis],
  ]

  for (const [name, where, wrap] of cases) {
    test(name, () => {
      const options = commandsFor(place(where))
      expect(options.length).toBeGreaterThan(0)
      expect(failures(options, wrap)).toEqual([])
    })
  }

  test('math offers the engine, not a short list', () => {
    expect(commandsFor(place({ math: true })).length).toBeGreaterThan(800)
  })

  test('draw and node options', () => {
    const draw = optionsFor('picture').filter((entry) => failures([{ ...entry, template: `\\draw[${entry.template}] (0,0) -- (1,1);` }], inPicture).length)
    const node = optionsFor('node').filter((entry) => failures([{ ...entry, template: `\\node[${entry.template}] at (0,0) {a};` }], inPicture).length)
    expect(draw.map((entry) => entry.label)).toEqual([])
    expect(node.map((entry) => entry.label)).toEqual([])
  })

  test('axis and plot options', () => {
    const line = '\\addplot coordinates {(0,0) (1,1)};'
    const axis = optionsFor('axis').filter((entry) => failures([{ ...entry, template: `\\begin{axis}[${entry.template}]\n${line}\n\\end{axis}` }], inPicture).length)
    const plot = optionsFor('plot').filter((entry) => failures([{ ...entry, template: `\\addplot[${entry.template}] coordinates {(0,0) (1,1)};` }], inAxis).length)
    expect(axis.map((entry) => entry.label)).toEqual([])
    expect(plot.map((entry) => entry.label)).toEqual([])
  })

  test('packages load', () => {
    const names = packageOptions().map((entry) => entry.label)
    expect(names).toContain('physics')
    expect(names).toContain('tikz')
    expect(names).toContain('amssymb')
    for (const name of names) expect(() => renderDocument(preamble(`\\usepackage{${name}}`))).not.toThrow()
  })

  test('colors work in math and pictures', () => {
    for (const name of colors) {
      expect(() => render(`\\textcolor{${name}}{a}`)).not.toThrow()
      expect(() => renderDocument(inPicture(`\\draw[${name}] (0,0) -- (1,1);`))).not.toThrow()
    }
  })

  test('package commands fail without the package', () => {
    const needing = commandsFor(place({ math: true })).filter((entry) => entry.needs)
    expect(needing.length).toBeGreaterThan(100)
    const quiet = needing.filter((entry) => {
      renderDocument(article(`\\[ ${filled(entry.template)} \\]`))
      return documentWarnings().length === 0
    })
    expect(quiet.length).toBeLessThan(needing.length / 4)
  })
})

describe('suggestions fit the place', () => {
  const labels = (where: Partial<Place>) => commandsFor(place(where)).map((entry) => entry.label)

  test('math symbols stay out of text', () => {
    expect(labels({ math: true })).toContain('\\alpha')
    expect(labels({ open: ['document'] })).not.toContain('\\alpha')
  })

  test('text only symbols stay out of math', () => {
    expect(labels({ math: true })).not.toContain('\\ae')
    expect(labels({ open: ['document'] })).toContain('\\ae')
    expect(labels({ math: true })).not.toContain('\\textdagger')
  })

  test('preamble commands stay in the preamble', () => {
    expect(labels({ preamble: true })).toContain('\\usepackage')
    expect(labels({ open: ['document'] })).not.toContain('\\usepackage')
    expect(labels({ preamble: true })).not.toContain('\\section')
  })

  test('list, float and table commands need their environment', () => {
    expect(labels({ open: ['document'] })).not.toContain('\\item')
    expect(labels({ open: ['document', 'enumerate'] })).toContain('\\item')
    expect(labels({ open: ['document'] })).not.toContain('\\caption')
    expect(labels({ open: ['document', 'table'] })).toContain('\\caption')
    expect(labels({ open: ['document'] })).not.toContain('\\hline')
    expect(labels({ open: ['document', 'tabular'] })).toContain('\\hline')
    expect(labels({ open: ['document', 'tabular'] })).not.toContain('\\section')
    expect(labels({ math: true })).not.toContain('\\hline')
    expect(labels({ math: true, open: ['array'] })).toContain('\\hline')
  })

  test('pictures offer drawing and plots offer plotting', () => {
    expect(labels({ picture: true })).toContain('\\draw')
    expect(labels({ picture: true })).toContain('\\begin{axis}')
    expect(labels({ picture: true, plot: true })).toContain('\\addplot')
    expect(labels({ picture: true, plot: true })).not.toContain('\\draw')
  })

  test('environments follow the mode', () => {
    const names = (where: Partial<Place>) => environmentsFor(place(where)).map(([name]) => name)
    expect(names({ math: true })).toContain('pmatrix')
    expect(names({ math: true })).not.toContain('itemize')
    expect(names({ open: ['document'] })).toContain('itemize')
    expect(names({ preamble: true })).toEqual(['document'])
    expect(names({ open: ['document', 'tabular'] })).toEqual([])
  })

  test('no label is offered twice', () => {
    for (const where of [{ math: true }, { open: ['document'] }, { preamble: true }]) {
      const all = labels(where)
      expect(new Set(all).size).toBe(all.length)
    }
  })

  test('common commands rank first', () => {
    const math = commandsFor(place({ math: true }))
    expect(math.find((entry) => entry.label === '\\frac')?.boost).toBeGreaterThan(0)
    expect(math.find((entry) => entry.label === '\\frown')?.boost ?? 0).toBe(0)
  })
})

describe('commands from the document', () => {
  test('finds macros with their arguments', () => {
    const found = userCommands(String.raw`\newcommand{\R}{\mathbb{R}} \newcommand{\norm}[1]{\|#1\|} \newcommand\pair[2]{(#1,#2)} \DeclareMathOperator{\tr}{tr}`)
    expect(found.map((entry) => [entry.label, filled(entry.template)])).toEqual([
      ['\\R', '\\R'],
      ['\\norm', '\\norm{a}'],
      ['\\pair', '\\pair{a}{a}'],
      ['\\tr', '\\tr'],
    ])
  })

  test('finds colors', () => {
    expect(userColors(String.raw`\definecolor{brand}{HTML}{336699} \definecolor{ink2}{rgb}{0,0,0}`)).toEqual(['brand', 'ink2'])
  })
})

describe('previews', () => {
  test('draw symbols and accents', () => {
    const math = commandsFor(place({ math: true }))
    for (const name of ['\\alpha', '\\frac', '\\hat', '\\sum', '\\bigl']) {
      const entry = math.find((candidate) => candidate.label === name)
      expect(previewOf(entry!.preview!)).toContain('<svg')
    }
  })

  test('every preview renders', () => {
    const missing = [...commandsFor(place({ math: true })), ...commandsFor(place({ open: ['document'] }))]
      .filter((entry) => entry.preview && !previewOf(entry.preview))
      .map((entry) => entry.label)
    expect(missing).toEqual([])
  })

  test('leave the laid out document alone', () => {
    const tex = article(String.raw`\section{A} text $x^2$ \begin{tikzpicture}\draw (0,0) -- (1,1);\end{tikzpicture}`)
    renderDocument(tex)
    const page = documentPage(0)
    const pdf = documentPdf().length
    render(String.raw`\frac{a}{b}`)
    render(String.raw`\textcolor{red}{a}`)
    expect(documentPage(0)).toBe(page)
    expect(documentPdf().length).toBe(pdf)
  })
})

describe('placeAt', () => {
  const at = (marked: string) => {
    const index = marked.indexOf('|')
    return placeAt(marked.replace('|', ''), index)
  }

  test('inline math', () => {
    expect(at('a $x| y$ b').math).toBe(true)
    expect(at('a $x$ |b').math).toBe(false)
    expect(at('a \\$ |b').math).toBe(false)
    expect(at('a \\(x|\\) b').math).toBe(true)
    expect(at('a $$x|$$ b').math).toBe(true)
    expect(at('a $$x$$ |b').math).toBe(false)
  })

  test('display math and environments', () => {
    expect(at('\\[ x| \\]').math).toBe(true)
    expect(at('\\[ x \\] |').math).toBe(false)
    expect(at('\\\\[2pt] |').math).toBe(false)
    expect(at('\\begin{align*} x| \\end{align*}').math).toBe(true)
    expect(at('\\begin{align*} x \\end{align*} |').math).toBe(false)
    expect(at('\\begin{itemize} \\item | \\end{itemize}').math).toBe(false)
  })

  test('text inside math', () => {
    expect(at('$\\text{a|}$').math).toBe(false)
    expect(at('$\\text{a}|$').math).toBe(true)
    expect(at('$\\text{a $b|$}$').math).toBe(true)
    expect(at('$\\mathbf{a|}$').math).toBe(true)
    expect(at('$\\context{a|}$').math).toBe(true)
  })

  test('comments and stray dollars', () => {
    expect(at('% $\n|').math).toBe(false)
    expect(at('\\% $x|$').math).toBe(true)
    expect(at('a $ b\n\nc |').math).toBe(false)
  })

  test('tracks open environments', () => {
    expect(at('\\begin{document}\\begin{itemize}\\begin{center}\\end{center}|').open).toEqual(['document', 'itemize'])
    expect(at('\\begin{tikzpicture}\\begin{axis}|').plot).toBe(true)
    expect(at('\\begin{tikzpicture}|\\end{tikzpicture}').picture).toBe(true)
    expect(at('\\begin{tikzpicture}\\end{tikzpicture}|').picture).toBe(false)
  })

  test('preamble', () => {
    expect(at('\\documentclass{article}\n|\n\\begin{document}\n\\end{document}').preamble).toBe(true)
    expect(at('\\begin{document}\n|\n\\end{document}').preamble).toBe(false)
    expect(at('plain |').preamble).toBe(false)
  })

  test('stops at the cursor', () => {
    expect(at('$x$|$').math).toBe(false)
    expect(at('\\begin{al|ign}').open).toEqual([])
  })
})

describe('outline', () => {
  const text = String.raw`\section{One}
\subsection{Two $x^{2}$}
% \section{Hidden}
\subsubsection*{Three}
100\% \section{Four}`

  test('lists headings with levels', () => {
    expect(outline(text).map((heading) => [heading.level, heading.title])).toEqual([
      [0, 'One'],
      [1, 'Two $x^{2}$'],
      [2, 'Three'],
      [0, 'Four'],
    ])
  })

  test('points at the source', () => {
    for (const heading of outline(text)) expect(text.slice(heading.at, heading.at + 4)).toMatch(/^\\(sub)?/)
    expect(text.slice(outline(text)[1].at).startsWith('\\subsection')).toBe(true)
  })

  test('survives an unfinished heading', () => {
    expect(outline(String.raw`\section{Open`)[0].title).toBe('Open')
  })

  test('finds the heading at the cursor', () => {
    const headings = outline(text)
    expect(currentHeading(headings, 0)).toBe(0)
    expect(currentHeading(headings, headings[2].at + 1)).toBe(2)
    expect(currentHeading(headings, text.length)).toBe(3)
    expect(currentHeading(outline(`x\n${text}`), 0)).toBe(-1)
  })
})

describe('comments', () => {
  test('removes real comments only', () => {
    expect(withoutComment('a % b')).toBe('a ')
    expect(withoutComment('% b')).toBe('')
    expect(withoutComment('100\\% b')).toBe('100\\% b')
    expect(withoutComment('a \\\\% b')).toBe('a \\\\')
  })
})

describe('countWords', () => {
  test('counts prose', () => {
    expect(countWords(article('one two three'))).toBe(3)
    expect(countWords("don't well-known")).toBe(2)
  })

  test('skips the preamble and commands', () => {
    expect(countWords('\\documentclass{article}\n\\title{Hi there}\n\\begin{document}\n\\section{Intro} \\textbf{bold} word\n\\end{document}')).toBe(3)
  })

  test('counts math as one word and skips pictures', () => {
    expect(countWords(article('a $x^2 + y^2$ b'))).toBe(3)
    expect(countWords(article('a \\begin{tikzpicture}\\draw (0,0);\\end{tikzpicture} b'))).toBe(2)
    expect(countWords(article('a\n% hidden words\nb'))).toBe(2)
  })

  test('handles an empty document', () => {
    expect(countWords('')).toBe(0)
  })
})

describe('packageLine', () => {
  test('skips a loaded package', () => {
    expect(packageLine('\\usepackage{physics}\n', 'physics')).toBeNull()
    expect(packageLine('\\usepackage[a]{amsmath, physics}\n', 'physics')).toBeNull()
  })

  test('does not mistake a longer name', () => {
    expect(packageLine('\\usepackage{physics2}\n', 'physics')).not.toBeNull()
  })

  test('goes after the last package', () => {
    const text = '\\documentclass{article}\n\\usepackage{a}\n\\usepackage{b}\n\\title{x}\n\\begin{document}\n\\usepackage{c}\n'
    expect(packageLine(text, 'physics')).toEqual({ at: text.indexOf('\\title'), insert: '\\usepackage{physics}\n' })
  })

  test('goes after the class when there are no packages', () => {
    const text = '\\documentclass{article}\n\\begin{document}\n'
    expect(packageLine(text, 'physics')?.at).toBe(text.indexOf('\\begin'))
  })

  test('goes to the top of a bare document', () => {
    expect(packageLine('hello', 'physics')?.at).toBe(0)
  })

  test('handles a class on the last line', () => {
    expect(packageLine('\\documentclass{article}', 'physics')).toEqual({ at: 23, insert: '\n\\usepackage{physics}\n' })
  })
})
