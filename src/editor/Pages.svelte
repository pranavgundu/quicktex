<script lang="ts">
  import { untrack } from 'svelte'
  import { documentPage } from '../../wasm-core/js/quicktex.js'
  import Icon from '../lib/Icon.svelte'
  import Typeset from '../lib/Typeset.svelte'
  import { minus, plus } from '../lib/icons'
  import { showPage } from './patch'

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

  const percent = $derived(Math.round(steps[step] * 100))
  const zoomButton =
    'flex w-10 items-center justify-center hover:bg-rule active:bg-rule-strong disabled:cursor-not-allowed disabled:bg-transparent disabled:text-stale'

  function watch(node: HTMLElement, page: number) {
    const observer = new IntersectionObserver(
      ([entry]) => {
        const next = new Set(visible)
        if (entry.isIntersecting) next.add(page)
        else next.delete(page)
        visible = next
      },
      { root: node.closest('[data-scroller]'), rootMargin: '600px' },
    )
    observer.observe(node)
    return { destroy: () => observer.disconnect() }
  }
</script>

<div class="relative h-full min-h-0 bg-well">
  <div class="h-full overflow-auto" data-scroller bind:clientWidth={width}>
    <div class="flex w-max min-w-full flex-col items-center gap-6 p-6 leading-none" style:font-size="{size}px">
      {#each { length: count }, page (page)}
        <article
          class="flex-none bg-white text-black ring-1 ring-rule-strong [&_svg]:block"
          aria-label="Page {page + 1} of {count}"
          style:width="{pageWidth}em"
          style:height="{pageHeight}em"
          use:watch={page}
          use:showPage={rendered[page] ?? ''}
        ></article>
      {/each}
    </div>
  </div>
  <div class="absolute right-4 bottom-4 z-10 flex h-10 overflow-hidden rounded-control border border-rule-strong bg-sheet">
    <button class={zoomButton} type="button" aria-label="Zoom out" disabled={step === 0} onclick={() => step--}>
      <Icon icon={minus} />
    </button>
    <button
      class="min-w-16 border-x border-rule px-2 hover:bg-rule active:bg-rule-strong"
      type="button"
      aria-label="Reset zoom"
      onclick={() => (step = 3)}
    >
      <Typeset text="{percent}%" tex={`\\text{${percent}\\%}`} />
    </button>
    <button class={zoomButton} type="button" aria-label="Zoom in" disabled={step === steps.length - 1} onclick={() => step++}>
      <Icon icon={plus} />
    </button>
  </div>
</div>
