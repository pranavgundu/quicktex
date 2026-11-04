<script lang="ts">
  let { icon }: { icon: string } = $props()
</script>

<span class="block size-5 shrink-0 [&_svg]:block [&_svg]:size-full" aria-hidden="true">{@html icon}</span>
