<script lang="ts">
  import { documentPdf, QuicktexError, renderDocument } from '../../wasm-core/js/quicktex.js'
  import Button from '../lib/Button.svelte'
  import Icon from '../lib/Icon.svelte'
  import Segmented from '../lib/Segmented.svelte'
  import ThemeToggle from '../lib/ThemeToggle.svelte'
  import Typeset from '../lib/Typeset.svelte'
  import { back, download as downloadIcon, warning } from '../lib/icons'
  import { charIndex } from '../lib/latex'
  import { remember, remembered } from '../lib/storage'
  import Pages from './Pages.svelte'
  import Source from './Source.svelte'
  import { starter } from './starter'

  const storageKey = 'quicktex-document'
  const saveDelay = 400
  const splitStorageKey = 'quicktex-split'

  let source = $state(remembered(storageKey) ?? starter)
  let pages = $state(0)
  let version = $state(0)
  let problem = $state('')
  let problemAt = $state<number | null>(null)
  let millis = $state(0)
  let split = $state(Number(remembered(splitStorageKey)) || 50)
  let showing = $state<'source' | 'pages'>('source')
  let workspace: HTMLElement
  let editor: Source

  const elapsed = $derived(millis < 0.1 ? 'under 0.1' : millis < 10 ? millis.toFixed(1) : Math.round(millis))

  function compile(tex: string) {
    const start = performance.now()
    try {
      pages = renderDocument(tex)
      millis = performance.now() - start
      version++
      problem = ''
      problemAt = null
    } catch (error) {
      if (!(error instanceof QuicktexError)) throw error
      problemAt = charIndex(tex, error.position)
      const line = tex.slice(0, problemAt).split('\n').length
      const message = error.message[0].toUpperCase() + error.message.slice(1)
      problem = `${message} on line ${line}`
    }
  }

  $effect(() => {
    const tex = source
    const frame = requestAnimationFrame(() => compile(tex))
    const save = setTimeout(() => remember(storageKey, tex), saveDelay)
    return () => {
      cancelAnimationFrame(frame)
      clearTimeout(save)
    }
  })

  function download() {
    const url = URL.createObjectURL(new Blob([documentPdf() as BlobPart], { type: 'application/pdf' }))
    const link = document.createElement('a')
    link.href = url
    link.download = 'document.pdf'
    link.click()
    URL.revokeObjectURL(url)
  }

  function resize(event: PointerEvent) {
    const handle = event.currentTarget as HTMLElement
    handle.setPointerCapture(event.pointerId)
    const move = (next: PointerEvent) => {
      const box = workspace.getBoundingClientRect()
      split = Math.min(80, Math.max(20, ((next.clientX - box.left) / box.width) * 100))
    }
    const stop = () => {
      handle.removeEventListener('pointermove', move)
      remember(splitStorageKey, String(split))
    }
    handle.addEventListener('pointermove', move)
    handle.addEventListener('pointerup', stop, { once: true })
  }

  function nudge(event: KeyboardEvent) {
    const change = event.key === 'ArrowLeft' ? -2 : event.key === 'ArrowRight' ? 2 : 0
    if (change === 0) return
    event.preventDefault()
    split = Math.min(80, Math.max(20, split + change))
    remember(splitStorageKey, String(split))
  }
</script>

<div class="grid h-full grid-cols-[minmax(0,1fr)] grid-rows-[auto_minmax(0,1fr)]">
  <header class="flex flex-wrap items-center gap-x-4 gap-y-2 border-b border-rule-strong px-4 py-2">
    <div class="flex-1">
      <a
        class="inline-flex items-center gap-2 rounded-control text-[24px] leading-none text-ink hover:text-green"
        href="../"
        aria-label="Back to the formula page"
      >
        <Icon icon={back} />
        <Typeset text="quicktex" tex={String.raw`\textbf{quicktex}`} />
      </a>
    </div>
    <ThemeToggle />
    <Button primary disabled={pages === 0} icon={downloadIcon} label="Download PDF" short="PDF" onclick={download} />
    <Segmented
      class="order-last basis-full md:hidden"
      legend="View"
      name="view"
      bind:value={showing}
      options={[
        { value: 'source', label: 'Source' },
        { value: 'pages', label: 'Pages' },
      ]}
    />
  </header>

  <main
    class="grid min-h-0 grid-cols-[minmax(0,1fr)] md:grid-cols-[minmax(0,var(--split))_8px_minmax(0,1fr)]"
    style:--split="{split}%"
    bind:this={workspace}
  >
    <section
      class={['grid min-h-0 min-w-0 grid-rows-[minmax(0,1fr)_40px]', showing === 'pages' && 'max-md:hidden']}
      aria-label="LaTeX source"
    >
      <div class="min-h-0"><Source bind:this={editor} bind:value={source} errorAt={problemAt} /></div>
      <div class="flex min-w-0 items-center border-t border-rule-strong bg-paper text-sm text-quiet" role="status">
        {#if problem}
          <button
            class="flex size-full items-center gap-2 bg-sheet px-4 text-left text-red shadow-[inset_0_2px_0_var(--color-red)] hover:bg-paper"
            type="button"
            onclick={() => problemAt !== null && editor.reveal(problemAt)}
          >
            <Icon icon={warning} />
            <span class="min-w-0 flex-1 truncate"><Typeset text={problem} /></span>
            {#if pages > 0}
              <span class="shrink-0 text-quiet max-lg:hidden"><Typeset text="Pages show the last good version" /></span>
            {/if}
          </button>
        {:else}
          <p class="px-4"><Typeset text={`${pages} ${pages === 1 ? 'page' : 'pages'}, ${elapsed} ms`} /></p>
        {/if}
      </div>
    </section>
    <button
      class="cursor-col-resize touch-none border-x border-rule-strong bg-paper hover:bg-rule-strong max-md:hidden"
      type="button"
      aria-label="Resize panes, use the left and right arrow keys"
      onpointerdown={resize}
      onkeydown={nudge}
    ></button>
    <section
      class={['min-h-0 min-w-0', showing === 'source' && 'max-md:hidden']}
      aria-label="Pages"
    >
      <Pages count={pages} {version} />
    </section>
  </main>
</div>
