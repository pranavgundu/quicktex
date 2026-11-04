<script lang="ts">
  import Button from './Button.svelte'
  import { moon, sun } from './icons'
  import { theme } from './theme.svelte'
</script>

<Button
  bare
  icon={theme.dark ? sun : moon}
  label={theme.dark ? 'Switch to light mode' : 'Switch to dark mode'}
  onclick={() => theme.toggle()}
/>
