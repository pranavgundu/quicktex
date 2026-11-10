import { describe, expect, test } from 'bun:test'
import { createHash } from 'node:crypto'
import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { corpus } from './support/corpus'
import { documentPdf, fixture, pages, render, renderDocument } from './support/engine'

const file = new URL('./golden/output.json', import.meta.url)
const update = process.env.UPDATE_GOLDEN === '1'
const saved: Record<string, string> = existsSync(file) ? JSON.parse(readFileSync(file, 'utf8')) : {}
const fresh: Record<string, string> = {}

const hash = (value: string | Uint8Array) => createHash('sha256').update(value).digest('hex').slice(0, 20)

function compare(name: string, value: string | Uint8Array) {
  const got = hash(value)
  fresh[name] = got
  if (!update) expect(`${name} ${got}`).toBe(`${name} ${saved[name]}`)
}

describe('golden output', () => {
  test('the formula corpus has broad coverage', () => {
    expect(corpus.length).toBeGreaterThan(200)
  })

  // Keep failures independent so one mismatch cannot skip the remaining hashes
  // and cause a misleading failure in the stale-output check.
  for (const tex of corpus) {
    for (const display of [false, true]) {
      const name = `${display ? 'display' : 'inline'} ${tex}`
      test(name, () => compare(name, render(tex, display)))
    }
  }

  for (const name of ['lab', 'shapes', 'sample']) {
    test(`${name} pages match`, () => {
      pages(fixture(name)).forEach((svg, index) => compare(`${name} page ${index + 1}`, svg))
    })

    test(`${name} pdf matches`, () => {
      renderDocument(fixture(name))
      compare(`${name} pdf`, documentPdf())
    })
  }

  test('no recorded output is stale', () => {
    if (update) {
      writeFileSync(file, JSON.stringify(fresh, null, 1) + '\n')
      return
    }
    expect(Object.keys(saved).sort()).toEqual(Object.keys(fresh).sort())
  })
})
