<script lang="ts">
  import { documentPdf, QuicktexError, renderDocument } from '../../wasm-core/js/quicktex.js'
  import { charIndex } from '../lib/latex'
  import Pages from './Pages.svelte'
  import Source from './Source.svelte'
  import { starter } from './starter'

  const storageKey = 'quicktex-document'
  const saveDelay = 400
  const splitStorageKey = 'quicktex-split'

  function remembered(key: string) {
    try {
      return localStorage.getItem(key)
    } catch {
      return null
    }
  }

  function remember(key: string, value: string) {
    try {
      localStorage.setItem(key, value)
    } catch {
      return
    }
  }

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

<div class="shell">
  <header>
    <a class="home" href="../">quicktex</a>
    <p class="status">
      {pages} {pages === 1 ? 'page' : 'pages'}, {elapsed} ms
    </p>
    <div class="switch">
      <button type="button" class:on={showing === 'source'} onclick={() => (showing = 'source')}>Source</button>
      <button type="button" class:on={showing === 'pages'} onclick={() => (showing = 'pages')}>Pages</button>
    </div>
    <button class="download" type="button" disabled={pages === 0} onclick={download}>Download PDF</button>
  </header>

  <main class="workspace" class:reading={showing === 'pages'} style:--split="{split}%" bind:this={workspace}>
    <section class="pane write" aria-label="LaTeX source">
      <div class="editor"><Source bind:this={editor} bind:value={source} errorAt={problemAt} /></div>
      <div class="notice" role="status">
        {#if problem}
          <button class="problem" type="button" onclick={() => problemAt !== null && editor.reveal(problemAt)}>{problem}</button>
        {/if}
      </div>
    </section>
    <button
      class="handle"
      type="button"
      aria-label="Resize panes, use the left and right arrow keys"
      onpointerdown={resize}
      onkeydown={nudge}
    ></button>
    <section class="pane read" class:stale={problem !== ''} aria-label="Pages">
      <Pages count={pages} {version} />
    </section>
  </main>
</div>

<style>
  .shell {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    grid-template-columns: minmax(0, 1fr);
    height: 100%;
    font-size: 15px;
  }

  header {
    display: flex;
    gap: var(--s3);
    align-items: center;
    padding: var(--s2) var(--s3);
    border-bottom: 2px solid var(--ink);
  }

  .home {
    color: inherit;
    font-size: 20px;
    font-weight: 700;
    text-decoration: none;
  }

  .home:hover {
    color: var(--green);
  }

  .status {
    flex: 1;
    margin: 0;
    color: var(--quiet);
    font-size: 14px;
    white-space: nowrap;
  }

  .switch {
    display: none;
    border: 2px solid var(--ink);
  }

  .switch button {
    min-height: 36px;
    padding: 0 var(--s2);
    border: 0;
    background: transparent;
    cursor: pointer;
  }

  .switch .on {
    background: var(--ink);
    color: var(--sheet);
  }

  .download {
    min-height: 40px;
    padding: 0 var(--s3);
    border: 0;
    background: var(--green);
    box-shadow: 0 var(--s1) 0 var(--green-deep);
    color: var(--sheet);
    cursor: pointer;
  }

  .download:hover:enabled {
    background: var(--green-deep);
    box-shadow: 0 var(--s1) 0 var(--ink);
  }

  .download:active:enabled {
    box-shadow: none;
    transform: translateY(var(--s1));
  }

  .download:disabled {
    background: var(--stale);
    box-shadow: none;
    cursor: not-allowed;
  }

  .workspace {
    display: grid;
    grid-template-columns: minmax(0, var(--split)) 8px minmax(0, 1fr);
    min-height: 0;
  }

  .pane {
    min-width: 0;
    min-height: 0;
  }

  .write {
    display: grid;
    grid-template-rows: minmax(0, 1fr) 40px;
  }

  .editor {
    min-height: 0;
  }

  .notice {
    min-width: 0;
    border-top: 1px solid var(--rule-strong);
    background: var(--paper);
  }

  .problem {
    width: 100%;
    height: 100%;
    padding: 0 var(--s3);
    overflow: hidden;
    border: 0;
    box-shadow: inset 0 2px var(--red);
    background: var(--sheet);
    color: var(--red);
    text-align: left;
    text-overflow: ellipsis;
    white-space: nowrap;
    cursor: pointer;
  }

  .problem:hover {
    background: var(--paper);
  }

  .handle {
    padding: 0;
    border: 0;
    border-inline: 1px solid var(--ink);
    background: var(--paper);
    cursor: col-resize;
    touch-action: none;
  }

  .handle:hover {
    background: var(--rule-strong);
  }

  .stale {
    opacity: 0.6;
  }

  @media (max-width: 760px) {
    header {
      flex-wrap: wrap;
      gap: var(--s2) var(--s3);
    }

    .home {
      flex: 1;
    }

    .status {
      display: none;
    }

    .switch {
      display: flex;
      flex-basis: 100%;
      order: 1;
    }

    .switch button {
      flex: 1;
    }

    .workspace {
      grid-template-columns: minmax(0, 1fr);
    }

    .handle,
    .workspace.reading .write,
    .workspace:not(.reading) .read {
      display: none;
    }
  }
</style>
