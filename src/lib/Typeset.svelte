<script lang="ts">
  import { render } from '../../wasm-core/js/quicktex.js'

  let { text, tex }: { text: string; tex?: string } = $props()

  const svg = $derived(render(tex ?? `\\text{${text}}`))
</script>

<span class="hidden">{text}</span>
<span aria-hidden="true">{@html svg}</span>
