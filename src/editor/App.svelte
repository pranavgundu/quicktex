<script lang="ts">
  import type { ProjectFiles } from '../../wasm-core/js/quicktex.js'
  import { untrack } from 'svelte'
  import Button from '../lib/Button.svelte'
  import Icon from '../lib/Icon.svelte'
  import Segmented from '../lib/Segmented.svelte'
  import ThemeToggle from '../lib/ThemeToggle.svelte'
  import Typeset from '../lib/Typeset.svelte'
  import { back, download as downloadIcon, warning } from '../lib/icons'
  import { remember, remembered } from '../lib/storage'
  import { timeTex, timeText } from '../lib/timing'
  import { connect, type Sheet } from './engine'
  import Outline from './Outline.svelte'
  import Pages from './Pages.svelte'
  import { countWords } from './scan'
  import Source from './Source.svelte'
  import { starter } from './starter'

  const storageKey = 'quicktex-document'
  const saveDelay = 400
  const pdfDelay = 150
  const splitStorageKey = 'quicktex-split'

  const opened = remembered(storageKey) ?? starter

  let source = $state(opened)
  let files = $state.raw<ProjectFiles>({})
  let fileInput: HTMLInputElement
  let sentFiles: ProjectFiles | null = null
  let pages = $state(0)
  let version = $state(0)
  let sheets = $state.raw<string[]>([])
  let problem = $state('')
  let problemAt = $state<number | null>(null)
  let micros = $state(0)
  let pdfMicros = $state<number | null>(null)
  let warningAt = $state<number | null>(null)
  let warningCount = $state(0)
  let split = $state(Number(remembered(splitStorageKey)) || 50)
  let showing = $state<'source' | 'pages'>('source')
  let words = $state(countWords(opened))
  let cursorAt = 0
  let workspace: HTMLElement
  let editor: Source

  const summary = $derived(`${pages} ${pages === 1 ? 'page' : 'pages'}, ${words} ${words === 1 ? 'word' : 'words'}, `)
  const statusText = $derived(`${summary}${timeText(micros)}${pdfMicros === null ? '' : `, PDF ${timeText(pdfMicros)}`}`)
  const statusTex = $derived(
    `\\text{${summary}}${timeTex(micros)}${pdfMicros === null ? '' : `\\text{, PDF }${timeTex(pdfMicros)}`}`,
  )
  const warningToken = $derived(warningAt === null ? '' : (/^\\[a-zA-Z@]+|^\S{1,16}/.exec(source.slice(warningAt))?.[0] ?? ''))
  const warningLine = $derived(warningAt === null ? 0 : source.slice(0, warningAt).split('\n').length)
  const warningText = $derived(`Ignored ${warningToken} on line ${warningLine}${warningCount > 1 ? `, ${warningCount - 1} more` : ''}`)
  const warningTex = $derived(`\\text{Ignored }\\texttt{${warningToken.replace(/[^a-zA-Z0-9]/g, (c) => (c === '\\' ? '\\textbackslash ' : ''))}}\\text{ on line ${warningLine}${warningCount > 1 ? `, ${warningCount - 1} more` : ''}}`)

  function show(fresh: Sheet[]) {
    if (fresh.length === 0) return
    const next = sheets.slice()
    for (const [page, svg] of fresh) next[page] = svg
    sheets = next
  }

  const engine = connect(new Worker(new URL('./engine.worker.ts', import.meta.url), { type: 'module' }), {
    compiled(answer) {
      pages = answer.pages
      micros = answer.micros
      version++
      problem = ''
      problemAt = null
      warningCount = answer.warningCount
      warningAt = answer.warningAt
      sheets = sheets.slice(0, answer.pages)
      show(answer.sheets)
    },
    failed(answer) {
      problemAt = answer.at
      problem = `${answer.message[0].toUpperCase()}${answer.message.slice(1)} on line ${answer.line}`
    },
    timed(answer) {
      micros = answer.micros
      pdfMicros = answer.pdfMicros
    },
    sheets: show,
    broken() {
      problem = 'The renderer stopped. Reload the page to try again.'
    },
  })

  $effect(() => {
    const tex = source
    const project = files
    engine.compile(tex, project === sentFiles ? undefined : project)
    sentFiles = project
    const save = setTimeout(() => {
      remember(storageKey, tex)
      words = countWords(tex)
    }, saveDelay)
    return () => clearTimeout(save)
  })

  $effect(() => {
    if (version === 0 || problem) return
    const timer = setTimeout(() => engine.time(untrack(() => micros)), pdfDelay)
    return () => clearTimeout(timer)
  })

  async function importFiles(event: Event) {
    const selected = Array.from((event.target as HTMLInputElement).files ?? [])
    const next = { ...files }
    for (const file of selected) {
      const name = file.webkitRelativePath || file.name
      next[name] = /\.tex$/i.test(name) ? await file.text() : new Uint8Array(await file.arrayBuffer())
      if (/^(main|document)\.tex$/i.test(name)) source = next[name] as string
    }
    files = next
    fileInput.value = ''
  }

  async function download() {
    const url = URL.createObjectURL(new Blob([(await engine.pdf()) as BlobPart], { type: 'application/pdf' }))
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
    <input class="hidden" type="file" multiple accept=".tex,.png,.jpg,.jpeg,.ttf,.otf" bind:this={fileInput} onchange={importFiles} />
    <Button label="Add files" onclick={() => fileInput.click()} />
    {#if Object.keys(files).length > 0}
      <span class="text-sm text-quiet" title={Object.keys(files).join('\n')}>{Object.keys(files).length} project files</span>
      <Button label="Clear files" onclick={() => (files = {})} />
    {/if}
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
      <div class="min-h-0">
        <Source
          bind:this={editor}
          bind:value={source}
          errorAt={problemAt ?? warningAt}
          onsave={() => remember(storageKey, source)}
          oncursor={(at) => (cursorAt = at)}
        />
      </div>
      <div class="flex min-w-0 items-center border-t border-rule-strong bg-paper text-sm text-quiet">
        <div class="flex h-full min-w-0 flex-1 items-center" role="status">
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
            <p class="min-w-0 truncate px-4"><Typeset text={statusText} tex={statusTex} /></p>
            {#if warningAt !== null}
              <button
                class="flex min-w-0 shrink items-center gap-2 pr-4 text-left text-ink hover:text-green"
                type="button"
                onclick={() => warningAt !== null && editor.reveal(warningAt)}
              >
                <Icon icon={warning} />
                <span class="min-w-0 truncate"><Typeset text={warningText} tex={warningTex} /></span>
              </button>
            {/if}
          {/if}
        </div>
        <Outline {source} cursor={() => cursorAt} onjump={(at) => editor.jump(at)} />
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
      <Pages count={pages} {sheets} onwatch={(list) => engine.watch(list)} />
    </section>
  </main>
</div>
