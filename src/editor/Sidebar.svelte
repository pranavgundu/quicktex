<script lang="ts">
  import type { ProjectFiles } from '../../wasm-core/js/quicktex.js'
  import Button from '../lib/Button.svelte'
  import Icon from '../lib/Icon.svelte'
  import Menu from '../lib/Menu.svelte'
  import Typeset from '../lib/Typeset.svelte'
  import { actions, document as textIcon, download, image, plus, upload } from '../lib/icons'
  import { outline, type Heading } from './scan'

  let { files, active, source, onopen, onadd, oncreate, ondownload, onrename, ondelete, onjump }: {
    files: ProjectFiles; active: string; source: string
    onopen: (name: string) => void; onadd: () => void; oncreate: () => void; ondownload: () => void
    onrename: (name: string) => void; ondelete: (name: string) => void; onjump: (at: number) => void
  } = $props()

  const filterFrom = 7

  let headings = $state.raw<Heading[]>([])
  let filter = $state('')
  const names = $derived(['main.tex', ...Object.keys(files).filter((name) => name !== 'main.tex').sort()])
  const shown = $derived(names.filter((name) => name.toLowerCase().includes(filter.toLowerCase())))

  $effect(() => {
    const text = source
    const timer = setTimeout(() => headings = outline(text), 300)
    return () => clearTimeout(timer)
  })
</script>

<aside
  class="flex min-h-0 flex-col rounded-control border border-rule-strong bg-sheet max-md:absolute max-md:inset-y-0 max-md:left-0 max-md:z-20 max-md:w-[min(17rem,85vw)]"
  aria-label="Project navigation"
>
  <div class="flex h-12 shrink-0 items-center gap-0.5 pr-2 pl-4">
    <h2 class="flex-1 font-normal"><Typeset text="Files" tex={String.raw`\textbf{Files}`} /></h2>
    <Button tool bare icon={plus} label="New file" title="New file" onclick={oncreate} />
    <Button tool bare icon={upload} label="Add files" title="Add files from your computer" onclick={onadd} />
    <Button tool bare icon={download} label="Download current file" title="Download current file" onclick={ondownload} />
  </div>
  {#if names.length >= filterFrom || filter}
    <input
      class="mx-2 mb-2 h-8 shrink-0 rounded-control border border-rule bg-paper px-2 text-sm outline-none focus:border-green"
      aria-label="Filter files"
      placeholder="Filter files"
      bind:value={filter}
    />
  {/if}
  <div class="max-h-[45%] shrink-0 overflow-y-auto px-2 pb-2">
    {#each shown as name (name)}
      <div class={['flex items-center rounded-control', active === name ? 'bg-rule' : 'hover:bg-paper']}>
        <button
          class="flex h-8 min-w-0 flex-1 items-center gap-2 px-2 text-left"
          type="button"
          aria-current={active === name ? 'page' : undefined}
          title={name}
          onclick={() => onopen(name)}
        >
          <span class="text-quiet"><Icon icon={typeof files[name] === 'object' ? image : textIcon} /></span>
          <span class="truncate font-mono text-[13px]">{name}</span>
        </button>
        {#if name !== 'main.tex'}
          <Menu
            bare
            icon={actions}
            label="Actions for {name}"
            items={[
              { label: 'Rename', action: () => onrename(name) },
              { label: 'Delete', action: () => ondelete(name) },
            ]}
          />
        {/if}
      </div>
    {:else}
      <p class="px-2 py-1 text-sm text-quiet">No files match that filter.</p>
    {/each}
  </div>
  <h2 class="flex h-12 shrink-0 items-center border-t border-rule px-4 font-normal"><Typeset text="Sections" tex={String.raw`\textbf{Sections}`} /></h2>
  <nav class="min-h-0 flex-1 overflow-y-auto px-2 pb-2" aria-label="Document outline">
    {#each headings as heading (heading.at)}
      <button
        class="block h-8 w-full truncate rounded-control pr-2 text-left text-sm text-quiet hover:bg-paper hover:text-ink"
        style:padding-left="{8 + heading.level * 12}px"
        type="button"
        title={heading.title}
        onclick={() => onjump(heading.at)}
      >
        {heading.title || 'Untitled'}
      </button>
    {:else}
      <p class="px-2 py-1 text-sm text-quiet">Add a <code class="font-mono text-[13px]">\section</code> to see it here</p>
    {/each}
  </nav>
</aside>
