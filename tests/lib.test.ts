import { beforeEach, describe, expect, test } from 'bun:test'
import { charIndex } from '../src/lib/latex'
import { remember, remembered } from '../src/lib/storage'
import { starter } from '../src/editor/starter'
import { clampZoom, doubleClickZoom, maxZoom, minZoom, nextPreset, savedZoom, wheelFactor } from '../src/editor/zoom'
import { measure, timeTex, timeText } from '../src/lib/timing'
import { renderDocument } from './support/engine'

describe('charIndex', () => {
  const rows: [string, number, number][] = [
    ['abc', 0, 0],
    ['abc', 1, 1],
    ['abc', 3, 3],
    ['abc', 99, 3],
    ['', 0, 0],
    ['é', 0, 0],
    ['é', 2, 1],
    ['éa', 3, 2],
    ['日本', 3, 1],
    ['日本', 6, 2],
    ['😀', 4, 2],
    ['😀a', 5, 3],
    ['a😀b', 1, 1],
    ['a😀b', 5, 3],
    ['a😀b', 6, 4],
  ]
  for (const [text, bytes, index] of rows) {
    test(`${JSON.stringify(text)} at byte ${bytes} is index ${index}`, () => {
      expect(charIndex(text, bytes)).toBe(index)
    })
  }

  test('matches a real encoder for every prefix', () => {
    const text = 'aé日😀z\\frac{α}{β}'
    const encoder = new TextEncoder()
    for (let i = 0; i <= text.length; i++) {
      if (i > 0 && text.charCodeAt(i - 1) >= 0xd800 && text.charCodeAt(i - 1) < 0xdc00) continue
      expect(charIndex(text, encoder.encode(text.slice(0, i)).length)).toBe(i)
    }
  })
})

describe('storage', () => {
  const values = new Map<string, string>()
  const working = {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => void values.set(key, value),
  }
  const broken = {
    getItem() {
      throw new Error('blocked')
    },
    setItem() {
      throw new Error('blocked')
    },
  }

  beforeEach(() => {
    values.clear()
    Object.defineProperty(globalThis, 'localStorage', { value: working, configurable: true })
  })

  test('returns null for an unknown key', () => {
    expect(remembered('missing')).toBeNull()
  })

  test('reads back what it stored', () => {
    remember('key', 'value')
    expect(remembered('key')).toBe('value')
  })

  test('overwrites an old value', () => {
    remember('key', 'one')
    remember('key', 'two')
    expect(remembered('key')).toBe('two')
  })

  test('keeps an empty string', () => {
    remember('key', '')
    expect(remembered('key')).toBe('')
  })

  test('reading survives blocked storage', () => {
    Object.defineProperty(globalThis, 'localStorage', { value: broken, configurable: true })
    expect(remembered('key')).toBeNull()
  })

  test('writing survives blocked storage', () => {
    Object.defineProperty(globalThis, 'localStorage', { value: broken, configurable: true })
    expect(() => remember('key', 'value')).not.toThrow()
  })
})

describe('starter document', () => {
  test('compiles', () => {
    expect(renderDocument(starter)).toBeGreaterThan(0)
  })
})

describe('zoom', () => {
  test('clamps to the range', () => {
    expect(clampZoom(0.01)).toBe(minZoom)
    expect(clampZoom(99)).toBe(maxZoom)
    expect(clampZoom(1.3)).toBe(1.3)
  })

  test('falls back to 100% for nonsense numbers', () => {
    expect(clampZoom(Number.NaN)).toBe(1)
    expect(clampZoom(Infinity)).toBe(1)
  })

  const saved: [string | null, number][] = [
    [null, 1],
    ['', 1],
    ['nonsense', 1],
    ['0', 1],
    ['-2', 1],
    ['1.5', 1.5],
    ['99', maxZoom],
    ['0.01', minZoom],
  ]
  for (const [text, zoom] of saved) {
    test(`saved value ${JSON.stringify(text)} reads as ${zoom}`, () => {
      expect(savedZoom(text)).toBe(zoom)
    })
  }

  test('steps up through every preset to the maximum', () => {
    const seen = [minZoom]
    while (seen.at(-1)! < maxZoom) seen.push(nextPreset(seen.at(-1)!, 1))
    expect(seen).toEqual([0.25, 0.33, 0.5, 0.67, 0.8, 1, 1.25, 1.5, 2, 3, 4, 5])
  })

  test('steps down through every preset to the minimum', () => {
    const seen = [maxZoom]
    while (seen.at(-1)! > minZoom) seen.push(nextPreset(seen.at(-1)!, -1))
    expect(seen.at(-1)).toBe(minZoom)
    expect(seen.length).toBe(12)
  })

  test('a free zoom level steps to the next preset each way', () => {
    expect(nextPreset(1.1, 1)).toBe(1.25)
    expect(nextPreset(1.1, -1)).toBe(1)
  })

  test('stays put at the ends', () => {
    expect(nextPreset(maxZoom, 1)).toBe(maxZoom)
    expect(nextPreset(minZoom, -1)).toBe(minZoom)
  })

  test('scrolling up zooms in and down zooms out', () => {
    expect(wheelFactor(-10, 0)).toBeGreaterThan(1)
    expect(wheelFactor(10, 0)).toBeLessThan(1)
    expect(wheelFactor(0, 0)).toBe(1)
  })

  test('opposite scrolls cancel out', () => {
    expect(wheelFactor(25, 0) * wheelFactor(-25, 0)).toBeCloseTo(1, 10)
  })

  test('a big mouse wheel notch is limited', () => {
    expect(wheelFactor(-5000, 0)).toBe(wheelFactor(-60, 0))
    expect(wheelFactor(5000, 0)).toBe(wheelFactor(60, 0))
  })

  test('line and page scroll modes are converted', () => {
    expect(wheelFactor(1, 1)).toBe(wheelFactor(16, 0))
    expect(wheelFactor(1, 2)).toBe(wheelFactor(60, 0))
  })

  test('double click goes in, then back to 100%', () => {
    expect(doubleClickZoom(1)).toBe(2)
    expect(doubleClickZoom(0.5)).toBe(2)
    expect(doubleClickZoom(2)).toBe(1)
    expect(doubleClickZoom(4)).toBe(1)
  })
})

describe('timing', () => {
  const rows: [number, string, string][] = [
    [0.4, '0.4 µs', '0.4 \\,\\mu\\text{s}'],
    [9.94, '9.9 µs', '9.9 \\,\\mu\\text{s}'],
    [10, '10 µs', '10 \\,\\mu\\text{s}'],
    [84.4, '84 µs', '84 \\,\\mu\\text{s}'],
    [999, '999 µs', '999 \\,\\mu\\text{s}'],
    [1000, '1.0 ms', '1.0 \\text{ ms}'],
    [12345, '12.3 ms', '12.3 \\text{ ms}'],
  ]

  for (const [micros, text, tex] of rows) {
    test(`${micros} reads as ${text}`, () => {
      expect(timeText(micros)).toBe(text)
      expect(timeTex(micros)).toBe(tex)
    })
  }

  test('fast work is averaged over many runs', () => {
    let runs = 0
    const micros = measure(() => runs++)
    expect(runs).toBeGreaterThan(10)
    expect(micros).toBeGreaterThan(0)
    expect(micros).toBeLessThan(500)
  })

  test('slow work runs once', () => {
    let runs = 0
    const micros = measure(() => {
      runs++
      const end = performance.now() + 5
      while (performance.now() < end);
    })
    expect(runs).toBe(1)
    expect(micros).toBeGreaterThanOrEqual(5000)
  })

  test('short work reports more precision than the browser clock', () => {
    expect(timeText(measure(() => 1))).toMatch(/µs$/)
  })
})
