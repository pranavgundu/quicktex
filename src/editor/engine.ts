export type Sheet = [page: number, svg: string]

export type Request =
  | { kind: 'compile'; id: number; tex: string }
  | { kind: 'time'; id: number; micros: number }
  | { kind: 'watch'; id: number; pages: number[] }
  | { kind: 'pdf'; id: number }

export type Compiled = {
  kind: 'compiled'
  id: number
  pages: number
  micros: number
  warningCount: number
  warningAt: number | null
  sheets: Sheet[]
}

export type Failed = { kind: 'failed'; id: number; message: string; at: number; line: number }
export type Timed = { kind: 'timed'; id: number; micros: number; pdfMicros: number }
export type Answer = Compiled | Failed | Timed | { kind: 'sheets'; id: number; sheets: Sheet[] } | { kind: 'pdf'; id: number; bytes: Uint8Array }

export type Port = {
  postMessage(request: Request): void
  onmessage: ((event: MessageEvent<Answer>) => void) | null
  onerror: ((event: ErrorEvent) => void) | null
}

export type Listeners = {
  compiled(answer: Compiled): void
  failed(answer: Failed): void
  timed(answer: Timed): void
  sheets(sheets: Sheet[]): void
  broken(): void
}

export function connect(port: Port, on: Listeners) {
  let next = 0
  let running = false
  let waiting: string | null = null
  let timing: number | null = null
  const printing: ((bytes: Uint8Array) => void)[] = []

  function send(request: Request) {
    port.postMessage(request)
  }

  function advance() {
    if (running) return
    if (waiting !== null) {
      const tex = waiting
      waiting = null
      running = true
      send({ kind: 'compile', id: ++next, tex })
    } else if (timing !== null) {
      const micros = timing
      timing = null
      running = true
      send({ kind: 'time', id: ++next, micros })
    }
  }

  port.onmessage = ({ data }) => {
    if (data.kind === 'sheets') {
      on.sheets(data.sheets)
      return
    }
    if (data.kind === 'pdf') {
      printing.shift()?.(data.bytes)
      return
    }
    running = false
    if (data.kind === 'compiled') on.compiled(data)
    else if (data.kind === 'failed') on.failed(data)
    else if (waiting === null) on.timed(data)
    advance()
  }

  port.onerror = () => on.broken()

  return {
    compile(tex: string) {
      waiting = tex
      timing = null
      advance()
    },
    time(micros: number) {
      if (waiting !== null) return
      timing = micros
      advance()
    },
    watch(pages: number[]) {
      send({ kind: 'watch', id: ++next, pages })
    },
    pdf() {
      return new Promise<Uint8Array>((resolve) => {
        printing.push(resolve)
        send({ kind: 'pdf', id: ++next })
      })
    },
  }
}
