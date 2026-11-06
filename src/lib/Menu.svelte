<script lang="ts">
  import type { Snippet } from 'svelte'
  import type { ClassValue } from 'svelte/elements'
  import Button from './Button.svelte'
  import Typeset from './Typeset.svelte'

  let {
    label,
    icon,
    bare = false,
    title,
    items,
    children,
    class: extra,
  }: {
    label: string
    icon?: string
    bare?: boolean
    title?: string
    items?: { label: string; action: () => void }[]
    children?: Snippet
    class?: ClassValue
  } = $props()

  const id = $props.id()
  const gap = 8

  let anchor: HTMLElement
  let panel: HTMLElement

  function place(event: ToggleEvent) {
    if (event.newState !== 'open') return
    const box = anchor.getBoundingClientRect()
    const below = box.top < innerHeight / 2
    const rightward = box.left < innerWidth / 2
    panel.style.top = below ? `${box.bottom + gap}px` : 'auto'
    panel.style.bottom = below ? 'auto' : `${innerHeight - box.top + gap}px`
    panel.style.left = rightward ? `${box.left}px` : 'auto'
    panel.style.right = rightward ? 'auto' : `${innerWidth - box.right}px`
  }

  function pick(action: () => void) {
    panel.hidePopover()
    action()
  }
</script>

<span class="inline-flex shrink-0" bind:this={anchor}>
  <Button tool {bare} {icon} {label} {title} class={extra} aria-label={label} popovertarget={id} />
</span>
<div
  class="fixed m-0 max-h-[60vh] max-w-[calc(100vw-2rem)] overflow-y-auto rounded-control border border-rule-strong bg-sheet p-1 text-ink"
  {id}
  popover="auto"
  bind:this={panel}
  onbeforetoggle={place}
>
  {#each items ?? [] as item (item.label)}
    <button
      class="flex h-8 w-full min-w-36 items-center rounded-control px-3 text-left text-sm whitespace-nowrap hover:bg-rule"
      type="button"
      onclick={() => pick(item.action)}
    >
      <Typeset text={item.label} />
    </button>
  {/each}
  {@render children?.()}
</div>
