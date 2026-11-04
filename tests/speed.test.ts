import { describe, expect, test } from 'bun:test'
import { article, documentPage, documentPdf, fixture, render, renderDocument, timed } from './support/engine'

const formulas = ['x^2', '\\frac{a}{b}', '\\sum_{i=1}^n i^2', '\\int_0^1 e^{-x^2}\\,dx', '\\begin{pmatrix}a&b\\\\c&d\\end{pmatrix}', '\\sqrt{1+\\sqrt{2}}']

describe('speed budgets', () => {
  test('a formula renders in under 50 microseconds', () => {
    const each = timed(() => {
      for (let i = 0; i < 2000; i++) render(formulas[i % formulas.length], true)
    }, 15)
    expect(each / 2000).toBeLessThan(0.05)
  })

  test('the lab document lays out in under 3 milliseconds', () => {
    const tex = fixture('lab')
    expect(timed(() => void renderDocument(tex), 60)).toBeLessThan(3)
  })

  test('the lab document reaches a page in under 5 milliseconds', () => {
    const tex = fixture('lab')
    expect(
      timed(() => {
        renderDocument(tex)
        documentPage(0)
      }, 60),
    ).toBeLessThan(5)
  })

  test('the lab document reaches a pdf in under 15 milliseconds', () => {
    const tex = fixture('lab')
    expect(
      timed(() => {
        renderDocument(tex)
        documentPdf()
      }, 40),
    ).toBeLessThan(15)
  })

  test('layout time grows with the text and not faster', () => {
    const words = (count: number) => article(Array.from({ length: count }, () => 'alpha beta gamma delta').join(' '))
    const small = words(2000)
    const large = words(20000)
    const a = timed(() => void renderDocument(small), 15)
    const b = timed(() => void renderDocument(large), 7)
    expect(b / a).toBeLessThan(25)
  })
})
