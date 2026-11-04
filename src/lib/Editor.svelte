<script lang="ts">
  import { bracePairs, markError, tokenize } from './latex'

  let {
    id,
    value = $bindable(),
    errorAt = null,
  }: { id: string; value: string; errorAt?: number | null } = $props()

  const wrappers: Record<string, string> = { '{': '}', '[': ']', '(': ')' }

  let area: HTMLTextAreaElement
  let caret = $state(-1)

  const tokens = $derived(tokenize(value))
  const pairs = $derived(bracePairs(tokens))
  const pieces = $derived(markError(tokens, errorAt))
  const brace = $derived(pairs.has(caret - 1) ? caret - 1 : pairs.has(caret) ? caret : -1)
  const partner = $derived(pairs.get(brace) ?? -1)

  function trackCaret() {
    const inside = document.activeElement === area && area.selectionStart === area.selectionEnd
    caret = inside ? area.selectionStart : -1
  }

  $effect(() => {
    document.addEventListener('selectionchange', trackCaret)
    return () => document.removeEventListener('selectionchange', trackCaret)
  })

  function replaceSelection(text: string) {
    if (text ? document.execCommand('insertText', false, text) : document.execCommand('delete')) return
    area.setRangeText(text, area.selectionStart, area.selectionEnd, 'end')
    area.dispatchEvent(new Event('input', { bubbles: true }))
  }

  function insert(text: string, selectFrom: number, selectTo = selectFrom) {
    const start = area.selectionStart
    replaceSelection(text)
    area.setSelectionRange(start + selectFrom, start + selectTo)
  }

  function newline(text: string, start: number, end: number) {
    const line = text.slice(text.lastIndexOf('\n', start - 1) + 1, start)
    const indent = /^[ \t]*/.exec(line)![0]
    const name = /\\begin\{([a-zA-Z*]+)\}(?:\{[^}]*\})?$/.exec(line)?.[1]
    if (name && !text.slice(end).includes(`\\end{${name}}`)) {
      const inner = `\n${indent}  `
      insert(`${inner}\n${indent}\\end{${name}}`, inner.length)
      return true
    }
    if (!indent) return false
    insert(`\n${indent}`, indent.length + 1)
    return true
  }

  function handled(event: KeyboardEvent) {
    const { selectionStart: start, selectionEnd: end, value: text } = area
    const selected = text.slice(start, end)
    const before = text[start - 1]
    const after = text[end]

    if (event.key === 'Enter') return !event.shiftKey && newline(text, start, end)

    if (event.key in wrappers && before !== '\\') {
      const close = wrappers[event.key]
      if (selected) {
        insert(event.key + selected + close, 1, selected.length + 1)
        return true
      }
      if (event.key !== '{') return false
      if (after !== undefined && !/[\s)}\]&^_]/.test(after)) return false
      insert('{}', 1)
      return true
    }

    if (event.key === '}' && !selected && after === '}' && pairs.has(start)) {
      area.setSelectionRange(start + 1, start + 1)
      return true
    }

    if (event.key === 'Backspace' && !selected && before === '{' && after === '}' && text[start - 2] !== '\\') {
      area.setSelectionRange(start - 1, start + 1)
      replaceSelection('')
      return true
    }

    return false
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.isComposing || event.metaKey || event.ctrlKey || event.altKey) return
    if (handled(event)) event.preventDefault()
  }
</script>

<div class="editor">
  <pre aria-hidden="true">{#each pieces as piece (piece.start)}<span
        class={piece.kind}
        class:error={piece.error}
        class:match={piece.start === brace || piece.start === partner}>{piece.text}</span
      >{/each}{'\n'}</pre>
  <textarea
    {id}
    bind:this={area}
    bind:value
    {onkeydown}
    onblur={trackCaret}
    placeholder={String.raw`Type LaTeX, like \frac{a}{b}`}
    spellcheck="false"
    autocapitalize="off"
    autocomplete="off"
  ></textarea>
</div>

<style>
  .editor {
    grid-column: 1 / -1;
    display: grid;
    min-height: 92px;
    max-height: 260px;
    overflow: auto;
    border: 2px solid var(--ink);
    background: var(--sheet);
    scroll-padding-block: var(--s2);
  }

  .editor:has(textarea:focus-visible) {
    outline: 2px solid var(--green);
    outline-offset: -2px;
  }

  pre {
    background-image: linear-gradient(transparent 23px, var(--rule) 23px);
    background-size: 100% 24px;
    background-origin: content-box;
    background-clip: content-box;
  }

  pre,
  textarea {
    grid-area: 1 / 1;
    min-width: 0;
    margin: 0;
    padding: var(--s2) var(--s3);
    border: 0;
    font: 16px/24px ui-monospace, 'SF Mono', Menlo, Consolas, monospace;
    tab-size: 2;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  textarea {
    width: 100%;
    height: 100%;
    overflow: hidden;
    outline: none;
    border-radius: 0;
    background: transparent;
    color: transparent;
    -webkit-text-fill-color: transparent;
    caret-color: var(--ink);
    resize: none;
  }

  textarea::selection {
    background: rgb(27 94 68 / 22%);
  }

  textarea::placeholder {
    color: var(--quiet);
    -webkit-text-fill-color: var(--quiet);
  }

  .command {
    color: var(--green);
  }

  .environment {
    color: var(--code-number);
  }

  .brace,
  .comment {
    color: var(--quiet);
  }

  .structure {
    color: var(--code-structure);
  }

  .number {
    color: var(--code-number);
  }

  .match {
    background: var(--rule-strong);
    color: var(--ink);
  }

  .error {
    background: rgb(169 30 20 / 14%);
    box-shadow: inset 0 -2px var(--red);
  }
</style>
