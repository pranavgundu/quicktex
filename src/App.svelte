<script lang="ts">
  import { QuicktexError, render } from '../wasm-core/js/quicktex.js'
  import Typeset from './lib/Typeset.svelte'
  import { charIndex } from './lib/latex'
  import { svgToPng } from './lib/png'

  const examples = [
    { name: 'Gaussian integral', tex: String.raw`\int_{-\infty}^{\infty} e^{-x^2}\,dx = \sqrt{\pi}` },
    { name: 'Quadratic formula', tex: String.raw`x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}` },
    { name: 'Basel problem', tex: String.raw`\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}` },
    { name: 'Euler identity', tex: String.raw`e^{i\pi} + 1 = 0` },
    { name: 'Rotation matrix', tex: String.raw`\begin{pmatrix} \cos\theta & -\sin\theta \\ \sin\theta & \cos\theta \end{pmatrix}` },
    { name: 'Absolute value', tex: String.raw`|x| = \begin{cases} x & x \ge 0 \\ -x & x < 0 \end{cases}` },
    { name: 'Schrödinger equation, physics package', tex: String.raw`\usepackage{physics}
i\hbar\pdv{t}\ket{\psi} = \hat{H}\ket{\psi}` },
    { name: 'Combustion of hydrogen, mhchem package', tex: String.raw`\usepackage{mhchem}
\ce{2H2 + O2 -> 2H2O}` },
  ]

  const cell = 24
  const frame = 2
  const stagePadding = 48
  const timingDelay = 150

  let source = $state(examples[0].tex)
  let display = $state(true)
  let svg = $state('')
  let problem = $state('')
  let micros = $state(0)
  let notice = $state({ target: '', text: '' })
  let pageWidth = $state(0)
  let stageHeight = $state(0)
  let copyTimer = 0
  let timed = false

  const empty = $derived(source.trim() === '')

  function wholeCells(space: number) {
    return Math.max(Math.floor((space - 2 * frame + 1) / cell), 1) * cell - 1
  }

  const gutter = $derived(pageWidth <= 640 ? 16 : 24)
  const sheetWidth = $derived(wholeCells(pageWidth - 2 * gutter))
  const sheetHeight = $derived(wholeCells(stageHeight))
  const columnWidth = $derived(sheetWidth + 2 * frame)
  const columnLeft = $derived(Math.floor((pageWidth - columnWidth) / 2))

  function time(tex: string, mode: boolean) {
    const start = performance.now()
    let runs = 0
    let elapsed = 0
    do {
      render(tex, mode)
      runs++
      elapsed = performance.now() - start
    } while (elapsed < 2)
    return (elapsed * 1000) / runs
  }

  $effect(() => {
    const tex = source
    const mode = display
    try {
      svg = render(tex, mode)
      problem = ''
    } catch (error) {
      if (!(error instanceof QuicktexError)) throw error
      const message = error.message[0].toUpperCase() + error.message.slice(1)
      problem = `${message} at character ${charIndex(tex, error.position) + 1}`
      return
    }
    if (empty) return
    if (!timed) {
      timed = true
      micros = time(tex, mode)
      return
    }
    const timer = setTimeout(() => (micros = time(tex, mode)), timingDelay)
    return () => clearTimeout(timer)
  })

  const fit = $derived.by(() => {
    const size = /width="([\d.]+)em" height="([\d.]+)em"/.exec(svg)
    if (!size || !pageWidth) return 48
    const across = (sheetWidth - stagePadding) / Math.max(Number(size[1]), 0.1)
    const down = (sheetHeight - stagePadding) / Math.max(Number(size[2]), 0.1)
    return Math.max(16, Math.min(64, across, down))
  })

  const slow = $derived(micros >= 1000)
  const amount = $derived(slow ? (micros / 1000).toFixed(1) : micros.toFixed(micros < 10 ? 1 : 0))
  const weight = $derived(`${(svg.length / 1024).toFixed(1)} KB`)

  function copyLabel(target: string, idle: string) {
    return notice.target === target ? notice.text : idle
  }

  async function copy(target: string, write: () => Promise<void>) {
    clearTimeout(copyTimer)
    notice = { target, text: 'Copying' }
    try {
      await write()
      notice = { target, text: 'Copied' }
    } catch {
      notice = { target, text: 'Copy blocked' }
    }
    copyTimer = window.setTimeout(() => (notice = { target: '', text: '' }), 1600)
  }

  const copySvg = () => copy('svg', () => navigator.clipboard.writeText(svg))
  const copyImage = () =>
    copy('image', () => navigator.clipboard.write([new ClipboardItem({ 'image/png': svgToPng(svg) })]))
</script>

<div class="page" bind:clientWidth={pageWidth}>
<main style:width="{columnWidth}px" style:margin-left="{columnLeft}px">
  <header>
    <h1><Typeset text="quicktex" tex={String.raw`\textbf{quicktex}`} /></h1>
    <p>
      <Typeset
        text={`Rendered in ${amount} ${slow ? 'ms' : 'µs'}`}
        tex={`\\text{${amount} }${slow ? '\\text{ms}' : '\\mu\\text{s}'}`}
      />
    </p>
    <p><Typeset text={`${weight} of SVG`} tex={`\\text{${weight}}`} /></p>
    <a class="full" href="./editor/"><Typeset text="Full editor" /></a>
  </header>

  <div class="stage" bind:clientHeight={stageHeight}>
    <section
      class="sheet"
      class:stale={problem !== ''}
      aria-label="Rendered formula"
      style:height="{sheetHeight + 2 * frame}px"
    >
      {#if empty}
        <div class="formula hint"><Typeset text="Type LaTeX below and it appears here" /></div>
      {:else}
        <div class="formula" style:font-size="{fit}px">{@html svg}</div>
      {/if}
    </section>
  </div>

  <section class="desk">
    <div class="source">
      <label for="source"><Typeset text="LaTeX" /></label>
      <p class="problem" role="status">
        {#if problem}<Typeset text={problem} />{/if}
      </p>
      <textarea
        id="source"
        rows="3"
        spellcheck="false"
        autocapitalize="off"
        autocomplete="off"
        bind:value={source}
      ></textarea>
    </div>

    <div class="tools">
      <div class="examples">
        {#each examples as example (example.name)}
          <button
            class="example"
            class:current={source === example.tex}
            type="button"
            aria-label={example.name}
            onclick={() => (source = example.tex)}
          >
            {@html render(example.tex)}
          </button>
        {/each}
      </div>

      <div class="actions">
        <fieldset>
          <legend class="hidden">Layout</legend>
          <label class="mode" class:on={display}>
            <input type="radio" name="mode" value={true} bind:group={display} />
            <Typeset text="Display" />
          </label>
          <label class="mode" class:on={!display}>
            <input type="radio" name="mode" value={false} bind:group={display} />
            <Typeset text="Inline" />
          </label>
        </fieldset>
        <button class="copy" type="button" disabled={empty} onclick={copyImage}>
          <Typeset text={copyLabel('image', 'Copy image')} />
        </button>
        <button class="copy" type="button" disabled={empty} onclick={copySvg}>
          <Typeset text={copyLabel('svg', 'Copy SVG')} />
        </button>
      </div>
    </div>
  </section>
</main>
</div>

<style>
  .page {
    height: 100%;
  }

  main {
    display: grid;
    grid-template-rows: auto minmax(240px, 1fr) auto;
    min-height: 100%;
  }

  header {
    display: flex;
    gap: var(--s3);
    align-items: baseline;
    padding: var(--s3) 0;
  }

  h1 {
    flex: 1;
    margin: 0;
    font-size: 32px;
    font-weight: inherit;
    line-height: 1;
  }

  header p {
    margin: 0;
    font-size: 14px;
    line-height: 1;
    color: var(--quiet);
    white-space: nowrap;
  }

  .full {
    align-self: center;
    padding: var(--s1) var(--s2);
    border: 1px solid var(--ink);
    color: inherit;
    line-height: 1;
    text-decoration: none;
  }

  .full:hover {
    background: var(--ink);
    color: var(--sheet);
  }

  .stage {
    position: relative;
  }

  .sheet {
    position: absolute;
    inset: 0 0 auto;
    display: flex;
    overflow: auto;
    padding: var(--s4);
    border: 2px solid var(--ink);
    background-color: var(--paper);
    background-image:
      linear-gradient(var(--rule-strong) 1px, transparent 1px),
      linear-gradient(90deg, var(--rule-strong) 1px, transparent 1px),
      linear-gradient(var(--rule) 1px, transparent 1px),
      linear-gradient(90deg, var(--rule) 1px, transparent 1px);
    background-size:
      120px 120px,
      120px 120px,
      24px 24px,
      24px 24px;
    background-position: -1px -1px;
  }

  .formula {
    margin: auto;
    line-height: 1;
  }

  .stale {
    color: var(--stale);
  }

  .hint {
    font-size: 24px;
    color: var(--green);
  }

  .desk {
    display: grid;
    gap: var(--s3);
    padding: var(--s3) 0 var(--s4);
  }

  .source {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: var(--s1) var(--s3);
    align-items: center;
  }

  .problem {
    height: var(--s5);
    margin: 0;
    line-height: var(--s5);
    color: var(--red);
    text-align: right;
  }

  textarea {
    grid-column: 1 / -1;
    width: 100%;
    padding: var(--s2) var(--s3);
    border: 2px solid var(--ink);
    border-radius: 0;
    background: var(--sheet);
    font-family: ui-monospace, 'SF Mono', Menlo, Consolas, monospace;
    font-size: 16px;
    line-height: 1.5;
    resize: vertical;
  }

  .tools {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s3) var(--s4);
    align-items: center;
    justify-content: space-between;
  }

  .examples {
    display: flex;
    flex: 1 1 480px;
    flex-wrap: wrap;
    gap: var(--s2);
  }

  .example {
    display: flex;
    align-items: center;
    min-height: 48px;
    padding: var(--s1) var(--s3);
    border: 1px solid var(--rule-strong);
    border-radius: 0;
    background: transparent;
    font-size: 15px;
    cursor: pointer;
  }

  .example:hover {
    border-color: var(--ink);
    background: var(--sheet);
  }

  .example:active {
    background: var(--rule);
  }

  .example.current {
    border-color: var(--ink);
    box-shadow: inset 0 -3px 0 var(--green);
  }

  .actions {
    display: flex;
    gap: var(--s3);
    align-items: center;
  }

  fieldset {
    display: flex;
    margin: 0;
    padding: 0;
    border: 2px solid var(--ink);
  }

  .mode {
    position: relative;
    display: flex;
    align-items: center;
    min-height: 44px;
    padding: 0 var(--s3);
    cursor: pointer;
  }

  .mode:hover {
    background: var(--sheet);
  }

  .mode.on {
    background: var(--ink);
    color: var(--sheet);
  }

  .mode input {
    position: absolute;
    inset: 0;
    margin: 0;
    opacity: 0;
    cursor: pointer;
  }

  .mode:has(input:focus-visible) {
    outline: 3px solid var(--green);
    outline-offset: 2px;
  }

  .copy {
    min-width: 136px;
    min-height: 48px;
    margin-bottom: var(--s1);
    padding: 0 var(--s4);
    border: 0;
    border-radius: 0;
    background: var(--green);
    box-shadow: 0 var(--s1) 0 var(--green-deep);
    color: var(--sheet);
    cursor: pointer;
  }

  .copy:hover {
    background: var(--green-deep);
    box-shadow: 0 var(--s1) 0 var(--ink);
  }

  .copy:active {
    box-shadow: none;
    transform: translateY(var(--s1));
  }

  .copy:disabled {
    background: var(--stale);
    box-shadow: none;
    cursor: not-allowed;
  }

  @media (max-width: 640px) {
    .tools {
      flex-direction: column;
      align-items: stretch;
    }

    .examples {
      flex-basis: auto;
    }

    .actions {
      flex-wrap: wrap;
      justify-content: space-between;
    }

    .copy {
      flex: 1;
    }
  }
</style>
