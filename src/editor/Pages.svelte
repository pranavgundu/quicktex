<script lang="ts">
  import { flushSync, untrack } from 'svelte'
  import { documentPage } from '../../wasm-core/js/quicktex.js'
  import Icon from '../lib/Icon.svelte'
  import Typeset from '../lib/Typeset.svelte'
  import { fit, minus, plus } from '../lib/icons'
  import { remember, remembered } from '../lib/storage'
  import { showPage } from './patch'
  import { clampZoom, doubleClickZoom, maxZoom, minZoom, nextPreset, savedZoom, wheelFactor } from './zoom'

  let { count, version }: { count: number; version: number } = $props()

  const margin = 48
  const zoomKey = 'quicktex-zoom'
  const saveDelay = 300
  const dragStart = 3

  let width = $state(0)
  let height = $state(0)
  let zoom = $state(savedZoom(remembered(zoomKey)))
  let dragging = $state(false)
  let scroller: HTMLElement
  let pageWidth = $state(55)
  let pageHeight = $state(71)
  let visible = $state.raw(new Set<number>())
  let rendered = $state.raw<string[]>([])
  const stamps: number[] = []

  const fitted = $derived(Math.max(width - margin, 120) / pageWidth)
  const size = $derived(fitted * zoom)

  $effect(() => {
    const value = String(zoom)
    const save = setTimeout(() => remember(zoomKey, value), saveDelay)
    return () => clearTimeout(save)
  })

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

  const percent = $derived(Math.round(zoom * 100))
  const zoomButton =
    'flex w-10 items-center justify-center hover:bg-rule active:bg-rule-strong disabled:cursor-not-allowed disabled:bg-transparent disabled:text-stale'

  function sheetAt(x: number, y: number) {
    const hit = document.elementFromPoint(x, y)?.closest('article')
    if (hit) return hit
    let nearest: Element | null = null
    let gap = Infinity
    for (const sheet of scroller.querySelectorAll('article')) {
      const box = sheet.getBoundingClientRect()
      const distance = Math.max(box.top - y, y - box.bottom, 0)
      if (distance < gap) {
        gap = distance
        nearest = sheet
      }
    }
    return nearest
  }

  function zoomAt(next: number, x: number, y: number) {
    const target = clampZoom(next)
    if (target === zoom) return
    const sheet = sheetAt(x, y)
    const before = sheet?.getBoundingClientRect()
    zoom = target
    flushSync()
    if (!sheet || !before) return
    const after = sheet.getBoundingClientRect()
    scroller.scrollLeft += after.left + ((x - before.left) / before.width) * after.width - x
    scroller.scrollTop += after.top + ((y - before.top) / before.height) * after.height - y
  }

  function middle() {
    const box = scroller.getBoundingClientRect()
    return [box.left + box.width / 2, box.top + box.height / 2] as const
  }

  function zoomMiddle(next: number) {
    zoomAt(next, ...middle())
  }

  function fitPage() {
    const sheet = sheetAt(...middle())
    zoom = clampZoom((height - margin) / (pageHeight * fitted))
    flushSync()
    sheet?.scrollIntoView({ block: 'center', inline: 'center' })
  }

  function keys(event: KeyboardEvent) {
    if (event.altKey) return
    if (event.key === '+' || event.key === '=') zoomMiddle(nextPreset(zoom, 1))
    else if (event.key === '-' || event.key === '_') zoomMiddle(nextPreset(zoom, -1))
    else if (event.key === '0') zoomMiddle(1)
    else return
    event.preventDefault()
  }

  function shortcuts(node: HTMLElement) {
    node.addEventListener('keydown', keys)
    return { destroy: () => node.removeEventListener('keydown', keys) }
  }

  function grab(event: PointerEvent) {
    if (event.pointerType !== 'mouse' || event.button !== 0) return
    if (!(event.target as Element).closest('article')) return
    const { clientX: startX, clientY: startY } = event
    const { scrollLeft: left, scrollTop: top } = scroller
    const move = (next: PointerEvent) => {
      const dx = next.clientX - startX
      const dy = next.clientY - startY
      if (!dragging && Math.hypot(dx, dy) < dragStart) return
      if (!dragging) scroller.setPointerCapture(event.pointerId)
      dragging = true
      scroller.scrollLeft = left - dx
      scroller.scrollTop = top - dy
    }
    const stop = () => {
      dragging = false
      scroller.removeEventListener('pointermove', move)
      scroller.removeEventListener('pointerup', stop)
      scroller.removeEventListener('pointercancel', stop)
    }
    scroller.addEventListener('pointermove', move)
    scroller.addEventListener('pointerup', stop)
    scroller.addEventListener('pointercancel', stop)
  }

  function gestures(node: HTMLElement) {
    node.tabIndex = 0
    let pending = 1
    let frame = 0
    let anchor = [0, 0]
    let pinch: { distance: number; zoom: number } | null = null
    let gestureZoom = 1

    const span = (touches: TouchList) => Math.hypot(touches[0].clientX - touches[1].clientX, touches[0].clientY - touches[1].clientY)

    const wheel = (event: WheelEvent) => {
      if (!event.ctrlKey && !event.metaKey) return
      event.preventDefault()
      pending *= wheelFactor(event.deltaY, event.deltaMode)
      anchor = [event.clientX, event.clientY]
      frame ||= requestAnimationFrame(() => {
        frame = 0
        const factor = pending
        pending = 1
        zoomAt(zoom * factor, anchor[0], anchor[1])
      })
    }

    const touchStart = (event: TouchEvent) => {
      if (event.touches.length === 2) pinch = { distance: span(event.touches), zoom }
    }

    const touchMove = (event: TouchEvent) => {
      if (!pinch || event.touches.length !== 2) return
      event.preventDefault()
      const [a, b] = [event.touches[0], event.touches[1]]
      zoomAt((pinch.zoom * span(event.touches)) / pinch.distance, (a.clientX + b.clientX) / 2, (a.clientY + b.clientY) / 2)
    }

    const touchEnd = (event: TouchEvent) => {
      if (event.touches.length < 2) pinch = null
    }

    const gestureStart = (event: Event) => {
      event.preventDefault()
      gestureZoom = zoom
    }

    const gestureChange = (event: Event) => {
      event.preventDefault()
      const { scale, clientX, clientY } = event as Event & { scale: number; clientX: number; clientY: number }
      zoomAt(gestureZoom * scale, clientX, clientY)
    }

    const listeners: [string, EventListener][] = [
      ['wheel', wheel as EventListener],
      ['touchstart', touchStart as EventListener],
      ['touchmove', touchMove as EventListener],
      ['touchend', touchEnd as EventListener],
      ['touchcancel', touchEnd as EventListener],
      ['gesturestart', gestureStart],
      ['gesturechange', gestureChange],
    ]
    for (const [name, listener] of listeners) node.addEventListener(name, listener, { passive: false })
    return {
      destroy() {
        cancelAnimationFrame(frame)
        for (const [name, listener] of listeners) node.removeEventListener(name, listener)
      },
    }
  }

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

<div class="relative h-full min-h-0 bg-well" use:shortcuts>
  <div
    class={['h-full touch-pan-x touch-pan-y overflow-auto outline-none', dragging && 'select-none']}
    role="region"
    aria-label="Pages, press plus, minus or zero to zoom"
    data-scroller
    bind:this={scroller}
    bind:clientWidth={width}
    bind:clientHeight={height}
    use:gestures
    onpointerdown={grab}
    ondblclick={(event) => zoomAt(doubleClickZoom(zoom), event.clientX, event.clientY)}
  >
    <div class="flex w-max min-w-full flex-col items-center gap-6 p-6 leading-none" style:font-size="{size}px">
      {#each { length: count }, page (page)}
        <article
          class={['flex-none bg-white text-black ring-1 ring-rule-strong [&_svg]:block', dragging ? 'cursor-grabbing' : 'cursor-grab']}
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
    <button class={zoomButton} type="button" aria-label="Zoom out" disabled={zoom <= minZoom} onclick={() => zoomMiddle(nextPreset(zoom, -1))}>
      <Icon icon={minus} />
    </button>
    <button
      class="min-w-16 border-x border-rule px-2 hover:bg-rule active:bg-rule-strong"
      type="button"
      aria-label="Reset zoom"
      onclick={() => zoomMiddle(1)}
    >
      <Typeset text="{percent}%" tex={`\\text{${percent}\\%}`} />
    </button>
    <button class={zoomButton} type="button" aria-label="Zoom in" disabled={zoom >= maxZoom} onclick={() => zoomMiddle(nextPreset(zoom, 1))}>
      <Icon icon={plus} />
    </button>
    <button class={[zoomButton, 'border-l border-rule']} type="button" aria-label="Fit whole page" onclick={fitPage}>
      <Icon icon={fit} />
    </button>
  </div>
</div>
