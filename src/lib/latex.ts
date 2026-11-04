export type Kind = 'plain' | 'command' | 'brace' | 'structure' | 'number' | 'comment' | 'environment'

export type Token = {
  kind: Kind
  text: string
  start: number
  error?: boolean
}

const pattern = /(%[^\n]*)|(\\\\)|(\\(?:[a-zA-Z]+\*?|[^a-zA-Z\n]?))|([{}[\]])|([\^_&])|(\d+(?:\.\d+)?)/g

const kinds: Kind[] = ['comment', 'structure', 'command', 'brace', 'structure', 'number']

export function tokenize(text: string) {
  const tokens: Token[] = []
  let at = 0

  const plain = (end: number) => {
    if (end === at) return
    const [command, brace] = tokens.slice(-2)
    const named = /^\\(begin|end)$/.test(command?.text ?? '') && brace?.text === '{' && tokens.length > 1
    tokens.push({ kind: named ? 'environment' : 'plain', text: text.slice(at, end), start: at })
  }

  for (const match of text.matchAll(pattern)) {
    plain(match.index)
    const group = match.slice(1).findIndex((part) => part !== undefined)
    tokens.push({ kind: kinds[group], text: match[0], start: match.index })
    at = match.index + match[0].length
  }
  plain(text.length)
  return tokens
}

export function bracePairs(tokens: Token[]) {
  const pairs = new Map<number, number>()
  const open: number[] = []
  for (const token of tokens) {
    if (token.text === '{') open.push(token.start)
    if (token.text !== '}') continue
    const start = open.pop()
    if (start === undefined) continue
    pairs.set(start, token.start)
    pairs.set(token.start, start)
  }
  return pairs
}

export function markError(tokens: Token[], at: number | null) {
  if (at === null) return tokens
  const marked: Token[] = []
  let found = false
  for (const token of tokens) {
    const offset = at - token.start
    if (found || offset < 0 || offset >= token.text.length) {
      marked.push(token)
      continue
    }
    found = true
    if (token.kind !== 'plain') {
      marked.push({ ...token, error: true })
      continue
    }
    const character = String.fromCodePoint(token.text.codePointAt(offset)!)
    const after = offset + character.length
    if (offset > 0) marked.push({ ...token, text: token.text.slice(0, offset) })
    marked.push({ ...token, text: character, start: at, error: true })
    if (after < token.text.length) marked.push({ ...token, text: token.text.slice(after), start: token.start + after })
  }
  if (!found) marked.push({ kind: 'plain', text: ' ', start: at, error: true })
  return marked
}

export function charIndex(text: string, bytes: number) {
  let seen = 0
  let index = 0
  while (index < text.length && seen < bytes) {
    const point = text.codePointAt(index)!
    seen += point < 0x80 ? 1 : point < 0x800 ? 2 : point < 0x10000 ? 3 : 4
    index += point > 0xffff ? 2 : 1
  }
  return index
}
