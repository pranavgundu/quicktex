<script lang="ts">
  import { untrack } from 'svelte'
  import { documentPage } from '../../wasm-core/js/quicktex.js'

  let { count, version }: { count: number; version: number } = $props()

  const margin = 48
  const steps = [0.5, 0.67, 0.8, 1, 1.25, 1.5, 2]

  let width = $state(0)
  let step = $state(3)
  let pageWidth = $state(55)
  let pageHeight = $state(71)
  let visible = $state.raw(new Set<number>())
  let rendered = $state.raw<string[]>([])
  const stamps: number[] = []

  const size = $derived((Math.max(width - margin, 120) / pageWidth) * steps[step])

  $effect(() => {
    const stamp = version
    const total = count
    const seen = visible
    untrack(() => {
      const next = rendered.slice(0, total)
      for (const page of new Set([0, ...seen])) {
        if (page >= total || stamps[page] === stamp) continue
        next[page] = documentPage(page)
        stamps[page] = stamp
      }
      const box = /width="([\d.]+)em" height="([\d.]+)em"/.exec(next[0] ?? '')
      if (box) {
        pageWidth = Number(box[1])
        pageHeight = Number(box[2])
      }
      rendered = next
    })
  })

  function watch(node: HTMLElement, page: number) {
    const observer = new IntersectionObserver(
      ([entry]) => {
        const next = new Set(visible)
        if (entry.isIntersecting) next.add(page)
        else next.delete(page)
        visible = next
      },
      { root: node.closest('.scroll'), rootMargin: '600px' },
    )
    observer.observe(node)
    return { destroy: () => observer.disconnect() }
  }
</script>

<div class="pages">
  <div class="zoom">
    <button type="button" aria-label="Zoom out" disabled={step === 0} onclick={() => step--}>−</button>
    <button type="button" class="level" aria-label="Reset zoom" onclick={() => (step = 3)}>{Math.round(steps[step] * 100)}%</button>
    <button type="button" aria-label="Zoom in" disabled={step === steps.length - 1} onclick={() => step++}>+</button>
  </div>
  <div class="scroll" bind:clientWidth={width}>
    <div class="sheets" style:font-size="{size}px">
      {#each { length: count }, page (page)}
        <article
          class="page"
          aria-label="Page {page + 1} of {count}"
          style:width="{pageWidth}em"
          style:height="{pageHeight}em"
          use:watch={page}
        >
          {@html rendered[page] ?? ''}
        </article>
      {/each}
    </div>
  </div>
</div>

<style>
  .pages {
    position: relative;
    height: 100%;
    min-height: 0;
    background: var(--rule);
  }

  .scroll {
    height: 100%;
    overflow: auto;
  }

  .sheets {
    display: flex;
    flex-direction: column;
    gap: var(--s4);
    align-items: center;
    width: max-content;
    min-width: 100%;
    padding: var(--s4);
    line-height: 0;
  }

  .page {
    flex: none;
    border: 1px solid var(--rule-strong);
    background: #fff;
    color: #000;
  }

  .page :global(svg) {
    display: block;
  }

  .zoom {
    position: absolute;
    z-index: 1;
    right: var(--s3);
    bottom: var(--s3);
    display: flex;
    border: 2px solid var(--ink);
    background: var(--sheet);
  }

  .zoom button {
    min-width: 36px;
    height: 36px;
    padding: 0 var(--s1);
    border: 0;
    background: transparent;
    font-size: 15px;
    cursor: pointer;
  }

  .zoom button:hover:enabled {
    background: var(--rule);
  }

  .zoom button:disabled {
    color: var(--stale);
    cursor: not-allowed;
  }

  .zoom .level {
    min-width: 56px;
    border-inline: 1px solid var(--rule-strong);
  }
</style>
