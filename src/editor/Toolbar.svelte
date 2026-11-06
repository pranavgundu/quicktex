<script lang="ts">
  import { tick } from 'svelte'
  let { run, insert, fontSize = $bindable(14), wrap = $bindable(true), numbers = $bindable(true) }: {
    run: (command: string) => void
    insert: (template: string) => void
    fontSize?: number; wrap?: boolean; numbers?: boolean
  } = $props()

  const snippets = [
    { label: 'Section', text: '\\section{${Title}}\n' },
    { label: 'Subsection', text: '\\subsection{${Title}}\n' },
    { label: 'Equation', text: '\n\\begin{equation}\n  ${x = y}\n\\end{equation}\n' },
    { label: 'Aligned equations', text: '\n\\begin{align}\n  ${x} &= ${y} \\\\\n  ${a} &= ${b}\n\\end{align}\n' },
    { label: 'Bullet list', text: '\n\\begin{itemize}\n  \\item ${First item}\n  \\item ${Second item}\n\\end{itemize}\n' },
    { label: 'Numbered list', text: '\n\\begin{enumerate}\n  \\item ${First item}\n  \\item ${Second item}\n\\end{enumerate}\n' },
    { label: 'Table', text: '\n\\begin{table}[ht]\n\\centering\n\\begin{tabular}{ll}\n\\hline\n${Column 1} & ${Column 2} \\\\\n\\hline\n${Value 1} & ${Value 2} \\\\\n\\hline\n\\end{tabular}\n\\caption{${Caption}}\n\\label{tab:${name}}\n\\end{table}\n' },
    { label: 'Figure', text: '\n\\begin{figure}[ht]\n\\centering\n\\includegraphics[width=0.8\\linewidth]{${image.png}}\n\\caption{${Caption}}\n\\label{fig:${name}}\n\\end{figure}\n' },
    { label: 'Reference', text: '\\ref{${label}}' },
    { label: 'Citation', text: '\\cite{${key}}' },
    { label: 'Link', text: '\\href{${https://example.com}}{${Link text}}' },
  ]
  const commands = [
    { label: 'Undo', id: 'undo', key: '⌘/Ctrl Z' },
    { label: 'Redo', id: 'redo', key: '⌘/Ctrl ⇧ Z' },
    { label: 'Bold', id: 'bold', key: '⌘/Ctrl B' },
    { label: 'Italic', id: 'italic', key: '⌘/Ctrl I' },
    { label: 'Find and replace', id: 'search', key: '⌘/Ctrl F' },
    { label: 'Go to line', id: 'goto', key: '⌘/Ctrl G' },
    { label: 'Toggle comment', id: 'comment', key: '⌘/Ctrl /' },
    { label: 'Indent selection', id: 'indent', key: '' },
  ]
  let dialog: HTMLDialogElement
  let search: HTMLInputElement
  let query = $state('')
  let selected = $state(0)
  const choices = $derived([
    ...commands.map((command) => ({ label: command.label, key: command.key, action: () => run(command.id) })),
    ...snippets.map((snippet) => ({ label: `Insert ${snippet.label.toLowerCase()}`, key: '', action: () => insert(snippet.text) })),
  ].filter((item) => item.label.toLowerCase().includes(query.toLowerCase())))

  async function open() {
    query = ''
    selected = 0
    dialog.showModal()
    await tick()
    search.focus()
  }
  function pick(index: number) {
    dialog.close()
    choices[index]?.action()
  }
  function keys(event: KeyboardEvent) {
    if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key.toLowerCase() === 'p') {
      event.preventDefault()
      open()
    }
  }
</script>

<svelte:window onkeydown={keys} />
<div class="editor-toolbar" role="toolbar" aria-label="Editing tools">
  <button type="button" title="Undo (⌘/Ctrl Z)" aria-label="Undo" onclick={() => run('undo')}>↶</button>
  <button type="button" title="Redo (⌘/Ctrl Shift Z)" aria-label="Redo" onclick={() => run('redo')}>↷</button>
  <span class="tool-divider"></span>
  <button type="button" title="Bold (⌘/Ctrl B)" aria-label="Bold" onclick={() => run('bold')}><b>B</b></button>
  <button type="button" title="Italic (⌘/Ctrl I)" aria-label="Italic" onclick={() => run('italic')}><i>I</i></button>
  <button type="button" title="Toggle comment (⌘/Ctrl /)" aria-label="Toggle comment" onclick={() => run('comment')}>%</button>
  <details class="tool-menu">
    <summary>Insert <span aria-hidden="true">⌄</span></summary>
    <div class="tool-popover">
      {#each snippets as item}
        <button type="button" onclick={(event) => { (event.currentTarget.closest('details') as HTMLDetailsElement).open = false; insert(item.text) }}>{item.label}</button>
      {/each}
    </div>
  </details>
  <span class="tool-divider"></span>
  <button type="button" aria-label="Find and replace" title="Find and replace (⌘/Ctrl F)" onclick={() => run('search')}>Find</button>
  <div class="flex-1"></div>
  <button type="button" aria-label="Command palette" title="Command palette (⌘/Ctrl Shift P)" onclick={open}>⌘ <span class="max-xl:hidden">Commands</span></button>
  <details class="tool-menu">
    <summary aria-label="Editor settings" title="Editor settings">Aa <span aria-hidden="true">⌄</span></summary>
    <div class="tool-popover settings-popover">
      <strong>Editor settings</strong>
      <label>Font size <select aria-label="Font size" bind:value={fontSize}>{#each [12, 13, 14, 15, 16, 18, 20] as size}<option value={size}>{size}px</option>{/each}</select></label>
      <label><input type="checkbox" bind:checked={wrap} /> Wrap long lines</label>
      <label><input type="checkbox" bind:checked={numbers} /> Line numbers</label>
      <p>Multiple cursors: Alt + click<br />Save locally: ⌘/Ctrl S<br />Go to line: ⌘/Ctrl G<br />Commands: ⌘/Ctrl Shift P</p>
    </div>
  </details>
</div>
<dialog class="command-dialog" bind:this={dialog} aria-label="Command palette">
  <div class="command-heading"><strong>Command palette</strong><button type="button" aria-label="Close command palette" onclick={() => dialog.close()}>Esc</button></div>
  <input bind:this={search} bind:value={query} oninput={() => selected = 0} aria-label="Search commands" placeholder="Search commands or insert LaTeX…" onkeydown={(event) => {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault()
      selected = (selected + (event.key === 'ArrowDown' ? 1 : -1) + choices.length) % Math.max(choices.length, 1)
      dialog.querySelectorAll('[data-choice]')[selected]?.scrollIntoView({ block: 'nearest' })
    } else if (event.key === 'Enter') { event.preventDefault(); pick(selected) }
  }} />
  <div class="command-results">
    {#each choices as item, index}
      <button type="button" data-choice class:chosen={selected === index} onclick={() => pick(index)}><span>{item.label}</span><kbd>{item.key}</kbd></button>
    {:else}<p>No matching commands.</p>{/each}
  </div>
</dialog>
