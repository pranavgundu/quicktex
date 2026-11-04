import { readFileSync } from 'node:fs'
import { documentPage, documentPdf, init, render, renderDocument } from '../../wasm-core/js/quicktex.js'

await init(readFileSync(new URL('../../wasm-core/js/quicktex.wasm', import.meta.url)))

export { documentPage, documentPdf, render, renderDocument }

export function fixture(name: string) {
  return readFileSync(new URL(`../fixtures/${name}.tex`, import.meta.url), 'utf8')
}

export function article(body: string) {
  return `\\documentclass{article}\n\\begin{document}\n${body}\n\\end{document}\n`
}

export function pages(tex: string) {
  const count = renderDocument(tex)
  return Array.from({ length: count }, (_, page) => documentPage(page))
}

export function median(values: number[]) {
  return [...values].sort((a, b) => a - b)[values.length >> 1]
}

export function timed(run: () => void, rounds: number) {
  run()
  const times: number[] = []
  for (let i = 0; i < rounds; i++) {
    const start = performance.now()
    run()
    times.push(performance.now() - start)
  }
  return median(times)
}

export function random(seed: number) {
  let state = seed >>> 0
  return (limit: number) => {
    state = (Math.imul(state, 1664525) + 1013904223) >>> 0
    return state % Math.max(limit, 1)
  }
}
