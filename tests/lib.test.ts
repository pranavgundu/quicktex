import { beforeEach, describe, expect, test } from 'bun:test'
import { charIndex } from '../src/lib/latex'
import { remember, remembered } from '../src/lib/storage'
import { measure, timeTex, timeText } from '../src/lib/timing'

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
