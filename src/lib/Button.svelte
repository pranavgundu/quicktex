<script lang="ts">
  import type { ClassValue } from 'svelte/elements'
  import Icon from './Icon.svelte'
  import Typeset from './Typeset.svelte'

  let {
    label,
    short,
    icon,
    href,
    onclick,
    disabled = false,
    primary = false,
    bare = false,
    class: extra,
  }: {
    label: string
    short?: string
    icon?: string
    href?: string
    onclick?: () => void
    disabled?: boolean
    primary?: boolean
    bare?: boolean
    class?: ClassValue
  } = $props()

  const look = $derived([
    'inline-flex h-10 items-center justify-center gap-2 rounded-control whitespace-nowrap',
    bare ? 'w-10' : 'px-4',
    primary
      ? [
          'bg-action pb-1 text-on-action shadow-[inset_0_-4px_0_var(--color-action-edge)]',
          'hover:bg-action-hover active:pt-1 active:pb-0 active:shadow-none',
          'disabled:bg-rule disabled:text-quiet disabled:shadow-none',
        ]
      : [
          'border border-rule-strong bg-sheet text-ink',
          'hover:border-ink hover:bg-paper active:bg-rule',
          'disabled:border-rule disabled:bg-transparent disabled:text-stale',
        ],
    'disabled:cursor-not-allowed',
    extra,
  ])
</script>

{#snippet face()}
  {#if icon}<Icon {icon} />{/if}
  {#if bare}
    <span class="sr-only">{label}</span>
  {:else if short}
    <span class="sm:hidden"><Typeset text={short} /></span>
    <span class="max-sm:hidden"><Typeset text={label} /></span>
  {:else}
    <Typeset text={label} />
  {/if}
{/snippet}

{#if href}
  <a class={look} {href} aria-label={short ? label : undefined}>{@render face()}</a>
{:else}
  <button class={look} type="button" aria-label={short ? label : undefined} {disabled} {onclick}>{@render face()}</button>
{/if}
