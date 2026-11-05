import { indentUnit, foldService, getIndentUnit, indentService, indentString } from '@codemirror/language'
import { EditorSelection, Prec, type EditorState, type Extension, type Text } from '@codemirror/state'
import { Decoration, EditorView, type Command } from '@codemirror/view'
import { placeAt, withoutComment } from './scan'

const token = /\\(begin|end)\{([a-zA-Z*]+)\}/g
const heading = /\\(section|subsection|subsubsection)\*?\s*\{/
const depth: Record<string, number> = { section: 0, subsection: 1, subsubsection: 2 }
const listNames = new Set(['itemize', 'enumerate'])
const closers: Record<string, string> = { '{': '\\}', '[': '\\]', '(': '\\)' }
const openers: Record<string, string> = { '}': '{', ']': '[', ')': '(' }

function escaped(state: EditorState, at: number) {
  let count = 0
  while (at - count > 0 && state.sliceDoc(at - count - 1, at - count) === '\\') count++
  return count % 2 === 1
}

function matchingEnd(doc: Text, lineNumber: number, offset: number, name: string) {
  let open = 1
  for (let number = lineNumber; number <= doc.lines; number++) {
    const line = doc.line(number)
    if (!line.text.includes(name)) continue
    const text = withoutComment(line.text)
    for (const match of text.matchAll(token)) {
      if (match[2] !== name || (number === lineNumber && match.index < offset)) continue
      open += match[1] === 'begin' ? 1 : -1
      if (open === 0) return { line, index: match.index, length: match[0].length }
    }
  }
  return null
}

function matchingBegin(doc: Text, lineNumber: number, offset: number, name: string) {
  let open = 1
  for (let number = lineNumber; number >= 1; number--) {
    const line = doc.line(number)
    if (!line.text.includes(name)) continue
    const matches = [...withoutComment(line.text).matchAll(token)].reverse()
    for (const match of matches) {
      if (match[2] !== name || (number === lineNumber && match.index >= offset)) continue
      open += match[1] === 'end' ? 1 : -1
      if (open === 0) return { line, index: match.index, length: match[0].length }
    }
  }
  return null
}

export function foldRange(state: EditorState, from: number, to: number) {
  const line = state.doc.lineAt(from)
  if (!line.text.includes('\\')) return null
  const text = withoutComment(line.text)
  const section = heading.exec(text)
  if (section) return sectionRange(state, line.number, to, depth[section[1]])
  for (const match of text.matchAll(token)) {
    if (match[1] !== 'begin' || match[2] === 'document') continue
    const end = matchingEnd(state.doc, line.number, match.index + match[0].length, match[2])
    if (!end || end.line.number === line.number) return null
    return { from: to, to: end.line.from - 1 }
  }
  return null
}

function sectionRange(state: EditorState, lineNumber: number, from: number, level: number) {
  const { doc } = state
  for (let number = lineNumber + 1; number <= doc.lines; number++) {
    const line = doc.line(number)
    if (!line.text.includes('\\')) continue
    const text = withoutComment(line.text)
    const next = heading.exec(text)
    if ((next && depth[next[1]] <= level) || text.includes('\\end{document}')) {
      let last = number - 1
      while (last > lineNumber && doc.line(last).text.trim() === '') last--
      return last > lineNumber ? { from, to: doc.line(last).to } : null
    }
  }
  return doc.lines > lineNumber ? { from, to: doc.length } : null
}

function pairAt(state: EditorState, at: number) {
  const line = state.doc.lineAt(at)
  if (!line.text.includes('\\')) return null
  const offset = at - line.from
  for (const match of withoutComment(line.text).matchAll(token)) {
    const stop = match.index + match[0].length
    if (offset < match.index || offset > stop) continue
    const pair = match[1] === 'begin'
      ? matchingEnd(state.doc, line.number, stop, match[2])
      : matchingBegin(state.doc, line.number, match.index, match[2])
    const own = { from: line.from + match.index, to: line.from + stop }
    if (!pair) return [own]
    const other = { from: pair.line.from + pair.index, to: pair.line.from + pair.index + pair.length }
    return own.from < other.from ? [own, other] : [other, own]
  }
  return null
}

const tagMark = Decoration.mark({ class: 'cm-matchingTag' })
const tagMissing = Decoration.mark({ class: 'cm-missingTag' })

const pairMarks = EditorView.decorations.compute(['doc', 'selection'], (state) => {
  const range = state.selection.main
  const pair = range.empty ? pairAt(state, range.head) : null
  if (!pair) return Decoration.none
  const mark = pair.length === 2 ? tagMark : tagMissing
  return Decoration.set(pair.map((part) => mark.range(part.from, part.to)))
})

function indentOf(state: EditorState, text: string) {
  const unit = getIndentUnit(state)
  const lead = /^[ \t]*/.exec(text)![0]
  let width = 0
  for (const c of lead) width += c === '\t' ? unit : 1
  return width
}

const indentation = indentService.of((context, pos) => {
  const { state } = context
  const current = state.doc.lineAt(pos)
  let number = current.number - 1
  while (number >= 1 && state.doc.line(number).text.trim() === '') number--
  if (number < 1) return 0
  const previous = withoutComment(state.doc.line(number).text)
  let width = indentOf(state, previous)
  const unit = getIndentUnit(state)
  let open = 0
  for (const match of previous.matchAll(token)) {
    if (match[2] === 'document') continue
    open += match[1] === 'begin' ? 1 : -1
  }
  const closing = /^\s*\\end\{([a-zA-Z*]+)\}/.exec(current.text)
  if (open > 0) width += unit
  if (closing && closing[1] !== 'document') width -= unit
  return Math.max(width, 0)
})

function endEnvironment(target: EditorView, name: string, head: number, indent: string) {
  const { state } = target
  const inner = name === 'document' ? `\n${indent}` : `\n${indent}${indentString(state, getIndentUnit(state))}`
  const body = listNames.has(name) ? '\\item ' : ''
  target.dispatch({
    changes: { from: head, insert: `${inner}${body}\n${indent}\\end{${name}}` },
    selection: EditorSelection.cursor(head + inner.length + body.length),
    scrollIntoView: true,
    userEvent: 'input',
  })
  return true
}

export const smartEnter: Command = (target) => {
  const { state } = target
  if (state.selection.ranges.length > 1) return false
  const range = state.selection.main
  if (!range.empty) return false
  const line = state.doc.lineAt(range.head)
  const before = line.text.slice(0, range.head - line.from)
  const indent = /^[ \t]*/.exec(line.text)![0]
  const name = /\\begin\{([a-zA-Z*]+)\}(?:\[[^\]]*\])?(?:\{[^}]*\})?$/.exec(before)?.[1]
  if (name) {
    const text = state.doc.toString()
    if (text.split(`\\end{${name}}`).length < text.split(`\\begin{${name}}`).length) {
      return endEnvironment(target, name, range.head, indent)
    }
  }
  const opener = /\\[[(]$/.exec(before)?.[0]
  if (opener && state.sliceDoc(range.head, range.head + 2) === closers[opener[1]]) {
    const inner = `\n${indent}${indentString(state, getIndentUnit(state))}`
    target.dispatch({
      changes: { from: range.head, insert: `${inner}\n${indent}` },
      selection: EditorSelection.cursor(range.head + inner.length),
      scrollIntoView: true,
      userEvent: 'input',
    })
    return true
  }
  const item = /^([ \t]*)\\item(?:\[[^\]]*\])?[ \t]*(.*)$/.exec(before)
  if (!item) return false
  if (item[2] === '' && line.text.slice(before.length).trim() === '') {
    target.dispatch({
      changes: { from: line.from, to: line.to },
      selection: EditorSelection.cursor(line.from),
      userEvent: 'delete',
    })
    return true
  }
  const insert = `\n${item[1]}\\item `
  target.dispatch({
    changes: { from: range.head, insert },
    selection: EditorSelection.cursor(range.head + insert.length),
    scrollIntoView: true,
    userEvent: 'input',
  })
  return true
}

function wrapped(state: EditorState, from: number, to: number, command: string) {
  const open = `\\${command}{`
  return state.sliceDoc(Math.max(0, from - open.length), from) === open && state.sliceDoc(to, to + 1) === '}'
}

function wrapCommand(text: string, math: string): Command {
  return (target) => {
    const { state } = target
    const source = state.doc.toString()
    const change = state.changeByRange((range) => {
      const command = placeAt(source, range.from).math ? math : text
      const open = `\\${command}{`
      if (wrapped(state, range.from, range.to, command)) {
        return {
          changes: [
            { from: range.from - open.length, to: range.from },
            { from: range.to, to: range.to + 1 },
          ],
          range: EditorSelection.range(range.from - open.length, range.to - open.length),
        }
      }
      return {
        changes: [
          { from: range.from, insert: open },
          { from: range.to, insert: '}' },
        ],
        range: EditorSelection.range(range.from + open.length, range.to + open.length),
      }
    })
    target.dispatch(state.update(change, { scrollIntoView: true, userEvent: 'input' }))
    return true
  }
}

export const bold = wrapCommand('textbf', 'mathbf')
export const italic = wrapCommand('textit', 'mathit')

export const deletePair: Command = (target) => {
  const { state } = target
  if (state.selection.ranges.length > 1) return false
  const range = state.selection.main
  if (!range.empty || range.head < 2) return false
  const before = state.sliceDoc(range.head - 2, range.head)
  if (before[0] !== '\\' || !closers[before[1]] || escaped(state, range.head - 2)) return false
  if (state.sliceDoc(range.head, range.head + 2) !== closers[before[1]]) return false
  target.dispatch({
    changes: { from: range.head - 2, to: range.head + 2 },
    selection: EditorSelection.cursor(range.head - 2),
    userEvent: 'delete.backward',
  })
  return true
}

const pairInput = EditorView.inputHandler.of((target, from, to, text) => {
  const { state } = target
  if (from !== to || state.selection.ranges.length > 1 || target.composing) return false
  const previous = state.sliceDoc(from - 1, from)
  const following = state.sliceDoc(to, to + 1)
  if (text === '$' && escaped(state, from)) {
    target.dispatch({ changes: { from, insert: text }, selection: EditorSelection.cursor(from + 1), userEvent: 'input.type' })
    return true
  }
  if (text === '$' && previous === '$' && following === '$' && state.sliceDoc(from - 2, from - 1) !== '$') {
    target.dispatch({ changes: { from, insert: '$$' }, selection: EditorSelection.cursor(from + 1), userEvent: 'input.type' })
    return true
  }
  if (!escaped(state, from)) return false
  const closer = closers[text]
  if (closer) {
    target.dispatch({ changes: { from, insert: text + closer }, selection: EditorSelection.cursor(from + 1), userEvent: 'input.type' })
    return true
  }
  if (openers[text] && state.sliceDoc(to, to + 2) === `\\${text}`) {
    target.dispatch({ changes: { from: from - 1, to: from }, selection: EditorSelection.cursor(from + 1), userEvent: 'input.type' })
    return true
  }
  return false
})

export const latexEditing: Extension = [
  indentUnit.of('  '),
  indentation,
  foldService.of(foldRange),
  pairMarks,
  Prec.high(pairInput),
]
