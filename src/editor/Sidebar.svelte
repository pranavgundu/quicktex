<script lang="ts">
  import type { ProjectFiles } from '../../wasm-core/js/quicktex.js'
  import { outline, type Heading } from './scan'
  let { files, active, source, onopen, onadd, oncreate, onrename, ondelete, onjump }: {
    files: ProjectFiles; active: string; source: string
    onopen: (name: string) => void; onadd: () => void; oncreate: () => void
    onrename: (name: string) => void; ondelete: (name: string) => void; onjump: (at: number) => void
  } = $props()
  let headings = $state.raw<Heading[]>([])
  let filter = $state('')
  const names = $derived(['main.tex', ...Object.keys(files).filter((name) => name !== 'main.tex').sort()])
  $effect(() => {
    const text = source
    const timer = setTimeout(() => headings = outline(text), 300)
    return () => clearTimeout(timer)
  })
</script>

<aside class="project-sidebar" aria-label="Project navigation">
  <div class="sidebar-heading"><strong>Files</strong><div><button type="button" aria-label="New file" title="New file" onclick={oncreate}>+</button><button type="button" aria-label="Import files" title="Import files" onclick={onadd}>↑</button></div></div>
  <input class="file-filter" aria-label="Filter files" placeholder="Filter files…" bind:value={filter} />
  <div class="file-tree">
    {#each names.filter((name) => name.toLowerCase().includes(filter.toLowerCase())) as name (name)}
      <div class="file-row" class:active={active === name}>
        <button type="button" class="file-name" aria-current={active === name ? 'page' : undefined} title={name} onclick={() => onopen(name)}><span class="file-type">{typeof files[name] === 'object' ? '▧' : '≡'}</span><span>{name}</span>{#if name === 'main.tex'}<span class="root-label">root</span>{/if}</button>
        {#if name !== 'main.tex'}
          <details class="file-actions"><summary aria-label="Actions for {name}">⋮</summary><div class="tool-popover"><button type="button" onclick={() => onrename(name)}>Rename</button><button type="button" onclick={() => ondelete(name)}>Delete</button></div></details>
        {/if}
      </div>
    {/each}
  </div>
  <div class="sidebar-heading outline-heading"><strong>Document outline</strong><span>{headings.length}</span></div>
  <nav class="outline-tree" aria-label="Document outline">
    {#each headings as heading (heading.at)}
      <button type="button" style:padding-left="{14 + heading.level * 14}px" title={heading.title} onclick={() => onjump(heading.at)}>{heading.title || 'Untitled'}</button>
    {:else}<p>Sections in the current file appear here.</p>{/each}
  </nav>
  <div class="sidebar-foot">LOCAL WORKSPACE<br /><span>No account. No server round trip.</span></div>
</aside>
