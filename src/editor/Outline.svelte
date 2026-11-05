<script lang="ts">
  import Icon from '../lib/Icon.svelte'
  import Typeset from '../lib/Typeset.svelte'
  import { list } from '../lib/icons'
  import { currentHeading, outline, type Heading } from './scan'

  let {
    source,
    cursor,
    onjump,
  }: { source: string; cursor: () => number; onjump: (at: number) => void } = $props()

  let open = $state(false)
  let headings = $state<Heading[]>([])
  let current = $state(-1)
  let root: HTMLElement
  let toggle: HTMLButtonElement

  const entries = () => [...root.querySelectorAll<HTMLButtonElement>('li button')]

  function show() {
    headings = outline(source)
    current = currentHeading(headings, cursor())
    open = true
    queueMicrotask(() => entries()[Math.max(current, 0)]?.focus())
  }

  function hide(refocus: boolean) {
    open = false
    if (refocus) toggle.focus()
  }

  function pick(at: number) {
    hide(false)
    onjump(at)
  }

  function keys(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault()
      hide(true)
      return
    }
    const step = event.key === 'ArrowDown' ? 1 : event.key === 'ArrowUp' ? -1 : 0
    if (step === 0) return
    event.preventDefault()
    const all = entries()
    if (all.length === 0) return
    const at = all.indexOf(document.activeElement as HTMLButtonElement)
    all[(at + step + all.length) % all.length].focus()
  }

  function outside(event: PointerEvent) {
    if (open && !root.contains(event.target as Node)) hide(false)
  }
</script>

<svelte:window onpointerdown={outside} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="relative h-full shrink-0" bind:this={root} onkeydown={keys}>
  <button
    class="flex h-full items-center gap-2 border-l border-rule-strong px-4 text-ink hover:bg-sheet"
    type="button"
    aria-expanded={open}
    aria-haspopup="true"
    bind:this={toggle}
    onclick={() => (open ? hide(true) : show())}
  >
    <Icon icon={list} />
    <Typeset text="Outline" />
  </button>
  {#if open}
    <div class="absolute right-0 bottom-full z-10 mb-2 max-h-96 w-80 max-w-[calc(100vw-32px)] overflow-y-auto rounded-control border border-rule-strong bg-sheet py-2">
      {#if headings.length === 0}
        <p class="px-4 py-2 text-quiet"><Typeset text="Add a \section to see it here" tex={String.raw`\text{Add a }\texttt{\textbackslash section}\text{ to see it here}`} /></p>
      {:else}
        <ul>
          {#each headings as heading, index (heading.at)}
            <li>
              <button
                class={['block w-full truncate py-1 pr-4 text-left hover:bg-paper focus-visible:bg-paper', index === current ? 'text-green' : 'text-ink']}
                style:padding-left="{16 + heading.level * 16}px"
                type="button"
                aria-current={index === current ? 'location' : undefined}
                onclick={() => pick(heading.at)}
              >
                {heading.title || 'Untitled'}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}
</div>
