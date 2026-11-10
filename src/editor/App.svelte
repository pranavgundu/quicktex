<script lang="ts">
  import type { ProjectFiles } from '../../wasm-core/js/quicktex.js'
  import { zipSync, strToU8 } from 'fflate'
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
  import { SharedProject, connectSession, newSessionToken, sessionToken } from './collaboration'

  const storageKey = 'quicktex-document'
  const incomingSession = sessionToken(location.hash)
  const opened = incomingSession ? '' : remembered(storageKey) ?? starter
  let session = $state.raw<{ token: string; project: SharedProject; connection: ReturnType<typeof connectSession> } | null>(null)
  let sessionRevision = $state(0)
  let participants = $state(1)
  let sessionReady = $state(false)
  let shareDialog: HTMLDialogElement
  let shareNotice = $state('')
  let sharingError = $state('')
  let connectionInput = $state('')
  let connectionOutput = $state('')
  let connectionBusy = $state(false)
  const shareLink = $derived(session ? `${location.href.split('#')[0]}#session=${session.token}` : '')
  const sharedEditor = $derived.by(() => {
    sessionRevision
    return session?.project.editor(active, session.connection.provider.awareness)
  })
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
  let editor = $state<Source>()
  let nameDialog: HTMLDialogElement
  let nameInput: HTMLInputElement
  let fileName = $state('')
  let renameFrom = $state('')
  let nameError = $state('')

  const summary = $derived(`${pages} ${pages === 1 ? 'page' : 'pages'}, ${words} ${words === 1 ? 'word' : 'words'}, `)
  const saveText = $derived(session ? 'Live - not saved' : storageProblem ? 'Save unavailable' : saved ? 'Saved locally' : 'Saving')
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
    if (session || incomingSession) return
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
    if (!ready || session || incomingSession) return
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
    if (incomingSession) {
      beginSession(incomingSession)
      ready = true
    } else {
      loadProject().then((project) => { if (mounted) { files = project; savedFiles = project } }).catch(() => {
        storageProblem = 'Project storage is unavailable. Imported files may not survive a reload.'
      }).finally(() => { if (mounted) ready = true })
    }
    return () => { mounted = false; session?.connection.destroy(); worker.terminate() }
  })
  function beginSession(token = newSessionToken(), seed = false) {
    const project = new SharedProject()
    try {
      if (seed) project.seed(source, files)
      let previousPaths = project.paths()
      const refresh = () => {
        if (!session) return
        const beforeId = previousPaths.get(active)
        previousPaths = project.paths()
        const snapshot = project.snapshot()
        source = snapshot.source
        const names = Object.keys(snapshot.files)
        if (names.length !== Object.keys(files).length || names.some(name => snapshot.files[name] !== files[name])) files = snapshot.files
        if (active !== 'main.tex' && !(active in snapshot.files)) {
          active = [...project.paths()].find(([, id]) => id === beforeId)?.[0] ?? 'main.tex'
        }
        sessionReady = project.initialized
        participants = session.connection.provider.awareness.getStates().size
        sessionRevision++
      }
      const connection = connectSession(token, project, refresh)
      session = { token, project, connection }
      localDirty = false
      storageProblem = ''
      history.replaceState(null, '', `#session=${token}`)
      refresh()
    } catch (error) {
      project.destroy()
      sharingError = error instanceof Error ? error.message : 'Could not start collaboration.'
    }
  }
  async function copyShareLink() {
    try { await navigator.clipboard.writeText(shareLink); shareNotice = 'Link copied' }
    catch { shareNotice = 'Select and copy the link below.' }
  }
  async function manualConnection(action: 'offer' | 'answer' | 'finish') {
    if (!session) return
    connectionBusy = true
    sharingError = ''
    try {
      const manual = session.connection.manual
      if (action === 'offer') connectionOutput = await manual.offer()
      else if (action === 'answer') connectionOutput = await manual.answer(connectionInput)
      else { await manual.finish(connectionInput); connectionOutput = ''; shareNotice = 'Connection started. Wait for the connected count to change.' }
      connectionInput = ''
    } catch (error) { sharingError = error instanceof Error ? error.message : 'Connection failed.' }
    finally { connectionBusy = false }
  }
  function leaveSession() {
    if (!confirm('Leave this temporary session? Download any work you want to keep first. Your previous local project will reopen.')) return
    session?.connection.destroy()
    history.replaceState(null, '', location.pathname + location.search)
    location.reload()
  }
  function edit(text: string) {
    if (session) return // CodeMirror writes shared text directly; the observer updates the preview.
    if (active === 'main.tex') { source = text; localDirty = true }
    else files = { ...files, [active]: text }
  }
  async function importFiles(event: Event) {
    const selected = Array.from((event.target as HTMLInputElement).files ?? [])
    const importingSession = session
    const imported: ProjectFiles = {}
    for (const file of selected) {
      const name = file.webkitRelativePath || file.name
      if (!validPath(name)) continue
      if ((name in files || name === 'main.tex') && !confirm(`Replace ${name}?`)) continue
      const content = textFile(name) ? await file.text() : new Uint8Array(await file.arrayBuffer())
      if (importingSession) {
        try { importingSession.project.put(/^(main|document)\.tex$/i.test(name) ? 'main.tex' : name, content) }
        catch (error) { sharingError = error instanceof Error ? error.message : 'Import failed.'; break }
        continue
      }
      if (/^(main|document)\.tex$/i.test(name) && typeof content === 'string') { source = content; localDirty = true; active = 'main.tex' }
      else imported[name] = content
    }
    // Preserve any edits made while files were being read.
    if (!importingSession) files = { ...files, ...imported }
    fileInput.value = ''
  }
  function openFile(name: string) {
    if (name === 'main.tex' || typeof files[name] === 'string') { active = name; showing = 'source'; if (matchMedia('(max-width: 767px)').matches) sidebar = false; return }
    if (/\.(png|jpe?g)$/i.test(name)) editor?.insert(`\\includegraphics[width=0.8\\linewidth]{${name}}`)
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
    if (session) {
      try {
        if (renameFrom) session.project.rename(renameFrom, name)
        else session.project.put(name, '')
        active = name
        nameDialog.close()
      } catch (error) { nameError = error instanceof Error ? error.message : 'File operation failed.' }
      return
    }
    const next = { ...files }
    next[name] = renameFrom ? next[renameFrom] : ''
    if (renameFrom && renameFrom !== name) { editor?.rename(renameFrom, name); delete next[renameFrom] }
    files = next
    active = name
    nameDialog.close()
  }
  function deleteFile(name: string) {
    if (!confirm(`Delete ${name}? This cannot be undone.`)) return
    if (session) { session.project.remove(name); return }
    editor?.forget(name)
    const next = { ...files }; delete next[name]; files = next
    if (active === name) active = 'main.tex'
  }
  function downloadBlob(bytes: BlobPart, name: string, type: string) {
    const url = URL.createObjectURL(new Blob([bytes], { type }))
    const link = document.createElement('a'); link.href = url; link.download = name; link.click()
    setTimeout(() => URL.revokeObjectURL(url), 1000)
  }
  function downloadProject() {
    const archive: Record<string, Uint8Array> = Object.create(null)
    archive['main.tex'] = strToU8(source)
    for (const [name, content] of Object.entries(files)) archive[name] = typeof content === 'string' ? strToU8(content) : content
    downloadBlob(zipSync(archive).buffer as ArrayBuffer, 'quicktex-project.zip', 'application/zip')
  }
  async function download() {
    if (pages === 0 || problem) return
    const bytes = await engine.pdf()
    if (pages && !problem) downloadBlob(bytes as BlobPart, 'document.pdf', 'application/pdf')
  }
  function jump(at: number) { if (matchMedia('(max-width: 767px)').matches) sidebar = false; showing = 'source'; layout = layout === 'pages' ? 'split' : layout; editor?.jump(at) }
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

<svelte:window onkeydown={shortcuts} onpagehide={() => { if (!session && !incomingSession && localDirty) remember(storageKey, source); if (!session && !incomingSession && ready && files !== savedFiles) void saveProject(files).catch(() => {}) }} />

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
    <span class="text-sm text-quiet max-md:hidden" title={session ? 'Temporary peer-to-peer session. Download a copy before everyone leaves.' : storageProblem || 'Files are saved in this browser, not synced to a server.'}>
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
    <Button label={session ? 'Share (' + participants + ')' : 'Share'} disabled={!ready} onclick={() => shareDialog.showModal()} />
    <Button primary disabled={pages === 0 || !!problem} icon={downloadIcon} label="Download PDF" short="PDF" onclick={download} />
  </header>

  {#if session}
    <div class="mb-3 flex flex-wrap items-center gap-3 rounded-control border border-rule bg-sheet px-4 py-2 text-sm" role="status">
      <span class="flex-1">{!sessionReady ? 'Waiting for someone with this session open…' : participants > 1 ? `${participants} people connected · changes shared live` : 'Session open · waiting for collaborators'} · No cloud or local autosave. Download a copy to keep your work. If connection stalls, open Share for manual connection.</span>
      <Button tool label="Download project" onclick={downloadProject} disabled={!sessionReady} />
      <Button tool label="Leave" onclick={leaveSession} />
    </div>
  {/if}
  {#if sharingError}<p class="mb-3 text-sm text-red" role="alert">{sharingError}</p>{/if}
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
        <Toolbar run={(command) => editor?.run(command)} insert={(text) => editor?.insert(text)} bind:fontSize bind:wrap bind:numbers />
        <div class="min-h-0 flex-1 overflow-hidden">
          {#if !session || sharedEditor}
          {#key session ? sharedEditor?.text : 'local'}
          <Source
            shared={sharedEditor}
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
          {/key}
          {:else}
            <p class="p-6 text-quiet">Keep the session creator’s tab open while you connect. If nobody is online, this session has no saved copy.</p>
          {/if}
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

<dialog
  class="m-auto w-[min(34rem,calc(100vw-2rem))] rounded-control border border-rule-strong bg-sheet p-6 text-ink backdrop:bg-black/40"
  bind:this={shareDialog}
  aria-label="Share a temporary editing session"
>
  <div class="flex flex-col gap-4">
    <h2 class="text-2xl"><Typeset text="Edit together" /></h2>
    <p>Everyone with the link can edit all project files. Changes travel directly between connected browsers, not to cloud storage.</p>
    <p class="text-sm text-quiet">Keep at least one participant’s tab open. When everyone leaves, the session is gone. Reloading can also lose unsent edits. Download important work before leaving.</p>
    <p class="text-sm text-quiet">A free public signaling service helps browsers connect. It sees connection metadata, not document content. Some school, work, or mobile networks may block peer-to-peer connections. There is no paid relay or guaranteed availability. Best for small trusted groups.</p>
    {#if session}
      <label class="flex flex-col gap-2">
        <span>Private editing link — share only with people you trust</span>
        <input class="w-full rounded-control border border-rule bg-paper p-3 font-mono text-sm" aria-label="Session link" readonly value={shareLink} onclick={(event) => event.currentTarget.select()} />
      </label>
      <p class="text-sm text-quiet">{participants} {participants === 1 ? 'person' : 'people'} connected. No cloud or local autosave during this session.</p>
      {#if shareNotice}<p role="status">{shareNotice}</p>{/if}
    {/if}
    {#if session}
      <details class="rounded-control border border-rule p-3 text-sm">
        <summary class="cursor-pointer">Not connecting? Connect manually without a signaling server</summary>
        <div class="mt-3 flex flex-col gap-3">
          <p>1. Both people open the same editing link. One person creates an invitation code and sends it privately to the other person.</p>
          <Button label="Create invitation code" disabled={connectionBusy} onclick={() => manualConnection('offer')} />
          <p>2. The other person pastes the invitation below and clicks “Create reply code”, then sends the reply back.</p>
          <label class="flex flex-col gap-1">Code received from your collaborator
            <textarea class="w-full rounded-control border border-rule bg-paper p-2 font-mono" rows="3" bind:value={connectionInput} spellcheck="false" aria-label="Received connection code"></textarea>
          </label>
          <div class="flex flex-wrap gap-2">
            <Button label="Create reply code" disabled={connectionBusy || !connectionInput.trim()} onclick={() => manualConnection('answer')} />
            <Button label="Connect using reply" disabled={connectionBusy || !connectionInput.trim()} onclick={() => manualConnection('finish')} />
          </div>
          <p>3. The person who created the invitation pastes the reply and clicks “Connect using reply”. Repeat for each additional collaborator. Keep this tab open while exchanging codes.</p>
          {#if connectionBusy}<p role="status">Preparing connection…</p>{/if}
          {#if connectionOutput}
            <label class="flex flex-col gap-1">Send this entire code privately to your collaborator
              <textarea class="w-full rounded-control border border-rule bg-paper p-2 font-mono" rows="3" readonly value={connectionOutput} aria-label="Generated connection code" onclick={(event) => event.currentTarget.select()}></textarea>
            </label>
          {/if}
          <p class="text-quiet">Codes contain network connection details. Only exchange them with trusted people. Manual connections still cannot bypass networks that require a TURN relay.</p>
        </div>
      </details>
    {/if}
    {#if sharingError}<p class="text-red" role="alert">{sharingError}</p>{/if}
    <div class="flex justify-end gap-2">
      <Button label="Close" onclick={() => shareDialog.close()} />
      {#if session}
        <Button primary label="Copy editing link" onclick={copyShareLink} />
      {:else}
        <Button primary label="Start temporary session" onclick={async () => { await persist(); beginSession(undefined, true) }} />
      {/if}
    </div>
  </div>
</dialog>
