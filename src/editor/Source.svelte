<script lang="ts">
  import { untrack } from 'svelte'
  import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands'
  import {
    bracketMatching,
    defaultHighlightStyle,
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

  const palette = HighlightStyle.define([
    { tag: [tags.tagName, tags.keyword], color: 'var(--green)' },
    { tag: tags.comment, color: 'var(--quiet)' },
    { tag: [tags.bracket, tags.punctuation], color: 'var(--quiet)' },
    { tag: [tags.atom, tags.number], color: 'var(--code-number)' },
    { tag: [tags.string, tags.meta], color: 'var(--code-structure)' },
  ])

  const theme = EditorView.theme({
    '&': { height: '100%', backgroundColor: 'var(--sheet)', color: 'var(--ink)', fontSize: '15px' },
    '&.cm-focused': { outline: 'none' },
    '.cm-scroller': { fontFamily: "ui-monospace, 'SF Mono', Menlo, Consolas, monospace", lineHeight: '24px' },
    '.cm-content': { caretColor: 'var(--ink)', padding: '8px 0' },
    '.cm-cursor': { borderLeftColor: 'var(--ink)', borderLeftWidth: '2px' },
    '.cm-gutters': { backgroundColor: 'var(--paper)', color: 'var(--quiet)', border: 'none', borderRight: '1px solid var(--rule-strong)' },
    '.cm-activeLine': { backgroundColor: 'rgb(27 94 68 / 6%)' },
    '.cm-activeLineGutter': { backgroundColor: 'var(--rule)', color: 'var(--ink)' },
    '.cm-selectionBackground, &.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground': { backgroundColor: 'rgb(27 94 68 / 22%)' },
    '.cm-matchingBracket, &.cm-focused .cm-matchingBracket': { backgroundColor: 'var(--rule-strong)', color: 'var(--ink)' },
    '.cm-selectionMatch': { backgroundColor: 'var(--rule)' },
    '.cm-errorLine': { backgroundColor: 'rgb(169 30 20 / 7%)' },
    '.cm-errorMark': { backgroundColor: 'rgb(169 30 20 / 16%)', boxShadow: 'inset 0 -2px var(--red)' },
    '.cm-panels': { backgroundColor: 'var(--paper)', color: 'var(--ink)', borderColor: 'var(--ink)' },
    '.cm-textfield, .cm-button': { borderRadius: '0', backgroundImage: 'none' },
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
          syntaxHighlighting(palette),
          syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
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

<div class="source" bind:this={host}></div>

<style>
  .source {
    height: 100%;
    min-height: 0;
    overflow: hidden;
  }
</style>
