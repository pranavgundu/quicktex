export type Place = {
  math: boolean
  preamble: boolean
  picture: boolean
  plot: boolean
  open: string[]
}

export type Heading = { level: number; title: string; at: number }

type Frame = { close: string; mode: 'math' | 'text' | null; name?: string }

const mathEnvironments = new Set([
  'equation', 'equation*', 'align', 'align*', 'alignat', 'alignat*', 'flalign', 'flalign*', 'gather', 'gather*',
  'multline', 'multline*', 'displaymath', 'math',
])
const textGroups = /(?:text|textrm|textbf|textit|textsf|texttt|textnormal|textup|mbox|hbox|emph)$/
const levels: Record<string, number> = { section: 0, subsection: 1, subsubsection: 2 }

function inMath(frames: Frame[]) {
  for (let i = frames.length - 1; i >= 0; i--) {
    const mode = frames[i].mode
    if (mode) return mode === 'math'
  }
  return false
}

function closeLast(frames: Frame[], close: string) {
  for (let i = frames.length - 1; i >= 0; i--) {
    if (frames[i].close !== close) continue
    frames.length = i
    return
  }
}

export function placeAt(text: string, at: number): Place {
  const frames: Frame[] = []
  let started = false
  let i = 0
  while (i < at) {
    const c = text[i]
    if (c === '%') {
      const end = text.indexOf('\n', i)
      i = end < 0 ? at : end
      continue
    }
    if (c === '\n') {
      const top = frames[frames.length - 1]
      if (top?.close === '$' && /^[ \t]*\n/.test(text.slice(i + 1, i + 81))) frames.pop()
      i++
      continue
    }
    if (c === '{') {
      frames.push({ close: '}', mode: null })
      i++
      continue
    }
    if (c === '}') {
      if (frames[frames.length - 1]?.close === '}') frames.pop()
      i++
      continue
    }
    if (c === '$') {
      const mark = i + 1 < at && text[i + 1] === '$' ? '$$' : '$'
      if (frames[frames.length - 1]?.close === mark) frames.pop()
      else frames.push({ close: mark, mode: 'math' })
      i += mark.length
      continue
    }
    if (c !== '\\') {
      i++
      continue
    }
    const next = text[i + 1]
    if (next === '(' || next === '[') {
      frames.push({ close: next === '(' ? '\\)' : '\\]', mode: 'math' })
      i += 2
      continue
    }
    if (next === ')' || next === ']') {
      closeLast(frames, next === ')' ? '\\)' : '\\]')
      i += 2
      continue
    }
    let end = i + 1
    while (end < at && /[a-zA-Z]/.test(text[end])) end++
    if (end === i + 1) {
      i += 2
      continue
    }
    const word = text.slice(i + 1, end)
    if (word === 'begin' || word === 'end') {
      const stop = text.indexOf('}', end)
      if (text[end] !== '{' || stop < 0 || stop >= at) {
        i = end
        continue
      }
      const name = text.slice(end + 1, stop)
      i = stop + 1
      if (word === 'end') {
        closeLast(frames, `end:${name}`)
        continue
      }
      if (name === 'document') started = true
      frames.push({ close: `end:${name}`, mode: mathEnvironments.has(name) ? 'math' : null, name })
      continue
    }
    i = end
    if (text[end] === '{' && textGroups.test(word) && word.length === textGroups.exec(word)![0].length && end < at) {
      frames.push({ close: '}', mode: 'text' })
      i = end + 1
    }
  }
  const open = frames.flatMap((frame) => (frame.name ? [frame.name] : []))
  return {
    math: inMath(frames),
    preamble: !started && text.includes('\\begin{document}'),
    picture: open.includes('tikzpicture'),
    plot: open.includes('axis'),
    open,
  }
}

export function withoutComment(line: string) {
  if (!line.includes('%')) return line
  return line.replace(/(^|[^\\])(\\\\)*%.*$/, '$1$2')
}

export function outline(text: string): Heading[] {
  const headings: Heading[] = []
  const pattern = /\\(section|subsection|subsubsection)\*?\s*\{/g
  for (const match of text.matchAll(pattern)) {
    const line = text.lastIndexOf('\n', match.index) + 1
    if (withoutComment(text.slice(line, match.index + 1)).length <= match.index - line) continue
    let depth = 1
    let end = match.index + match[0].length
    const start = end
    while (end < text.length && depth > 0) {
      if (text[end] === '\\') end++
      else if (text[end] === '{') depth++
      else if (text[end] === '}') depth--
      end++
    }
    const title = text.slice(start, depth === 0 ? end - 1 : end).replace(/\s+/g, ' ').trim()
    headings.push({ level: levels[match[1]], title, at: match.index })
  }
  return headings
}

export function currentHeading(headings: Heading[], at: number) {
  let found = -1
  for (let i = 0; i < headings.length && headings[i].at <= at; i++) found = i
  return found
}

export function countWords(text: string) {
  const start = text.indexOf('\\begin{document}')
  const body = text
    .slice(start < 0 ? 0 : start)
    .replace(/(^|[^\\])%.*$/gm, '$1')
    .replace(/\\begin\{(tikzpicture|tabular|equation\*?|align\*?|gather\*?)\}[\s\S]*?\\end\{\1\}/g, ' ')
    .replace(/\$\$[\s\S]*?\$\$|\\\[[\s\S]*?\\\]|\\\([\s\S]*?\\\)|(?<!\\)\$[^$]*\$/g, ' x ')
    .replace(/\\(?:begin|end|label|usepackage|documentclass|vspace|setlength)\*?(?:\[[^\]]*\])?(?:\{[^}]*\})*/g, ' ')
    .replace(/\\[a-zA-Z]+\*?(?:\[[^\]]*\])?/g, ' ')
  return body.match(/[\p{L}\p{N}][\p{L}\p{N}'’-]*/gu)?.length ?? 0
}

export function packageLine(text: string, name: string) {
  const loaded = new RegExp(String.raw`\\usepackage(?:\[[^\]]*\])?\{[^}]*(?<![a-zA-Z0-9-])${name}(?![a-zA-Z0-9-])`)
  if (loaded.test(text)) return null
  const body = text.indexOf('\\begin{document}')
  const preamble = body < 0 ? text : text.slice(0, body)
  let at = 0
  for (const match of preamble.matchAll(/^[ \t]*\\(?:usepackage|documentclass)\b.*$/gm)) {
    at = match.index + match[0].length + 1
  }
  if (at > text.length) return { at: text.length, insert: `\n\\usepackage{${name}}\n` }
  return { at, insert: `\\usepackage{${name}}\n` }
}
