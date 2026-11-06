<script lang="ts">
  import { tick } from 'svelte'
  import Button from '../lib/Button.svelte'
  import Menu from '../lib/Menu.svelte'
  import Typeset from '../lib/Typeset.svelte'
  import { command, find, plus, redo, settings, undo } from '../lib/icons'
  import { mod } from '../lib/keys'

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
    { label: 'Undo', id: 'undo', key: `${mod}Z` },
    { label: 'Redo', id: 'redo', key: `${mod}⇧Z` },
    { label: 'Bold', id: 'bold', key: `${mod}B` },
    { label: 'Italic', id: 'italic', key: `${mod}I` },
    { label: 'Find and replace', id: 'search', key: `${mod}F` },
    { label: 'Go to line', id: 'goto', key: `${mod}G` },
    { label: 'Toggle comment', id: 'comment', key: `${mod}/` },
    { label: 'Indent selection', id: 'indent', key: '' },
  ]
  const hints = [
    ['Multiple cursors', 'Alt+click'],
    ['Save locally', `${mod}S`],
    ['Compile', `${mod}Enter`],
    ['Go to line', `${mod}G`],
    ['Commands', `${mod}⇧P`],
  ]
  const field = 'rounded-control border border-rule-strong bg-paper text-ink outline-none focus:border-green'

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
  function browse(event: KeyboardEvent) {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault()
      selected = (selected + (event.key === 'ArrowDown' ? 1 : -1) + choices.length) % Math.max(choices.length, 1)
      dialog.querySelectorAll('[data-choice]')[selected]?.scrollIntoView({ block: 'nearest' })
    } else if (event.key === 'Enter') {
      event.preventDefault()
      pick(selected)
    }
  }
</script>

<svelte:window onkeydown={keys} />

{#snippet divider()}
  <span class="mx-1 h-5 w-px shrink-0 bg-rule @max-md:hidden" aria-hidden="true"></span>
{/snippet}

<div class="@container flex shrink-0 items-center gap-0.5 overflow-x-auto border-b border-rule p-1 [scrollbar-width:none]" role="toolbar" aria-label="Editing tools">
  <Button tool bare icon={undo} label="Undo" title="Undo ({mod}Z)" onclick={() => run('undo')} />
  <Button tool bare icon={redo} label="Redo" title="Redo ({mod}⇧Z)" onclick={() => run('redo')} />
  {@render divider()}
  <Button tool bare tex={String.raw`\textbf{B}`} label="Bold" title="Bold ({mod}B)" onclick={() => run('bold')} />
  <Button tool bare tex={String.raw`\textit{I}`} label="Italic" title="Italic ({mod}I)" onclick={() => run('italic')} />
  <Button tool bare tex={String.raw`\%`} label="Toggle comment" title="Toggle comment ({mod}/)" onclick={() => run('comment')} />
  {@render divider()}
  <Menu class="@max-md:w-8 @max-md:px-0 @max-md:[&>:not(:first-child)]:hidden" label="Insert" icon={plus} items={snippets.map((item) => ({ label: item.label, action: () => insert(item.text) }))} />
  <div class="flex-1"></div>
  <Button tool bare icon={find} label="Find and replace" title="Find and replace ({mod}F)" onclick={() => run('search')} />
  <Button tool bare icon={command} label="Command palette" title="Command palette ({mod}⇧P)" onclick={open} />
  <Menu bare label="Editor settings" title="Editor settings" icon={settings}>
    <div class="w-60 p-3 text-sm">
      <label class="flex h-8 items-center justify-between gap-2">
        <Typeset text="Font size" />
        <select class={[field, 'h-8 px-1']} aria-label="Font size" bind:value={fontSize}>
          {#each [12, 13, 14, 15, 16, 18, 20] as size (size)}<option value={size}>{size}px</option>{/each}
        </select>
      </label>
      <label class="flex h-8 cursor-pointer items-center gap-2">
        <input class="size-4 accent-green" type="checkbox" bind:checked={wrap} />
        <Typeset text="Wrap long lines" />
      </label>
      <label class="flex h-8 cursor-pointer items-center gap-2">
        <input class="size-4 accent-green" type="checkbox" bind:checked={numbers} />
        <Typeset text="Line numbers" />
      </label>
      <dl class="mt-2 grid grid-cols-[1fr_auto] gap-x-4 gap-y-1 border-t border-rule pt-3 text-xs text-quiet">
        {#each hints as [action, shortcut] (action)}
          <dt>{action}</dt>
          <dd class="text-right font-mono">{shortcut}</dd>
        {/each}
      </dl>
    </div>
  </Menu>
</div>

<dialog
  class="m-auto w-[min(35rem,calc(100vw-2rem))] rounded-control border border-rule-strong bg-sheet p-2 text-ink backdrop:bg-black/40"
  bind:this={dialog}
  aria-label="Command palette"
>
  <input
    class={[field, 'h-10 w-full px-3']}
    bind:this={search}
    bind:value={query}
    oninput={() => (selected = 0)}
    aria-label="Search commands"
    placeholder="Search commands or insert LaTeX"
    onkeydown={browse}
  />
  <div class="mt-2 max-h-80 overflow-y-auto">
    {#each choices as item, index (item.label)}
      <button
        class={['flex h-9 w-full items-center justify-between gap-4 rounded-control px-3 text-left text-sm', selected === index ? 'bg-ink text-sheet' : 'hover:bg-rule']}
        type="button"
        data-choice
        onclick={() => pick(index)}
      >
        <Typeset text={item.label} />
        <kbd class="font-mono text-xs opacity-70">{item.key}</kbd>
      </button>
    {:else}
      <p class="p-3 text-sm text-quiet">No commands match that search.</p>
    {/each}
  </div>
</dialog>
