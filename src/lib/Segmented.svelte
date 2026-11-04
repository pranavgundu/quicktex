<script lang="ts" generics="T extends string">
  import type { ClassValue } from 'svelte/elements'
  import Typeset from './Typeset.svelte'

  let {
    legend,
    name,
    options,
    value = $bindable(),
    class: extra,
  }: {
    legend: string
    name: string
    options: { value: T; label: string }[]
    value: T
    class?: ClassValue
  } = $props()
</script>

<fieldset class={['flex h-10 gap-1 rounded-control border border-rule-strong bg-sheet p-1', extra]}>
  <legend class="sr-only">{legend}</legend>
  {#each options as option (option.value)}
    <label
      class="relative flex flex-1 cursor-pointer items-center justify-center rounded-control px-4 not-has-checked:hover:bg-rule has-checked:bg-ink has-checked:text-sheet has-focus-visible:outline-2 has-focus-visible:outline-offset-2 has-focus-visible:outline-green"
    >
      <input class="absolute inset-0 cursor-pointer opacity-0" type="radio" {name} value={option.value} bind:group={value} />
      <Typeset text={option.label} />
    </label>
  {/each}
</fieldset>
