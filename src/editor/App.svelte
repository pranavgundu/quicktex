<script lang="ts">
  import type { ProjectFiles } from '../../wasm-core/js/quicktex.js'
  import { onMount, untrack, tick } from 'svelte'
  import Button from '../lib/Button.svelte'
  import Icon from '../lib/Icon.svelte'
  import Segmented from '../lib/Segmented.svelte'
  import ThemeToggle from '../lib/ThemeToggle.svelte'
  import Typeset from '../lib/Typeset.svelte'
  import { download as downloadIcon, files as filesIcon, recompile, warning } from '../lib/icons'
  import { mod } from '../lib/keys'
  import { remember, remembered } from '../lib/storage'
  import { timeTex, timeText } from '../lib/timing'
  import { connect, type Sheet } from './engine'
  import Outline from './Outline.svelte'
  import Pages from './Pages.svelte'
  import Sidebar from './Sidebar.svelte'
  import Toolbar from './Toolbar.svelte'
  import { countWords } from './scan'
  import Source from './Source.svelte'
  import { starter } from './starter'
  import { loadProject, saveProject, validPath, textFile } from './project'

  const storageKey = 'quicktex-document'
  const opened = remembered(storageKey) ?? starter
  let source = $state(opened)
  let files = $state.raw<ProjectFiles>({})
  let active = $state('main.tex')
  const activeText = $derived(active === 'main.tex' ? source : typeof files[active] === 'string' ? files[active] as string : '')
  const tabs = $derived(['main.tex', ...Object.keys(files).filter((name) => name !== 'main.tex' && typeof files[name] === 'string').sort()])
  let ready = $state(false)
  let saved = $state(false)
  let localDirty = false
  let savedFiles: ProjectFiles | null = null
  let storageProblem = $state('')
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
  let split = $state(Math.min(80, Math.max(20, Number(remembered('quicktex-split')) || 50)))
  let showing = $state<'source' | 'pages'>('source')
  let layout = $state<'split' | 'source' | 'pages'>('split')
  let sidebar = $state(remembered('quicktex-sidebar') === null ? matchMedia('(min-width: 768px)').matches : remembered('quicktex-sidebar') !== 'false')
  let fontSize = $state(Number(remembered('quicktex-font-size')) || 14)
  let wrap = $state(remembered('quicktex-wrap') !== 'false')
  let numbers = $state(remembered('quicktex-numbers') !== 'false')
  let automatic = $state(true)
  let compiling = $state(false)
  let words = $state(countWords(opened))
  let cursorAt = 0
  let cursorLine = $state(1)
  let cursorColumn = $state(1)
  let workspace: HTMLElement
  let editor: Source
  let nameDialog: HTMLDialogElement
  let nameInput: HTMLInputElement
  let fileName = $state('')
  let renameFrom = $state('')
  let nameError = $state('')

  const summary = $derived(`${pages} ${pages === 1 ? 'page' : 'pages'}, ${words} ${words === 1 ? 'word' : 'words'}, `)
  const saveText = $derived(storageProblem ? 'Save unavailable' : saved ? 'Saved locally' : 'Saving')
  const statusText = $derived(`${summary}${timeText(micros)}${pdfMicros === null ? '' : `, PDF ${timeText(pdfMicros)}`}`)
  const statusTex = $derived(`\\text{${summary}}${timeTex(micros)}${pdfMicros === null ? '' : `\\text{, PDF }${timeTex(pdfMicros)}`}`)
  const warningToken = $derived(warningAt === null ? '' : (/^\\[a-zA-Z@]+|^\S{1,16}/.exec(source.slice(warningAt))?.[0] ?? ''))
  const warningLine = $derived(warningAt === null ? 0 : source.slice(0, warningAt).split('\n').length)
  const warningText = $derived(`Compatibility warning: ${warningToken} on line ${warningLine}${warningCount > 1 ? `, ${warningCount - 1} more` : ''}`)

  function show(fresh: Sheet[]) {
    if (problem || fresh.length === 0) return
    const next = sheets.slice()
    for (const [page, svg] of fresh) next[page] = svg
    sheets = next
  }
  const worker = new Worker(new URL('./engine.worker.ts', import.meta.url), { type: 'module' })
  const engine = connect(worker, {
    compiled(answer) {
      compiling = false
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
      compiling = false
      clearPreview()
      problemAt = answer.at
      problem = `${answer.message[0].toUpperCase()}${answer.message.slice(1)} on line ${answer.line}`
    },
    timed(answer) { micros = answer.micros; pdfMicros = answer.pdfMicros },
    sheets: show,
    broken() { compiling = false; clearPreview(); problem = 'The renderer stopped. Reload the page to try again.' },
  })
  function clearPreview() { pages = 0; sheets = []; warningAt = null; warningCount = 0; pdfMicros = null }
  function compile() {
    compiling = true
    engine.compile(source, files === sentFiles ? undefined : files)
    sentFiles = files
  }
  $effect(() => {
    const tex = source
    const project = files
    if (automatic) untrack(compile)
    const timer = setTimeout(() => words = countWords(tex), 400)
    return () => clearTimeout(timer)
  })
  $effect(() => {
    if (version === 0 || problem) return
    const timer = setTimeout(() => engine.time(untrack(() => micros)), 150)
    return () => clearTimeout(timer)
  })
  let saveSequence = 0
  async function persist(tex = source, project = files) {
    const sequence = ++saveSequence
    remember(storageKey, tex)
    if (tex === source) localDirty = false
    try {
      // Check rather than silently claiming success when localStorage is full/blocked.
      if (remembered(storageKey) !== tex) throw new Error('storage unavailable')
      if (project !== savedFiles) {
        await saveProject(project)
        savedFiles = project
      }
      if (sequence === saveSequence) { saved = true; storageProblem = '' }
    } catch {
      storageProblem = 'Local save failed. Download your source to keep a copy.'
      saved = false
    }
  }
  $effect(() => {
    const tex = source
    const project = files
    if (!ready) return
    saved = false
    const timer = setTimeout(() => persist(tex, project), 600)
    return () => clearTimeout(timer)
  })
  $effect(() => {
    remember('quicktex-sidebar', String(sidebar))
    remember('quicktex-font-size', String(fontSize))
    remember('quicktex-wrap', String(wrap))
    remember('quicktex-numbers', String(numbers))
  })
  onMount(() => {
    let mounted = true
    loadProject().then((project) => { if (mounted) { files = project; savedFiles = project } }).catch(() => {
      storageProblem = 'Project storage is unavailable. Imported files may not survive a reload.'
    }).finally(() => { if (mounted) ready = true })
    return () => { mounted = false; worker.terminate() }
  })
  function edit(text: string) {
    if (active === 'main.tex') { source = text; localDirty = true }
    else files = { ...files, [active]: text }
  }
  async function importFiles(event: Event) {
    const selected = Array.from((event.target as HTMLInputElement).files ?? [])
    const imported: ProjectFiles = {}
    for (const file of selected) {
      const name = file.webkitRelativePath || file.name
      if (!validPath(name)) continue
      if ((name in files || name === 'main.tex') && !confirm(`Replace ${name}?`)) continue
      const content = textFile(name) ? await file.text() : new Uint8Array(await file.arrayBuffer())
      if (/^(main|document)\.tex$/i.test(name) && typeof content === 'string') { source = content; localDirty = true; active = 'main.tex' }
      else imported[name] = content
    }
    // Preserve any edits made while files were being read.
    files = { ...files, ...imported }
    fileInput.value = ''
  }
  function openFile(name: string) {
    if (name === 'main.tex' || typeof files[name] === 'string') { active = name; showing = 'source'; if (matchMedia('(max-width: 767px)').matches) sidebar = false; return }
    if (/\.(png|jpe?g)$/i.test(name)) editor.insert(`\\includegraphics[width=0.8\\linewidth]{${name}}`)
    else alert(`${name} is a binary project asset. It is available to the compiler.`)
  }
  function nameFile(from = '') {
    renameFrom = from; fileName = from; nameError = ''
    nameDialog.showModal()
    queueMicrotask(() => nameInput.focus())
  }
  function commitFile(event: SubmitEvent) {
    event.preventDefault()
    const name = fileName.trim()
    if (!validPath(name) || !textFile(name)) { nameError = 'Use a relative path ending in .tex, .bib, .sty, .cls, .txt, or .csv.'; return }
    if (name === 'main.tex' || (name in files && name !== renameFrom)) { nameError = 'A file with this name already exists.'; return }
    const next = { ...files }
    next[name] = renameFrom ? next[renameFrom] : ''
    if (renameFrom && renameFrom !== name) { editor.rename(renameFrom, name); delete next[renameFrom] }
    files = next
    active = name
    nameDialog.close()
  }
  function deleteFile(name: string) {
    if (!confirm(`Delete ${name}? This cannot be undone.`)) return
    editor.forget(name)
    const next = { ...files }; delete next[name]; files = next
    if (active === name) active = 'main.tex'
  }
  function downloadBlob(bytes: BlobPart, name: string, type: string) {
    const url = URL.createObjectURL(new Blob([bytes], { type }))
    const link = document.createElement('a'); link.href = url; link.download = name; link.click()
    setTimeout(() => URL.revokeObjectURL(url), 1000)
  }
  async function download() {
    if (pages === 0 || problem) return
    const bytes = await engine.pdf()
    if (pages && !problem) downloadBlob(bytes as BlobPart, 'document.pdf', 'application/pdf')
  }
  function jump(at: number) { if (matchMedia('(max-width: 767px)').matches) sidebar = false; showing = 'source'; layout = layout === 'pages' ? 'split' : layout; editor.jump(at) }
  async function diagnostic(at: number) {
    active = 'main.tex'
    await tick()
    jump(at)
  }
  function resize(event: PointerEvent) {
    const handle = event.currentTarget as HTMLElement
    handle.setPointerCapture(event.pointerId)
    const move = (next: PointerEvent) => {
      const box = workspace.getBoundingClientRect()
      split = Math.min(80, Math.max(20, ((next.clientX - box.left) / box.width) * 100))
    }
    const stop = () => { handle.removeEventListener('pointermove', move); remember('quicktex-split', String(split)) }
    handle.addEventListener('pointermove', move)
    handle.addEventListener('pointerup', stop, { once: true })
    handle.addEventListener('pointercancel', stop, { once: true })
  }
  function nudge(event: KeyboardEvent) {
    const change = event.key === 'ArrowLeft' ? -2 : event.key === 'ArrowRight' ? 2 : 0
    if (!change) return
    event.preventDefault(); split = Math.min(80, Math.max(20, split + change)); remember('quicktex-split', String(split))
  }
  function shortcuts(event: KeyboardEvent) {
    if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') { event.preventDefault(); compile() }
  }
</script>

<svelte:window onkeydown={shortcuts} onpagehide={() => { if (localDirty) remember(storageKey, source); if (ready && files !== savedFiles) void saveProject(files).catch(() => {}) }} />

{#snippet tools()}
  <label class="flex h-10 cursor-pointer items-center gap-2 px-2" title="Compile as you type">
    <input class="size-4 accent-green" type="checkbox" bind:checked={automatic} />
    <Typeset text="Auto" />
  </label>
  {#if !automatic}
    <Button bare class={compiling ? 'motion-safe:[&_svg]:animate-spin' : undefined} icon={recompile} label="Recompile" title="Recompile ({mod}Enter)" onclick={compile} />
  {/if}
{/snippet}

{#snippet notice()}
  {#if problem}
    <div class="flex max-w-md flex-col items-center gap-4 rounded-control border border-rule-strong bg-sheet p-6 text-center">
      <p class="text-red">{problem}</p>
      {#if problemAt !== null}
        <Button label="Show in source" onclick={() => problemAt !== null && diagnostic(problemAt)} />
      {/if}
    </div>
  {/if}
{/snippet}

<div class="flex h-full flex-col px-6 pb-6 max-sm:px-4 max-sm:pb-4">
  <header class="flex shrink-0 items-center gap-4 py-4 max-sm:gap-2">
    <h1 class="min-w-0 flex-1 text-[32px] leading-none font-normal">
      <a href="../" aria-label="Back to the formula page"><Typeset text="quicktex" tex={String.raw`\textbf{quicktex}`} /></a>
    </h1>
    <span class="text-sm text-quiet max-md:hidden" title={storageProblem || 'Files are saved in this browser, not synced to a server.'}>
      <span class="save-indicator"><Typeset text={saveText} /></span>
    </span>
    <input class="hidden" type="file" multiple bind:this={fileInput} onchange={importFiles} />
    <Button bare icon={filesIcon} label="Toggle project sidebar" title="Files and sections" aria-pressed={sidebar} onclick={() => (sidebar = !sidebar)} />
    <Segmented
      class="max-md:hidden"
      legend="Layout"
      name="layout"
      bind:value={layout}
      options={[
        { value: 'split', label: 'Split' },
        { value: 'source', label: 'Source' },
        { value: 'pages', label: 'Pages' },
      ]}
    />
    <ThemeToggle />
    <Button primary disabled={pages === 0 || !!problem} icon={downloadIcon} label="Download PDF" short="PDF" onclick={download} />
  </header>

  <div class="mb-2 md:hidden">
    <Segmented
      legend="View"
      name="view"
      bind:value={showing}
      options={[
        { value: 'source', label: 'Source' },
        { value: 'pages', label: 'Pages' },
      ]}
    />
  </div>

  <div class={['relative grid min-h-0 flex-1 grid-cols-1 grid-rows-[minmax(0,1fr)] gap-x-4', sidebar && 'md:grid-cols-[14rem_minmax(0,1fr)]']}>
    {#if sidebar}
      <Sidebar
        {files}
        {active}
        source={activeText}
        onopen={openFile}
        onadd={() => ready && fileInput.click()}
        oncreate={() => ready && nameFile()}
        ondownload={() => downloadBlob(activeText, active, 'text/plain')}
        onrename={nameFile}
        ondelete={deleteFile}
        onjump={jump}
      />
    {/if}
    <main
      class={['grid min-h-0 min-w-0 grid-cols-1 grid-rows-[minmax(0,1fr)]', layout === 'split' && 'md:grid-cols-[minmax(0,var(--split))_1rem_minmax(0,1fr)]']}
      style:--split="{split}%"
      bind:this={workspace}
    >
      <section
        class={[
          'flex min-h-0 min-w-0 flex-col rounded-control border border-rule-strong bg-sheet focus-within:border-green',
          layout === 'pages' && 'md:hidden',
          showing === 'pages' && 'max-md:hidden',
        ]}
        aria-label="LaTeX source"
      >
        {#if tabs.length > 1}
          <div class="flex shrink-0 overflow-x-auto rounded-t-control border-b border-rule bg-paper [scrollbar-width:none]" role="tablist" aria-label="Open files">
            {#each tabs as name (name)}
              <button
                class={[
                  '-mb-px h-9 shrink-0 border-r border-rule px-4 font-mono text-[13px]',
                  active === name ? 'bg-sheet text-ink' : 'border-b border-b-rule text-quiet hover:text-ink',
                ]}
                type="button"
                role="tab"
                aria-selected={active === name}
                onclick={() => openFile(name)}
              >
                {name}
              </button>
            {/each}
          </div>
        {/if}
        <Toolbar run={(command) => editor.run(command)} insert={(text) => editor.insert(text)} bind:fontSize bind:wrap bind:numbers />
        <div class="min-h-0 flex-1 overflow-hidden">
          <Source
            bind:this={editor}
            value={activeText}
            documentId={active}
            ondoc={edit}
            {fontSize}
            {wrap}
            {numbers}
            errorAt={active === 'main.tex' ? problemAt ?? warningAt : null}
            onsave={() => persist()}
            oncursor={(at, line, column) => { cursorAt = at; cursorLine = line; cursorColumn = column }}
          />
        </div>
        <div class="@container flex shrink-0 items-center gap-4 border-t border-rule p-2 pl-4 text-sm text-quiet">
          <p class="shrink-0 @max-md:hidden"><Typeset text="Ln {cursorLine}, Col {cursorColumn}" /></p>
          <div class="flex h-10 min-w-0 flex-1 items-center gap-4 overflow-hidden" role="status">
            {#if problem}
              <button class="flex min-w-0 items-center gap-2 text-left text-red" type="button" title={problem} onclick={() => problemAt !== null && diagnostic(problemAt)}>
                <Icon icon={warning} /><span class="truncate">{problem}</span>
              </button>
            {:else}
              <p class={['shrink-0', warningAt !== null && '-mr-4 [&>[aria-hidden]]:hidden']}><Typeset text={statusText} tex={statusTex} /></p>
              {#if warningAt !== null}
                <button class="flex min-w-0 items-center gap-2 text-left text-ink" type="button" title={warningText} onclick={() => warningAt !== null && diagnostic(warningAt)}>
                  <Icon icon={warning} /><span class="truncate">{warningText}</span>
                </button>
              {/if}
            {/if}
          </div>
          <Outline source={activeText} cursor={() => cursorAt} onjump={jump} />
        </div>
      </section>
      <button
        class={['group hidden cursor-col-resize touch-none items-center justify-center', layout === 'split' && 'md:flex']}
        type="button"
        aria-label="Resize panes, use the left and right arrow keys"
        onpointerdown={resize}
        onkeydown={nudge}
      >
        <span class="h-10 w-1 rounded-full bg-rule-strong group-hover:bg-green group-focus-visible:bg-green"></span>
      </button>
      <section
        class={[
          'min-h-0 min-w-0 rounded-control border border-rule-strong bg-sheet',
          layout === 'source' && 'md:hidden',
          showing === 'source' && 'max-md:hidden',
        ]}
        aria-label="Pages"
      >
        <Pages count={pages} {sheets} onwatch={(list) => engine.watch(list)} {notice} {tools} />
      </section>
    </main>
  </div>
  {#if storageProblem}
    <p class="fixed right-6 bottom-6 z-30 max-w-[calc(100%-3rem)] rounded-control border border-red bg-sheet px-4 py-2 text-sm text-red">{storageProblem}</p>
  {/if}
</div>

<dialog
  class="m-auto w-[min(28rem,calc(100vw-2rem))] rounded-control border border-rule-strong bg-sheet p-6 text-ink backdrop:bg-black/40"
  bind:this={nameDialog}
  aria-label={renameFrom ? 'Rename file' : 'New file'}
>
  <form class="flex flex-col gap-4" onsubmit={commitFile}>
    <h2 class="text-2xl font-normal"><Typeset text={renameFrom ? 'Rename file' : 'New file'} tex={`\\textbf{${renameFrom ? 'Rename file' : 'New file'}}`} /></h2>
    <label class="flex flex-col gap-2">
      <Typeset text="File name" />
      <input
        class="h-10 w-full rounded-control border border-rule-strong bg-paper px-3 font-mono text-sm outline-none focus:border-green"
        bind:this={nameInput}
        bind:value={fileName}
        placeholder="chapters/introduction.tex"
        required
      />
    </label>
    {#if nameError}<p class="text-sm text-red">{nameError}</p>{/if}
    {#if renameFrom}<p class="text-sm text-quiet">Update any references to the old file name in your source.</p>{/if}
    <div class="flex justify-end gap-2">
      <Button label="Cancel" onclick={() => nameDialog.close()} />
      <Button primary type="submit" label={renameFrom ? 'Rename' : 'Create file'} />
    </div>
  </form>
</dialog>
