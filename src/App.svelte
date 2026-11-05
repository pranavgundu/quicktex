<script lang="ts">
  import { QuicktexError, render } from '../wasm-core/js/quicktex.js'
  import Button from './lib/Button.svelte'
  import Segmented from './lib/Segmented.svelte'
  import ThemeToggle from './lib/ThemeToggle.svelte'
  import Typeset from './lib/Typeset.svelte'
  import { check, code, edit, image } from './lib/icons'
  import { charIndex } from './lib/latex'
  import { svgToPng } from './lib/png'
  import { remember, remembered } from './lib/storage'
  import { measure, timeTex, timeText } from './lib/timing'

  const layoutKey = 'quicktex-layout'
  const firstFormula = String.raw`\int_{-\infty}^{\infty} e^{-x^2}\,dx = \sqrt{\pi}`

  const cell = 24
  const frame = 1
  const stagePadding = 48
  const timingDelay = 150

  let source = $state(firstFormula)
  let layout = $state<'display' | 'inline'>(remembered(layoutKey) === 'inline' ? 'inline' : 'display')
  let svg = $state('')
  let problem = $state('')
  let micros = $state(0)
  let notice = $state({ target: '', text: '' })
  let pageWidth = $state(0)
  let stageHeight = $state(0)
  let copyTimer = 0
  let timed = false

  const empty = $derived(source.trim() === '')
  const display = $derived(layout === 'display')

  function wholeCells(space: number) {
    return Math.max(Math.floor((space - 2 * frame + 1) / cell), 1) * cell - 1
  }

  const gutter = $derived(pageWidth <= 640 ? 16 : 24)
  const sheetWidth = $derived(wholeCells(pageWidth - 2 * gutter))
  const sheetHeight = $derived(wholeCells(stageHeight))
  const columnWidth = $derived(sheetWidth + 2 * frame)
  const columnLeft = $derived(Math.floor((pageWidth - columnWidth) / 2))
  const spare = $derived(Math.max(stageHeight - sheetHeight - 2 * frame, 0))

  $effect(() => remember(layoutKey, layout))

  $effect(() => {
    const tex = source
    const mode = display
    try {
      svg = render(tex, mode)
      problem = ''
    } catch (error) {
      if (!(error instanceof QuicktexError)) throw error
      const message = error.message[0].toUpperCase() + error.message.slice(1)
      problem = `${message} at character ${charIndex(tex, error.position) + 1}`
      return
    }
    if (empty) return
    if (!timed) {
      timed = true
      micros = measure(() => render(tex, mode))
      return
    }
    const timer = setTimeout(() => (micros = measure(() => render(tex, mode))), timingDelay)
    return () => clearTimeout(timer)
  })

  const fit = $derived.by(() => {
    const size = /width="([\d.]+)em" height="([\d.]+)em"/.exec(svg)
    if (!size || !pageWidth) return 48
    const across = (sheetWidth - stagePadding) / Math.max(Number(size[1]), 0.1)
    const down = (sheetHeight - stagePadding) / Math.max(Number(size[2]), 0.1)
    return Math.max(16, Math.min(64, across, down))
  })

  const weight = $derived(`${(svg.length / 1024).toFixed(1)} KB`)

  function copyLabel(target: string, idle: string) {
    return notice.target === target ? notice.text : idle
  }

  function copyIcon(target: string, idle: string) {
    return notice.target === target && notice.text === 'Copied' ? check : idle
  }

  async function copy(target: string, write: () => Promise<void>) {
    clearTimeout(copyTimer)
    notice = { target, text: 'Copying' }
    try {
      await write()
      notice = { target, text: 'Copied' }
    } catch {
      notice = { target, text: 'Copy blocked' }
    }
    copyTimer = window.setTimeout(() => (notice = { target: '', text: '' }), 1600)
  }

  const copySvg = () => copy('svg', () => navigator.clipboard.writeText(svg))
  const copyImage = () =>
    copy('image', () => navigator.clipboard.write([new ClipboardItem({ 'image/png': svgToPng(svg) })]))
</script>

<div class="h-full" bind:clientWidth={pageWidth}>
  <main
    class="grid min-h-full grid-rows-[auto_minmax(240px,1fr)_auto]"
    style:width="{columnWidth}px"
    style:margin-left="{columnLeft}px"
  >
    <header class="flex items-center gap-4 py-4">
      <h1 class="flex-1 text-[32px] leading-none font-normal">
        <Typeset text="quicktex" tex={String.raw`\textbf{quicktex}`} />
      </h1>
      <p class="hidden text-sm text-quiet sm:block">
        <Typeset
          text={`Rendered in ${timeText(micros)}`}
          tex={timeTex(micros)}
        />
      </p>
      <p class="hidden text-sm text-quiet sm:block">
        <Typeset text={`${weight} of SVG`} tex={`\\text{${weight}}`} />
      </p>
      <ThemeToggle />
      <Button href="./editor/" icon={edit} label="Open editor" />
    </header>

    <div class="relative" bind:clientHeight={stageHeight}>
      <section
        class={[
          'graph-paper absolute inset-x-0 top-0 flex overflow-auto rounded-control border border-rule-strong p-6',
          problem !== '' && 'text-stale',
        ]}
        aria-label="Rendered formula"
        style:height="{sheetHeight + 2 * frame}px"
      >
        {#if empty}
          <div class="m-auto text-2xl text-green"><Typeset text="Type LaTeX below and it appears here" /></div>
        {:else}
          <div class="m-auto leading-none" style:font-size="{fit}px">{@html svg}</div>
        {/if}
      </section>
    </div>

    <section class="pt-4 pb-6" style:translate="0 -{spare}px">
      <div class="rounded-control border border-rule-strong bg-sheet focus-within:border-green">
        <label class="sr-only" for="source">LaTeX source</label>
        <textarea
          id="source"
          class="block w-full resize-y rounded-t-control bg-transparent px-4 py-2 font-mono text-base leading-6 outline-none"
          rows="3"
          spellcheck="false"
          autocapitalize="off"
          autocomplete="off"
          bind:value={source}
        ></textarea>
        <div class="flex flex-wrap items-center gap-2 border-t border-rule p-2">
          <Segmented
            class="max-sm:basis-full"
            legend="Layout"
            name="layout"
            bind:value={layout}
            options={[
              { value: 'display', label: 'Display' },
              { value: 'inline', label: 'Inline' },
            ]}
          />
          <p class="min-w-0 flex-1 px-2 text-red max-sm:order-last max-sm:basis-full" role="status">
            {#if problem}<Typeset text={problem} />{/if}
          </p>
          <Button
            primary
            class="min-w-36 max-sm:flex-1"
            disabled={empty}
            icon={copyIcon('image', image)}
            label={copyLabel('image', 'Copy image')}
            onclick={copyImage}
          />
          <Button
            class="min-w-36 max-sm:flex-1"
            disabled={empty}
            icon={copyIcon('svg', code)}
            label={copyLabel('svg', 'Copy SVG')}
            onclick={copySvg}
          />
        </div>
      </div>
    </section>
  </main>
</div>
