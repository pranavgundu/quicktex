<script lang="ts">
  import { untrack } from 'svelte'
  import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands'
  import {
    bracketMatching,
    HighlightStyle,
    StreamLanguage,
    syntaxHighlighting,
  } from '@codemirror/language'
  import { stex } from '@codemirror/legacy-modes/mode/stex'
  import { EditorSelection, EditorState, Prec, StateEffect, StateField } from '@codemirror/state'
  import {
    Decoration,
    drawSelection,
    EditorView,
    highlightActiveLine,
    highlightActiveLineGutter,
    keymap,
    lineNumbers,
  } from '@codemirror/view'
  import { closeBrackets, closeBracketsKeymap } from '@codemirror/autocomplete'
  import { highlightSelectionMatches, search, searchKeymap } from '@codemirror/search'
  import { tags } from '@lezer/highlight'

  let {
    value = $bindable(),
    errorAt = null,
  }: { value: string; errorAt?: number | null } = $props()

  let host: HTMLDivElement
  let view: EditorView | undefined

  const setError = StateEffect.define<number | null>()

  const errorMark = StateField.define({
    create: () => Decoration.none,
    update(marks, transaction) {
      let next = marks.map(transaction.changes)
      for (const effect of transaction.effects) {
        if (!effect.is(setError)) continue
        const at = effect.value
        if (at === null) {
          next = Decoration.none
          continue
        }
        const length = transaction.state.doc.length
        const from = Math.min(at, length)
        const line = transaction.state.doc.lineAt(from)
        const word = /^\\[a-zA-Z]+/.exec(line.text.slice(from - line.from))
        const to = Math.min(from + (word ? word[0].length : 1), line.to)
        const marker = Decoration.line({ class: 'cm-errorLine' }).range(line.from)
        next = from < to
          ? Decoration.set([marker, Decoration.mark({ class: 'cm-errorMark' }).range(from, to)])
          : Decoration.set([marker])
      }
      return next
    },
    provide: (field) => EditorView.decorations.from(field),
  })

  const tint = (color: string, amount: number) => `color-mix(in srgb, var(--color-${color}) ${amount}%, transparent)`

  const palette = HighlightStyle.define([
    { tag: [tags.tagName, tags.keyword], color: 'var(--color-green)' },
    { tag: tags.comment, color: 'var(--color-quiet)', fontStyle: 'italic' },
    { tag: [tags.bracket, tags.punctuation], color: 'var(--color-quiet)' },
    { tag: [tags.atom, tags.number], color: 'var(--color-code-number)' },
    { tag: [tags.string, tags.meta], color: 'var(--color-code-structure)' },
  ])

  const theme = EditorView.theme({
    '&': { height: '100%', backgroundColor: 'var(--color-sheet)', color: 'var(--color-ink)', fontSize: '15px' },
    '&.cm-focused': { outline: 'none' },
    '.cm-scroller': { fontFamily: 'var(--font-mono)', lineHeight: '24px' },
    '.cm-content': { caretColor: 'var(--color-ink)', padding: '8px 0' },
    '.cm-cursor, .cm-dropCursor': { borderLeftColor: 'var(--color-ink)', borderLeftWidth: '2px' },
    '.cm-gutters': {
      backgroundColor: 'var(--color-paper)',
      color: 'var(--color-quiet)',
      border: 'none',
      borderRight: '1px solid var(--color-rule)',
    },
    '.cm-activeLine': { backgroundColor: tint('green', 8) },
    '.cm-activeLineGutter': { backgroundColor: 'var(--color-rule)', color: 'var(--color-ink)' },
    '.cm-selectionBackground, &.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground': {
      backgroundColor: tint('green', 28),
    },
    '.cm-matchingBracket, &.cm-focused .cm-matchingBracket': {
      backgroundColor: 'var(--color-rule-strong)',
      color: 'var(--color-ink)',
    },
    '.cm-nonmatchingBracket, &.cm-focused .cm-nonmatchingBracket': { backgroundColor: tint('red', 24), color: 'var(--color-ink)' },
    '.cm-selectionMatch': { backgroundColor: 'var(--color-rule)' },
    '.cm-searchMatch': { backgroundColor: tint('code-number', 28), outline: 'none' },
    '.cm-searchMatch.cm-searchMatch-selected': { backgroundColor: tint('code-number', 56) },
    '.cm-errorLine': { backgroundColor: tint('red', 10) },
    '.cm-errorMark': { backgroundColor: tint('red', 20), boxShadow: 'inset 0 -2px var(--color-red)' },
    '.cm-panels': { backgroundColor: 'var(--color-paper)', color: 'var(--color-ink)' },
    '.cm-panels.cm-panels-top': { borderBottom: '1px solid var(--color-rule-strong)' },
    '.cm-panel.cm-search': { padding: '8px 16px', fontFamily: 'var(--font-serif)', fontSize: '14px' },
    '.cm-panel.cm-search label': { color: 'var(--color-quiet)', fontSize: '14px' },
    '.cm-panel.cm-search input[type=checkbox]': { accentColor: 'var(--color-green)' },
    '.cm-panel.cm-search [name=close]': { color: 'var(--color-ink)', fontSize: '20px', top: '8px', right: '16px' },
    '.cm-textfield': {
      backgroundColor: 'var(--color-sheet)',
      border: '1px solid var(--color-rule-strong)',
      borderRadius: 'var(--radius-control)',
      color: 'var(--color-ink)',
      fontSize: '14px',
    },
    '.cm-button, .cm-button:active': {
      backgroundColor: 'var(--color-sheet)',
      backgroundImage: 'none',
      border: '1px solid var(--color-rule-strong)',
      borderRadius: 'var(--radius-control)',
      color: 'var(--color-ink)',
      fontSize: '14px',
    },
    '.cm-button:hover': { borderColor: 'var(--color-ink)', backgroundColor: 'var(--color-paper)' },
  })

  function closeEnvironment(target: EditorView) {
    const { state } = target
    const range = state.selection.main
    if (!range.empty) return false
    const line = state.doc.lineAt(range.head)
    const before = line.text.slice(0, range.head - line.from)
    const name = /\\begin\{([a-zA-Z*]+)\}(?:\{[^}]*\})?$/.exec(before)?.[1]
    if (!name) return false
    const text = state.doc.toString()
    if (text.split(`\\end{${name}}`).length >= text.split(`\\begin{${name}}`).length) return false
    const indent = /^[ \t]*/.exec(line.text)![0]
    const inner = `\n${indent}  `
    target.dispatch({
      changes: { from: range.head, insert: `${inner}\n${indent}\\end{${name}}` },
      selection: EditorSelection.cursor(range.head + inner.length),
      scrollIntoView: true,
      userEvent: 'input',
    })
    return true
  }

  $effect(() => {
    const editor = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: untrack(() => value),
        extensions: [
          lineNumbers(),
          highlightActiveLine(),
          highlightActiveLineGutter(),
          history(),
          drawSelection(),
          bracketMatching(),
          closeBrackets(),
          search({ top: true }),
          highlightSelectionMatches(),
          EditorView.lineWrapping,
          StreamLanguage.define(stex),
          EditorState.languageData.of(() => [{ closeBrackets: { brackets: ['{'] } }]),
          EditorState.phrases.of({
            next: 'Next',
            previous: 'Previous',
            all: 'All',
            'match case': 'Match case',
            regexp: 'Regex',
            'by word': 'Whole word',
            replace: 'Replace',
            'replace all': 'Replace all',
          }),
          syntaxHighlighting(palette),
          theme,
          errorMark,
          Prec.high(keymap.of([{ key: 'Enter', run: closeEnvironment }])),
          keymap.of([...closeBracketsKeymap, ...defaultKeymap, ...searchKeymap, ...historyKeymap, indentWithTab]),
          EditorView.contentAttributes.of({ 'aria-label': 'LaTeX source', spellcheck: 'false', autocapitalize: 'off' }),
          EditorView.updateListener.of((update) => {
            if (update.docChanged) value = update.state.doc.toString()
          }),
        ],
      }),
    })
    view = editor
    return () => {
      view = undefined
      editor.destroy()
    }
  })

  $effect(() => {
    view?.dispatch({ effects: setError.of(errorAt) })
  })

  export function reveal(at: number) {
    if (!view) return
    view.dispatch({ selection: { anchor: Math.min(at, view.state.doc.length) }, scrollIntoView: true })
    view.focus()
  }
</script>

<div class="h-full min-h-0 overflow-hidden" bind:this={host}></div>
