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

<fieldset class={['flex h-10 gap-1 rounded-control bg-well p-1 text-sm text-quiet', extra]}>
  <legend class="sr-only">{legend}</legend>
  {#each options as option (option.value)}
    <label
      class="relative flex flex-1 cursor-pointer items-center justify-center rounded-control px-4 transition-colors not-has-checked:hover:text-ink has-checked:bg-sheet has-checked:text-ink has-checked:shadow-sm has-focus-visible:outline-2 has-focus-visible:outline-offset-2 has-focus-visible:outline-green"
    >
      <input class="absolute inset-0 cursor-pointer opacity-0" type="radio" {name} value={option.value} bind:group={value} />
      <Typeset text={option.label} />
    </label>
  {/each}
</fieldset>
