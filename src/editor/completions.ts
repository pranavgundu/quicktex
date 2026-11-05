import { snippet, type Completion, type CompletionContext, type CompletionResult } from '@codemirror/autocomplete'
import type { EditorView } from '@codemirror/view'
import { commandList, render } from '../../wasm-core/js/quicktex.js'
import { packageLine, placeAt, type Place } from './scan'

export type Option = Completion & { template: string; preview?: string; needs?: string }

type Row = [label: string, template: string, where?: string]

const kinds = {
  accent: [2, 3, 6, 9, 14, 18, 25, 26, 27, 29, 30, 35, 36, 37, 38, 39, 42],
  pair: [7, 34],
  plain: [1, 4, 5, 8, 15, 16, 17, 28, 31, 32, 40, 41],
  textPlain: [31, 32, 41],
}

const big = 13
const textGroup = 18
const inText = 1
const textOnly = 2
const preloaded = new Set(['amsmath'])
const missingInText = new Set(['circledR', 'yen', 'checkmark', 'maltese'])

const packageTemplates: Record<string, string> = {
  qty: String.raw`\qty(#{})#{}`,
  quantity: String.raw`\quantity(#{})#{}`,
  mqty: String.raw`\mqty(#{})#{}`,
  matrixquantity: String.raw`\matrixquantity(#{})#{}`,
  cancel: String.raw`\cancel{#{}}#{}`,
  bcancel: String.raw`\bcancel{#{}}#{}`,
  xcancel: String.raw`\xcancel{#{}}#{}`,
  centernot: String.raw`\centernot{#{=}}#{}`,
}

const common = new Set(String.raw`frac sqrt sum int prod lim infty alpha beta gamma delta theta lambda mu pi sigma omega partial nabla cdot times leq geq neq approx to rightarrow mathbb mathbf mathrm text left right hat bar vec dots ldots cdots quad section subsection textbf textit emph item caption label usepackage title author`.split(' '))

export const colors = [
  'black', 'white', 'red', 'green', 'blue', 'cyan', 'magenta', 'yellow', 'gray', 'darkgray', 'lightgray', 'brown', 'lime',
  'olive', 'orange', 'pink', 'purple', 'teal', 'violet',
]

const mathRows: Row[] = [
  [String.raw`\left( \right)`, String.raw`\left( #{} \right)#{}`],
  [String.raw`\left[ \right]`, String.raw`\left[ #{} \right]#{}`],
  [String.raw`\left\{ \right\}`, String.raw`\left\\{ #{} \right\\}#{}`],
  [String.raw`\left| \right|`, String.raw`\left| #{} \right|#{}`],
  [String.raw`\sum_{}^{}`, String.raw`\sum_{#{i=1}}^{#{n}} #{}`],
  [String.raw`\prod_{}^{}`, String.raw`\prod_{#{i=1}}^{#{n}} #{}`],
  [String.raw`\int_{}^{}`, String.raw`\int_{#{a}}^{#{b}} #{}`],
  [String.raw`\lim_{}`, String.raw`\lim_{#{x \to 0}} #{}`],
  [String.raw`\hspace`, String.raw`\hspace{#{1em}}`],
  [String.raw`\kern`, String.raw`\kern#{1em}`],
  [String.raw`\hskip`, String.raw`\hskip#{1em}`],
  [String.raw`\mkern`, String.raw`\mkern#{18mu}`],
  [String.raw`\mskip`, String.raw`\mskip#{18mu}`],
  [String.raw`\rule`, String.raw`\rule{#{1em}}{#{1pt}}`],
  [String.raw`\color`, String.raw`\color{#{red}}`],
  [String.raw`\textcolor`, String.raw`\textcolor{#{red}}{#{}}#{}`],
  [String.raw`\hline`, String.raw`\hline`, 'array'],
  [String.raw`\limits`, String.raw`\limits_{#{}}^{#{}}`],
  [String.raw`\nolimits`, String.raw`\nolimits_{#{}}^{#{}}`],
]

const textRows: Row[] = [
  [String.raw`\documentclass`, String.raw`\documentclass{#{article}}`, 'preamble'],
  [String.raw`\usepackage`, String.raw`\usepackage{#{amsmath}}`, 'preamble'],
  [String.raw`\title`, String.raw`\title{#{}}`, 'preamble'],
  [String.raw`\author`, String.raw`\author{#{}}`, 'preamble'],
  [String.raw`\date`, String.raw`\date{#{}}`, 'preamble'],
  [String.raw`\geometry`, String.raw`\geometry{margin=#{1in}}`, 'preamble'],
  [String.raw`\setlength`, String.raw`\setlength{\parindent}{#{0pt}}`, 'preamble'],
  [String.raw`\newcommand`, String.raw`\newcommand{\#{name}}{#{}}`, 'preamble'],
  [String.raw`\renewcommand`, String.raw`\renewcommand{\#{name}}{#{}}`, 'preamble'],
  [String.raw`\DeclareMathOperator`, String.raw`\DeclareMathOperator{\#{name}}{#{name}}`, 'preamble'],
  [String.raw`\definecolor`, String.raw`\definecolor{#{name}}{HTML}{#{336699}}`, 'preamble'],
  [String.raw`\maketitle`, String.raw`\maketitle`, 'block'],
  [String.raw`\section`, String.raw`\section{#{}}`, 'block'],
  [String.raw`\subsection`, String.raw`\subsection{#{}}`, 'block'],
  [String.raw`\subsubsection`, String.raw`\subsubsection{#{}}`, 'block'],
  [String.raw`\section*`, String.raw`\section*{#{}}`, 'block'],
  [String.raw`\subsection*`, String.raw`\subsection*{#{}}`, 'block'],
  [String.raw`\newpage`, String.raw`\newpage`, 'block'],
  [String.raw`\clearpage`, String.raw`\clearpage`, 'block'],
  [String.raw`\noindent`, String.raw`\noindent`, 'block'],
  [String.raw`\centering`, String.raw`\centering`, 'body'],
  [String.raw`\smallskip`, String.raw`\smallskip`, 'block'],
  [String.raw`\medskip`, String.raw`\medskip`, 'block'],
  [String.raw`\bigskip`, String.raw`\bigskip`, 'block'],
  [String.raw`\vspace`, String.raw`\vspace{#{1em}}`, 'block'],
  [String.raw`\hspace`, String.raw`\hspace{#{1em}}`, 'body'],
  [String.raw`\label`, String.raw`\label{#{}}`, 'body'],
  [String.raw`\bfseries`, String.raw`\bfseries`, 'body'],
  [String.raw`\itshape`, String.raw`\itshape`, 'body'],
  [String.raw`\ttfamily`, String.raw`\ttfamily`, 'body'],
  [String.raw`\sffamily`, String.raw`\sffamily`, 'body'],
  [String.raw`\rmfamily`, String.raw`\rmfamily`, 'body'],
  [String.raw`\normalfont`, String.raw`\normalfont`, 'body'],
  [String.raw`\LaTeX`, String.raw`\LaTeX`, 'body'],
  [String.raw`\TeX`, String.raw`\TeX`, 'body'],
  [String.raw`\tiny`, String.raw`\tiny`, 'body'],
  [String.raw`\scriptsize`, String.raw`\scriptsize`, 'body'],
  [String.raw`\footnotesize`, String.raw`\footnotesize`, 'body'],
  [String.raw`\small`, String.raw`\small`, 'body'],
  [String.raw`\normalsize`, String.raw`\normalsize`, 'body'],
  [String.raw`\large`, String.raw`\large`, 'body'],
  [String.raw`\Large`, String.raw`\Large`, 'body'],
  [String.raw`\LARGE`, String.raw`\LARGE`, 'body'],
  [String.raw`\huge`, String.raw`\huge`, 'body'],
  [String.raw`\Huge`, String.raw`\Huge`, 'body'],
  [String.raw`\item`, String.raw`\item `, 'list'],
  [String.raw`\caption`, String.raw`\caption{#{}}`, 'float'],
  [String.raw`\hline`, String.raw`\hline`, 'table'],
]

const pictureRows: Row[] = [
  [String.raw`\draw`, String.raw`\draw #{(0,0)} -- #{(1,1)};`],
  [String.raw`\draw[]`, String.raw`\draw[#{thick}] #{(0,0)} -- #{(1,1)};`],
  [String.raw`\fill`, String.raw`\fill[#{gray}] #{(0,0)} rectangle #{(1,1)};`],
  [String.raw`\filldraw`, String.raw`\filldraw[fill=#{gray}] #{(0,0)} circle (#{1});`],
  [String.raw`\path`, String.raw`\path #{(0,0)} -- #{(1,1)};`],
  [String.raw`\node`, String.raw`\node at #{(0,0)} {#{}};`],
  [String.raw`\coordinate`, String.raw`\coordinate (#{A}) at #{(0,0)};`],
]

const plotRows: Row[] = [
  [String.raw`\addplot`, String.raw`\addplot[#{blue}] coordinates {#{(0,0) (1,1)}};`],
  [String.raw`\addplot{}`, String.raw`\addplot[#{blue}, domain=#{0:1}] {#{x^2}};`],
  [String.raw`\addlegendentry`, String.raw`\addlegendentry{#{}}`],
]

const block = (name: string, body: string, head = '') =>
  `\\begin{${name}}${head}\n  ${body}\n\\end{${name}}#{}`

const textEnvironments: [string, string][] = [
  ['itemize', block('itemize', String.raw`\item #{}`)],
  ['enumerate', block('enumerate', String.raw`\item #{}`)],
  ['equation', block('equation', '#{}')],
  ['equation*', block('equation*', '#{}')],
  ['align', block('align', '#{} &= #{}')],
  ['align*', block('align*', '#{} &= #{}')],
  ['gather', block('gather', '#{}')],
  ['gather*', block('gather*', '#{}')],
  ['center', block('center', '#{}')],
  ['quote', block('quote', '#{}')],
  ['quotation', block('quotation', '#{}')],
  ['abstract', block('abstract', '#{}')],
  ['figure', block('figure', String.raw`\centering
  #{}
  \caption{#{}}`, '[h]')],
  ['table', block('table', String.raw`\centering
  \begin{tabular}{#{ll}}
    #{} & #{} \\
  \end{tabular}
  \caption{#{}}`, '[h]')],
  ['tabular', block('tabular', String.raw`#{} & #{} \\`, '{#{ll}}')],
  ['tikzpicture', block('tikzpicture', String.raw`\draw #{(0,0)} -- #{(1,1)};`)],
]

const matrix = (name: string) => [name, block(name, String.raw`#{} & #{} \\
  #{} & #{}`)] as [string, string]

const mathEnvironments: [string, string][] = [
  ...['matrix', 'pmatrix', 'bmatrix', 'Bmatrix', 'vmatrix', 'Vmatrix', 'smallmatrix', 'cases', 'dcases', 'rcases'].map(matrix),
  ['aligned', block('aligned', '#{} &= #{}')],
  ['gathered', block('gathered', '#{}')],
  ['split', block('split', '#{} &= #{}')],
  ['array', block('array', String.raw`#{} & #{} \\`, '{#{cc}}')],
]

const pictureEnvironments: [string, string][] = [['axis', block('axis', String.raw`\addplot[#{blue}] coordinates {#{(0,0) (1,1)}};`, '[xlabel=#{x}, ylabel=#{y}]')]]

const documentEnvironment: [string, string] = ['document', '\\begin{document}\n#{}\n\\end{document}']

const drawOptions = [
  'thin', 'very thin', 'ultra thin', 'semithick', 'thick', 'very thick', 'ultra thick', 'solid', 'dashed', 'densely dashed',
  'loosely dashed', 'dotted', 'densely dotted', 'loosely dotted', '->', '<-', '<->',
  'line width=#{1pt}', 'draw=#{black}', 'fill=#{gray}', 'color=#{black}', 'rotate=#{45}', 'scale=#{2}', 'xscale=#{2}',
  'yscale=#{2}', 'xshift=#{1cm}', 'yshift=#{1cm}', 'line cap=#{round}', 'line join=#{round}',
]

const nodeOptions = [
  'circle', 'rectangle', 'midway', 'near start', 'near end', 'at start', 'at end', 'transform shape', 'anchor=#{center}',
  'text=#{black}', 'font=#{\\small}', 'align=#{center}', 'inner sep=#{2pt}', 'outer sep=#{2pt}', 'minimum width=#{1cm}',
  'minimum height=#{1cm}', 'minimum size=#{1cm}', 'pos=#{0.5}',
]

const axisOptions = [
  'title=#{}', 'xlabel=#{x}', 'ylabel=#{y}', 'xmin=#{0}', 'xmax=#{1}', 'ymin=#{0}', 'ymax=#{1}', 'width=#{8cm}', 'height=#{6cm}',
  'grid=#{major}', 'xmajorgrids', 'ymajorgrids', 'xtick=#{{0,1}}', 'ytick=#{{0,1}}',
  'legend pos=#{north east}', 'scale only axis', 'enlargelimits=#{false}',
]

const plotOptions = [
  'mark=#{*}', 'mark size=#{2pt}', 'only marks', 'no marks', 'no markers', 'smooth', 'sharp plot', 'forget plot', 'domain=#{0:1}',
  'samples=#{50}', 'thick', 'thin', 'dashed', 'dotted',
]

const optionValues: Record<string, string[]> = {
  draw: colors,
  fill: colors,
  color: colors,
  text: colors,
  anchor: ['center', 'north', 'south', 'east', 'west', 'north east', 'north west', 'south east', 'south west', 'base', 'mid'],
  'legend pos': ['north east', 'north west', 'south east', 'south west', 'outer north east'],
  'line cap': ['round', 'butt', 'rect'],
  'line join': ['round', 'miter', 'bevel'],
}

function replaceBrace(template: string) {
  const write = snippet(template)
  return (view: EditorView, completion: Completion, from: number, to: number) => {
    write(view, completion, from, view.state.sliceDoc(to, to + 1) === '}' ? to + 1 : to)
  }
}

function loadPackage(view: EditorView, name: string) {
  const line = packageLine(view.state.doc.toString(), name)
  if (line) view.dispatch({ changes: { from: line.at, insert: line.insert } })
}

function option(label: string, template: string, extra: Partial<Option> = {}): Option {
  const name = label.slice(1)
  const write = snippet(template)
  return {
    label,
    template,
    boost: common.has(name) ? 30 : undefined,
    apply: extra.needs
      ? (view, completion, from, to) => {
          write(view, completion, from, to)
          loadPackage(view, extra.needs!)
        }
      : write,
    ...extra,
  }
}

function valueOption(label: string, template = label): Option {
  return { label, template, apply: snippet(template) }
}

function argumentOption(label: string, template = label): Option {
  return { label, template, apply: replaceBrace(`${template}}`) }
}

function arguments_(name: string, count: number) {
  return `\\${name}${'{#{}}'.repeat(count)}${count ? '#{}' : ''}`
}

function sample(name: string, count: number) {
  return `\\${name}${['', '{a}', '{a}{b}'][count]}`
}

function specTemplate(name: string, spec: string) {
  const required = spec.replace(/[^m]/g, '').length
  return arguments_(name, required + (spec.includes('g') ? 1 : 0))
}

export function isPlain(completion: Completion) {
  const { template, needs } = completion as Option
  return !needs && !template.includes('#{')
}

type Lists = { math: Option[]; text: Option[]; packages: Option[] }

let lists: Lists | undefined

function build(): Lists {
  const math = new Map<string, Option>()
  const text = new Map<string, Option>()
  const packages: Option[] = [argumentOption('tikz'), argumentOption('pgfplots')]
  let current = ''
  for (const row of commandList().split('\n')) {
    if (!row) continue
    if (row[0] === '=') {
      current = row.slice(1)
      packages.push(argumentOption(current))
      continue
    }
    if (row[0] === '+') {
      packages.push(argumentOption(row.slice(1)))
      continue
    }
    const [name, second, third] = row.split(' ')
    if (third === undefined) {
      const label = `\\${name}`
      const needs = preloaded.has(current) ? undefined : current
      if (!math.has(label)) math.set(label, option(label, packageTemplates[name] ?? specTemplate(name, second), { detail: needs, needs }))
      continue
    }
    const kind = Number(second)
    const modes = Number(third)
    const label = `\\${name}`
    const count = kinds.pair.includes(kind) ? 2 : kinds.accent.includes(kind) ? 1 : 0
    if (kind === big) math.set(label, option(label, `\\${name}#{(}`, { preview: `\\${name}(` }))
    else if ((kinds.plain.includes(kind) || count > 0) && !(modes & textOnly)) {
      math.set(label, option(label, arguments_(name, count), { preview: sample(name, count) }))
    }
    if (kind === 1 && modes & inText && !missingInText.has(name)) {
      text.set(label, option(label, `\\${name}`, { preview: `\\text{\\${name}}` }))
    }
    if (kinds.textPlain.includes(kind)) text.set(label, option(label, `\\${name}`))
    if (kind === textGroup) text.set(label, option(label, arguments_(name, 1)))
  }
  for (const [label, template] of mathRows) math.set(label, option(label, template))
  for (const [label, template] of textRows) text.set(label, option(label, template))
  return { math: [...math.values()], text: [...text.values()], packages }
}

function all() {
  return (lists ??= build())
}

const textWhere = new Map(textRows.map(([label, , where]) => [label, where]))

function allowedText(label: string, place: Place) {
  const where = textWhere.get(label)
  const top = place.open[place.open.length - 1]
  if (where === 'preamble') return place.preamble
  if (where === 'block') return !place.preamble && top !== 'tabular'
  if (where === 'body') return !place.preamble
  if (where === 'list') return place.open.includes('itemize') || place.open.includes('enumerate')
  if (where === 'float') return place.open.includes('figure') || place.open.includes('table')
  if (where === 'table') return top === 'tabular'
  return !place.preamble
}

function allowedMath(label: string, place: Place) {
  if (label !== '\\hline') return true
  return place.open[place.open.length - 1] === 'array'
}

function environmentOption([name, template]: [string, string]) {
  return option(`\\begin{${name}}`, template)
}

export function environmentsFor(place: Place): [string, string][] {
  if (place.math) return mathEnvironments
  if (place.preamble) return [documentEnvironment]
  if (place.plot) return []
  if (place.picture) return pictureEnvironments
  if (place.open[place.open.length - 1] === 'tabular') return []
  return textEnvironments
}

export function commandsFor(place: Place): Option[] {
  const { math, text } = all()
  const environments = environmentsFor(place).map(environmentOption)
  if (place.math) return [...math.filter((entry) => allowedMath(entry.label, place)), ...environments]
  if (place.plot) return plotRows.map(([label, template]) => option(label, template))
  if (place.picture) return [...pictureRows.map(([label, template]) => option(label, template)), ...environments]
  return [...text.filter((entry) => allowedText(entry.label, place)), ...environments]
}

export function userCommands(text: string): Option[] {
  const found = new Map<string, Option>()
  const pattern = /\\(?:newcommand|renewcommand|providecommand|DeclareMathOperator)\*?\s*\{?\\([a-zA-Z]+)\}?(?:\[(\d)\])?/g
  for (const match of text.matchAll(pattern)) {
    const label = `\\${match[1]}`
    found.set(label, option(label, arguments_(match[1], Number(match[2] ?? 0)), { boost: 40 }))
  }
  return [...found.values()]
}

export function userColors(text: string) {
  return [...text.matchAll(/\\definecolor\{([a-zA-Z0-9]+)\}/g)].map((match) => match[1])
}

export function packageOptions() {
  return all().packages
}

type Target = 'picture' | 'axis' | 'plot' | 'node'

function optionList(target: Target) {
  if (target === 'axis') return axisOptions
  if (target === 'plot') return [...plotOptions, ...colors]
  if (target === 'node') return [...nodeOptions, ...drawOptions, ...colors]
  return [...drawOptions, ...nodeOptions, ...colors]
}

export function optionsFor(target: Target) {
  return optionList(target).map((template) => valueOption(template.replace(/#\{.*\}$/, ''), template))
}

const optionTarget = /\\(draw|fill|filldraw|path|node|coordinate|addplot)\+?\s*\[([^\]]*)$|\\begin\{(tikzpicture|axis)\}\[([^\]]*)$/
const packageTarget = /\\(?:usepackage|RequirePackage)(?:\[[^\]]*\])?\{(?:[^}]*,\s*)?([a-zA-Z0-9-]*)$/
const environmentTarget = /\\begin\{([a-zA-Z*]*)$/
const endTarget = /\\end\{([a-zA-Z*]*)$/
const colorTarget = /\\(?:textcolor|color)\{([a-zA-Z0-9]*)$/

function inComment(before: string) {
  return /(^|[^\\])(\\\\)*%/.test(before)
}

function optionResult(match: RegExpExecArray, pos: number, state: CompletionContext['state']): CompletionResult | null {
  const command = match[1] ?? match[3]
  const list = match[2] ?? match[4]
  const segment = /[^,]*$/.exec(list)![0]
  const value = /^\s*([a-z ]+?)\s*=\s*([a-zA-Z ]*)$/.exec(segment)
  if (value) {
    const values = optionValues[value[1]]
    if (!values) return null
    const extra = colors === values ? userColors(state.doc.toString()) : []
    return { from: pos - value[2].length, options: [...values, ...extra].map((name) => valueOption(name)), validFor: /^[a-zA-Z ]*$/ }
  }
  const lead = /^\s*/.exec(segment)![0].length
  const target = command === 'addplot' ? 'plot' : command === 'axis' ? 'axis' : command === 'node' ? 'node' : 'picture'
  return { from: pos - segment.length + lead, options: optionsFor(target), validFor: /^[a-zA-Z ,<>=-]*$/ }
}

function environmentResult(context: CompletionContext, name: string): CompletionResult {
  const { state, pos } = context
  const text = state.doc.toString()
  const from = pos - name.length
  const place = placeAt(text, from - '\\begin{'.length)
  const hasEnd = name.length > 0 && text.includes(`\\end{${name}}`, pos)
  const options = environmentsFor(place).map(([env, template]) => {
    const body = template.replace(`\\begin{${env}}`, env + '}')
    return { label: env, template, apply: hasEnd ? replaceBrace(`${env}}`) : replaceBrace(body) } as Option
  })
  return { from, options, validFor: /^[a-zA-Z*]*$/ }
}

function endResult(context: CompletionContext, name: string): CompletionResult | null {
  const from = context.pos - name.length
  const place = placeAt(context.state.doc.toString(), from - '\\end{'.length)
  const open = place.open[place.open.length - 1]
  if (!open) return null
  return { from, options: [argumentOption(open)], validFor: /^[a-zA-Z*]*$/ }
}

export function latexCompletions(context: CompletionContext): CompletionResult | null {
  const { state, pos } = context
  const line = state.doc.lineAt(pos)
  const before = line.text.slice(0, pos - line.from)
  if (!before.includes('\\') && !context.explicit && !before.includes('[')) return null
  if (inComment(before)) return null
  const word = context.matchBefore(/\\[a-zA-Z]*/)
  if (word && word.to === pos) {
    const slashes = /\\*$/.exec(state.sliceDoc(Math.max(0, word.from - 16), word.from))![0].length
    if (slashes % 2 === 1) return null
    const text = state.doc.toString()
    const options = [...userCommands(text), ...commandsFor(placeAt(text, word.from))]
    return { from: word.from, options, validFor: /^\\[a-zA-Z]*$/ }
  }
  const pack = packageTarget.exec(before)
  if (pack) return { from: pos - pack[1].length, options: packageOptions(), validFor: /^[a-zA-Z0-9-]*$/ }
  const begin = environmentTarget.exec(before)
  if (begin) return environmentResult(context, begin[1])
  const end = endTarget.exec(before)
  if (end) return endResult(context, end[1])
  const named = colorTarget.exec(before)
  if (named) {
    const options = [...colors, ...userColors(state.doc.toString())].map((name) => argumentOption(name))
    return { from: pos - named[1].length, options, validFor: /^[a-zA-Z0-9]*$/ }
  }
  const target = optionTarget.exec(before)
  if (target) return optionResult(target, pos, state)
  return null
}

const previews = new Map<string, string>()

export function previewOf(tex: string) {
  let svg = previews.get(tex)
  if (svg === undefined) {
    try {
      svg = render(tex)
    } catch {
      svg = ''
    }
    previews.set(tex, svg)
  }
  return svg
}

export function renderPreview(completion: Completion) {
  const tex = (completion as Option).preview
  if (!tex) return null
  const svg = previewOf(tex)
  if (!svg) return null
  const node = document.createElement('span')
  node.className = 'cm-completionPreview'
  node.setAttribute('aria-hidden', 'true')
  node.innerHTML = svg
  return node
}
