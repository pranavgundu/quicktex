<script lang="ts">
  import type { ProjectFiles } from '../../wasm-core/js/quicktex.js'
  import { onMount, untrack, tick } from 'svelte'
  import './workspace.css'
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
  let layout = $state('split')
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
<div class="editor-shell">
  <header class="workspace-header">
    <a class="workspace-brand" href="../" aria-label="Back to the formula page"><Icon icon={back} /><strong>quicktex<span>.</span></strong></a>
    <span class="workspace-badge">WORKSPACE</span>
    <div class="header-spacer"></div>
    <span class="save-indicator" title={storageProblem || 'Files are saved in this browser, not synced to a server.'}>{storageProblem ? 'Save unavailable' : saved ? 'Saved locally' : 'Saving…'}</span>
    <input class="hidden" type="file" multiple bind:this={fileInput} onchange={importFiles} />
    <Button label="Add files" disabled={!ready} onclick={() => fileInput.click()} />
    <button class="header-source" type="button" title="Download current source file" onclick={() => downloadBlob(activeText, active, 'text/plain')}>Source ↓</button>
    <ThemeToggle />
    <Button primary disabled={pages === 0 || !!problem} icon={downloadIcon} label="Download PDF" short="PDF" onclick={download} />
  </header>
  <div class="workspace-body" class:sidebar-open={sidebar}>
    {#if sidebar}<Sidebar {files} {active} source={activeText} onopen={openFile} onadd={() => ready && fileInput.click()} oncreate={() => ready && nameFile()} onrename={nameFile} ondelete={deleteFile} onjump={jump} />{/if}
    <div class="work-area">
      <div class="workspace-controls">
        <button type="button" class="sidebar-toggle" aria-label="Toggle project sidebar" aria-pressed={sidebar} onclick={() => sidebar = !sidebar}>☰ <span class="max-md:hidden">Project</span></button>
        <span class="root-file max-md:hidden">main.tex <span> / LaTeX</span></span>
        <div class="header-spacer"></div>
        <label class="layout-select max-md:hidden">Layout <select aria-label="Workspace layout" bind:value={layout}><option value="split">Split view</option><option value="source">Editor only</option><option value="pages">Preview only</option></select></label>
        <div class="md:hidden"><Segmented legend="View" name="view" bind:value={showing} options={[{ value: 'source', label: 'Source' }, { value: 'pages', label: 'Pages' }]} /></div>
      </div>
      <main class="editor-panes" class:editor-only={layout === 'source'} class:preview-only={layout === 'pages'} style:--split="{split}%" bind:this={workspace}>
        <section class={['source-pane', showing === 'pages' && 'max-md:hidden']} aria-label="LaTeX source">
          <div class="file-tabs" role="tablist" aria-label="Open files">
            {#each tabs as name (name)}<button type="button" role="tab" aria-selected={active === name} class:active={active === name} onclick={() => openFile(name)}>≡ <span>{name}</span></button>{/each}
          </div>
          <Toolbar run={(command) => editor.run(command)} insert={(text) => editor.insert(text)} bind:fontSize bind:wrap bind:numbers />
          <div class="min-h-0 overflow-hidden">
            <Source bind:this={editor} value={activeText} documentId={active} ondoc={edit} {fontSize} {wrap} {numbers} errorAt={active === 'main.tex' ? problemAt ?? warningAt : null} onsave={() => persist()} oncursor={(at, line, column) => { cursorAt = at; cursorLine = line; cursorColumn = column }} />
          </div>
          <div class="source-footer"><span>Ln {cursorLine}, Col {cursorColumn}</span><span>{active.endsWith('.bib') ? 'BibTeX' : 'LaTeX'} · UTF-8</span><Outline source={activeText} cursor={() => cursorAt} onjump={jump} /></div>
        </section>
        <button class="pane-divider cursor-col-resize" type="button" aria-label="Resize panes, use the left and right arrow keys" onpointerdown={resize} onkeydown={nudge}></button>
        <section class={['preview-pane', showing === 'source' && 'max-md:hidden']} aria-label="Pages">
          <div class="preview-toolbar"><button class="compile-button" type="button" title="Compile (⌘/Ctrl Enter)" onclick={compile}>{compiling ? 'Compiling…' : 'Recompile'} <span>↻</span></button><label class="auto-compile"><input type="checkbox" bind:checked={automatic} /> Auto</label><span class="header-spacer"></span><span class="preview-time">{timeText(micros)}</span><span class="live-dot" title="Compiled locally in a Web Worker"></span></div>
          <div class="min-h-0"><Pages count={pages} {sheets} onwatch={(list) => engine.watch(list)} /></div>
        </section>
      </main>
    </div>
  </div>
  <footer class="workspace-status">
    <div class="render-status" role="status">
      {#if problem}<button type="button" class="diagnostic error" onclick={() => problemAt !== null && diagnostic(problemAt)}><Icon icon={warning} />{problem}</button>
      {:else}<p><Typeset text={statusText} tex={statusTex} /></p>{#if warningAt !== null}<button type="button" class="diagnostic" onclick={() => warningAt !== null && diagnostic(warningAt)}><Icon icon={warning} /><span>{warningText}</span></button>{/if}{/if}
    </div>
    <span class="max-xl:hidden">WASM · ON DEVICE</span>
  </footer>
  {#if storageProblem}<div class="storage-warning">{storageProblem}</div>{/if}
</div>
<dialog class="file-dialog" bind:this={nameDialog} aria-label={renameFrom ? 'Rename file' : 'New file'}>
  <form onsubmit={commitFile}>
    <h2>{renameFrom ? 'Rename file' : 'New file'}</h2><p>Use folders in the path, e.g. chapters/introduction.tex.</p>
    <label>File name <input bind:this={nameInput} bind:value={fileName} placeholder="chapter.tex" required /></label>
    {#if nameError}<p class="text-red">{nameError}</p>{/if}
    <div class="dialog-actions"><button type="button" onclick={() => nameDialog.close()}>Cancel</button><button class="compile-button" type="submit">{renameFrom ? 'Rename' : 'Create file'}</button></div>
    {#if renameFrom}<p>Update any references to the old file name in your source.</p>{/if}
  </form>
</dialog>
