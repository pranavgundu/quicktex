<script lang="ts">
  import { untrack } from 'svelte'
  import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands'
  import {
    bracketMatching,
    codeFolding,
    foldGutter,
    foldKeymap,
    HighlightStyle,
    indentOnInput,
    StreamLanguage,
    syntaxHighlighting,
  } from '@codemirror/language'
  import { stex } from '@codemirror/legacy-modes/mode/stex'
  import { EditorState, Prec, StateEffect, StateField } from '@codemirror/state'
  import {
    crosshairCursor,
    Decoration,
    drawSelection,
    dropCursor,
    EditorView,
    highlightActiveLine,
    highlightActiveLineGutter,
    keymap,
    lineNumbers,
    rectangularSelection,
  } from '@codemirror/view'
  import {
    acceptCompletion,
    autocompletion,
    closeBrackets,
    closeBracketsKeymap,
    closeCompletion,
    completionKeymap,
    completionStatus,
    selectedCompletion,
  } from '@codemirror/autocomplete'
  import { highlightSelectionMatches, search, searchKeymap } from '@codemirror/search'
  import { tags } from '@lezer/highlight'
  import { chevronDown, chevronRight } from '../lib/icons'
  import { isPlain, latexCompletions, renderPreview } from './completions'
  import { bold, deletePair, italic, latexEditing, smartEnter } from './editing'

  let {
    value = $bindable(),
    errorAt = null,
    onsave,
    oncursor,
  }: {
    value: string
    errorAt?: number | null
    onsave?: () => void
    oncursor?: (at: number) => void
  } = $props()

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
    '.cm-matchingTag': { backgroundColor: 'var(--color-rule-strong)' },
    '.cm-missingTag': { backgroundColor: tint('red', 24) },
    '.cm-foldGutter .cm-gutterElement': { color: 'var(--color-quiet)', cursor: 'pointer', display: 'flex', alignItems: 'center' },
    '.cm-foldGutter .cm-gutterElement:hover': { color: 'var(--color-ink)' },
    '.cm-foldGutter svg': { width: '16px', height: '16px', fill: 'currentColor' },
    '.cm-foldPlaceholder': {
      backgroundColor: 'var(--color-rule)',
      border: 'none',
      borderRadius: 'var(--radius-control)',
      color: 'var(--color-ink)',
      margin: '0 8px',
      padding: '0 8px',
    },
    '.cm-snippetField': { backgroundColor: tint('green', 16) },
    '.cm-snippetFieldPosition': { borderLeft: '2px solid var(--color-green)' },
    '.cm-tooltip': {
      backgroundColor: 'var(--color-sheet)',
      border: '1px solid var(--color-rule-strong)',
      borderRadius: 'var(--radius-control)',
      color: 'var(--color-ink)',
      overflow: 'hidden',
    },
    '.cm-tooltip.cm-tooltip-autocomplete > ul': { fontFamily: 'var(--font-mono)', fontSize: '14px', maxHeight: '320px', minWidth: '256px' },
    '.cm-tooltip.cm-tooltip-autocomplete > ul > li': {
      alignItems: 'center',
      display: 'flex',
      gap: '16px',
      lineHeight: '24px',
      minHeight: '32px',
      padding: '4px 8px',
    },
    '.cm-tooltip.cm-tooltip-autocomplete > ul > li[aria-selected]': { backgroundColor: tint('green', 20), color: 'var(--color-ink)' },
    '.cm-completionLabel': { flex: '1' },
    '.cm-completionMatchedText': { color: 'var(--color-green)', fontWeight: '700', textDecoration: 'none' },
    '.cm-completionDetail': { color: 'var(--color-quiet)', fontFamily: 'var(--font-serif)', fontStyle: 'normal', marginLeft: '0' },
    '.cm-completionPreview': { color: 'var(--color-ink)', flex: 'none', fontSize: '16px', lineHeight: '24px' },
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

  function foldMarker(open: boolean) {
    const node = document.createElement('span')
    node.title = open ? 'Fold' : 'Unfold'
    node.innerHTML = open ? chevronDown : chevronRight
    return node
  }

  function enterAfterFullWord(target: EditorView) {
    if (completionStatus(target.state) !== 'active') return false
    const chosen = selectedCompletion(target.state)
    if (!chosen || !isPlain(chosen)) return false
    const head = target.state.selection.main.head
    if (target.state.sliceDoc(Math.max(0, head - chosen.label.length), head) === chosen.label) closeCompletion(target)
    return false
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
          dropCursor(),
          rectangularSelection(),
          crosshairCursor(),
          EditorState.allowMultipleSelections.of(true),
          indentOnInput(),
          bracketMatching(),
          closeBrackets(),
          codeFolding({ placeholderText: 'folded' }),
          foldGutter({ markerDOM: foldMarker }),
          Prec.highest(
            keymap.of([
              { key: 'Enter', run: enterAfterFullWord },
              { key: 'Tab', run: acceptCompletion },
            ]),
          ),
          autocompletion({
            override: [latexCompletions],
            icons: false,
            maxRenderedOptions: 40,
            addToOptions: [{ render: renderPreview, position: 90 }],
          }),
          search({ top: true }),
          highlightSelectionMatches(),
          EditorView.lineWrapping,
          StreamLanguage.define(stex),
          EditorState.languageData.of(() => [
            {
              closeBrackets: { brackets: ['{', '$'], before: ')]}$:;>,' },
              indentOnInput: /^\s*\\end\{[a-zA-Z*]+\}$/,
            },
          ]),
          latexEditing,
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
          Prec.high(
            keymap.of([
              { key: 'Enter', run: smartEnter },
              { key: 'Backspace', run: deletePair },
              { key: 'Mod-b', run: bold },
              { key: 'Mod-i', run: italic },
              { key: 'Mod-s', run: () => (onsave?.(), true) },
            ]),
          ),
          keymap.of([
            ...closeBracketsKeymap,
            ...completionKeymap,
            ...defaultKeymap,
            ...searchKeymap,
            ...historyKeymap,
            ...foldKeymap,
            indentWithTab,
          ]),
          EditorView.contentAttributes.of({ 'aria-label': 'LaTeX source', spellcheck: 'false', autocapitalize: 'off' }),
          EditorView.updateListener.of((update) => {
            if (update.docChanged) value = update.state.doc.toString()
            if (update.selectionSet || update.docChanged) oncursor?.(update.state.selection.main.head)
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
    const text = value
    if (view && view.state.doc.toString() !== text) {
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text } })
    }
  })

  $effect(() => {
    view?.dispatch({ effects: setError.of(errorAt) })
  })

  export function jump(at: number) {
    if (!view) return
    const anchor = Math.min(at, view.state.doc.length)
    view.dispatch({ selection: { anchor }, effects: EditorView.scrollIntoView(anchor, { y: 'start', yMargin: 32 }) })
    view.focus()
  }

  export function reveal(at: number) {
    if (!view) return
    view.dispatch({ selection: { anchor: Math.min(at, view.state.doc.length) }, scrollIntoView: true })
    view.focus()
  }
</script>

<div class="h-full min-h-0 overflow-hidden" bind:this={host}></div>
