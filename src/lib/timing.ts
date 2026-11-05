const budget = 2

export function measure(run: () => unknown) {
  const start = performance.now()
  let runs = 0
  let elapsed = 0
  do {
    run()
    runs++
    elapsed = performance.now() - start
  } while (elapsed < budget)
  return (elapsed * 1000) / runs
}

export function timeText(micros: number) {
  if (micros >= 1000) return `${(micros / 1000).toFixed(1)} ms`
  return `${micros.toFixed(micros < 10 ? 1 : 0)} µs`
}

export function timeTex(micros: number) {
  if (micros >= 1000) return `${(micros / 1000).toFixed(1)} \\text{ ms}`
  return `${micros.toFixed(micros < 10 ? 1 : 0)} \\,\\mu\\text{s}`
}
