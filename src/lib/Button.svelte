<script lang="ts">
  import type { ClassValue, HTMLButtonAttributes } from 'svelte/elements'
  import Icon from './Icon.svelte'
  import Typeset from './Typeset.svelte'

  let {
    label,
    short,
    icon,
    tex,
    href,
    onclick,
    disabled = false,
    primary = false,
    bare = false,
    tool = false,
    type = 'button',
    class: extra,
    ...rest
  }: {
    label: string
    short?: string
    icon?: string
    tex?: string
    href?: string
    onclick?: () => void
    disabled?: boolean
    primary?: boolean
    bare?: boolean
    tool?: boolean
    type?: 'button' | 'submit'
    class?: ClassValue
  } & Omit<HTMLButtonAttributes, 'class' | 'disabled' | 'onclick' | 'type'> = $props()

  const named = $derived(bare || short !== undefined)

  const look = $derived([
    'inline-flex shrink-0 items-center justify-center gap-2 rounded-control whitespace-nowrap',
    tool ? 'h-8' : 'h-10',
    bare ? (tool ? 'w-8' : 'w-10') : tool ? 'px-2 text-sm' : 'px-4',
    primary
      ? [
          'bg-action pb-1 text-on-action shadow-[inset_0_-4px_0_var(--color-action-edge)]',
          'hover:bg-action-hover active:pt-1 active:pb-0 active:shadow-none',
          'disabled:bg-rule disabled:text-quiet disabled:shadow-none',
        ]
      : tool
        ? 'text-ink hover:bg-rule active:bg-rule-strong disabled:text-stale'
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
    {#if tex}<Typeset text={label} {tex} />{/if}
  {:else if short !== undefined}
    {#if short}<span class="sm:hidden"><Typeset text={short} /></span>{/if}
    <span class="max-sm:hidden"><Typeset text={label} {tex} /></span>
  {:else}
    <Typeset text={label} {tex} />
  {/if}
{/snippet}

{#if href}
  <a class={look} {href} aria-label={named ? label : undefined}>{@render face()}</a>
{:else}
  <button class={look} {type} aria-label={named ? label : undefined} {disabled} {onclick} {...rest}>{@render face()}</button>
{/if}
