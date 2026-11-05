import { afterAll, beforeAll, beforeEach, describe, expect, test } from 'bun:test'
import type { Browser } from 'playwright-core'
import { dist, launch, serve, session, type Session } from '../support/browser'
import { article } from '../support/engine'

const site = serve(dist)
let browser: Browser
let s: Session

const content = () => s.page.locator('.cm-content')
const popup = () => s.page.locator('.cm-tooltip-autocomplete')
const labels = () => popup().locator('.cm-completionLabel').allInnerTexts()
const status = () => s.page.locator('[role=status]').first().innerText()

async function open(tex: string, marker = '|') {
  const at = tex.indexOf(marker)
  const doc = tex.replace(marker, '')
  await s.page.goto(`${site.url}editor/`)
  await s.page.waitForSelector('.cm-content')
  await s.page.evaluate((value) => localStorage.setItem('quicktex-document', value), doc)
  await s.page.reload()
  await s.page.waitForSelector('.cm-content')
  await s.page.waitForTimeout(100)
  await content().click()
  await s.page.keyboard.press('ControlOrMeta+Home')
  for (let i = 0; i < at; i++) await s.page.keyboard.press('ArrowRight')
}

async function text() {
  await s.page.keyboard.press('ControlOrMeta+s')
  return s.page.evaluate(() => localStorage.getItem('quicktex-document')!)
}

async function suggest(typed: string) {
  await s.page.keyboard.type(typed)
  await popup().waitFor()
  await s.page.waitForTimeout(120)
}

async function accept(typed: string) {
  await suggest(typed)
  await s.page.keyboard.press('Enter')
}

const key = (name: string) => s.page.keyboard.press(name)
const type = (value: string) => s.page.keyboard.type(value)
const body = (inner: string) => article(inner)
const clean = () => expect(s.problems).toEqual([])

beforeAll(async () => {
  browser = await launch()
})

afterAll(async () => {
  await browser.close()
  site.stop()
})

beforeEach(async () => {
  s = await session(browser)
})

describe('completing commands', () => {
  test('fills a command and walks its fields', async () => {
    await open(body('$|$'))
    await accept('\\fra')
    await type('1')
    await key('Tab')
    await type('2')
    await key('Tab')
    await type('x')
    expect(await text()).toBe(body('$\\frac{1}{2}x$'))
    clean()
  })

  test('tab accepts as well as enter', async () => {
    await open(body('$|$'))
    await suggest('\\alph')
    await key('Tab')
    expect(await text()).toBe(body('$\\alpha$'))
  })

  test('tab accepts inside a field before moving on', async () => {
    await open(body('$|$'))
    await accept('\\fra')
    await suggest('\\alph')
    await key('Tab')
    await key('Tab')
    await type('b')
    expect(await text()).toBe(body('$\\frac{\\alpha}{b}$'))
  })

  test('shows math only in math', async () => {
    await open(body('$|$\n\nprose '))
    await suggest('\\alp')
    expect(await labels()).toContain('\\alpha')
    await key('Escape')
    await key('ControlOrMeta+End')
    await key('ArrowUp')
    await key('ArrowUp')
    await key('End')
    await type(' \\sec')
    await popup().waitFor()
    const shown = await labels()
    expect(shown).toContain('\\section')
    expect(shown).not.toContain('\\alpha')
  })

  test('draws symbols in the list', async () => {
    await open(body('$|$'))
    await suggest('\\alph')
    expect(await popup().locator('.cm-completionPreview svg').count()).toBeGreaterThan(0)
    clean()
  })

  test('stays quiet in comments and after a line break', async () => {
    await open(body('% |'))
    await type('\\sec')
    await s.page.waitForTimeout(250)
    expect(await popup().count()).toBe(0)
    await open(body('a |'))
    await type('\\\\sec')
    await s.page.waitForTimeout(250)
    expect(await popup().count()).toBe(0)
  })

  test('stays quiet in plain prose', async () => {
    await open(body('|'))
    await type('plain words here')
    await s.page.waitForTimeout(250)
    expect(await popup().count()).toBe(0)
  })

  test('enter after a full command starts a new line', async () => {
    await open(body('|'))
    await suggest('\\maketitle')
    await key('Enter')
    await type('x')
    expect(await text()).toBe(body('\\maketitle\nx'))
  })

  test('offers macros from the document', async () => {
    await open(`\\documentclass{article}\n\\newcommand{\\norm}[1]{|#1|}\n\\begin{document}\n$~$\n\\end{document}\n`, '~')
    await accept('\\nor')
    await type('v')
    const result = await text()
    expect(result).toContain('$\\norm{v}$')
    await s.page.waitForTimeout(100)
    expect(await status()).not.toContain('nknown')
  })

  test('loads the package a command needs, once', async () => {
    const start = '\\documentclass{article}\n\\usepackage{amsmath}\n\\begin{document}\n$~$\n\\end{document}\n'
    await open(start, '~')
    await accept('\\dv')
    await type('f')
    await key('Tab')
    await type('x')
    await key('Tab')
    await type(' ')
    await accept('\\dv')
    await key('Escape')
    const result = await text()
    expect(result.split('\\usepackage{physics}').length).toBe(2)
    expect(result.indexOf('\\usepackage{physics}')).toBeGreaterThan(result.indexOf('\\usepackage{amsmath}'))
    expect(result.indexOf('\\usepackage{physics}')).toBeLessThan(result.indexOf('\\begin{document}'))
    expect(result).toContain('\\dv{f}{x}')
  })
})

describe('completing arguments', () => {
  test('fills an environment and drops the closing brace', async () => {
    await open(body('|'))
    await type('\\begin{')
    await accept('ite')
    await type('first')
    expect(await text()).toBe(body('\\begin{itemize}\n  \\item first\n\\end{itemize}'))
    await s.page.waitForTimeout(100)
    expect(await status()).toContain('page')
    clean()
  })

  test('uses math environments in math', async () => {
    await open(body('\\[ | \\]'))
    await type('\\begin{')
    await popup().waitFor()
    expect(await labels()).toContain('pmatrix')
    expect(await labels()).not.toContain('itemize')
  })

  test('offers whole environments from the command list', async () => {
    await open(body('|'))
    await accept('\\begin')
    expect(await text()).toContain('\\begin{')
    expect(await text()).toContain('\\end{')
  })

  test('renames without adding a second end', async () => {
    await open(body('\\begin{ite|}\n\\item a\n\\end{ite}'))
    await key('Backspace')
    await s.page.waitForTimeout(50)
    await type('e')
    await popup().waitFor()
    await s.page.waitForTimeout(120)
    await key('Enter')
    const result = await text()
    expect(result.split('\\end{').length).toBe(3)
    expect(result).toContain('\\begin{itemize}\n')
  })

  test('closes the innermost environment', async () => {
    await open(body('\\begin{itemize}\n\\begin{center}\n|'))
    await type('\\end{')
    await popup().waitFor()
    expect(await labels()).toEqual(['center'])
  })

  test('names packages, including after a comma', async () => {
    await open('\\documentclass{article}\n\\usepackage{amsmath, |}\n\\begin{document}\nx\n\\end{document}\n')
    await accept('physi')
    expect(await text()).toContain('\\usepackage{amsmath, physics}\n')
  })

  test('names colors, including your own', async () => {
    await open('\\documentclass{article}\n\\definecolor{brand}{HTML}{336699}\n\\begin{document}\n$~$\n\\end{document}\n', '~')
    await type('\\textcolor{')
    await popup().waitFor()
    const shown = await labels()
    expect(shown).toContain('red')
    expect(shown).toContain('brand')
  })

  test('names drawing options and their values', async () => {
    await open(body('\\begin{tikzpicture}\n\\draw[|] (0,0) -- (1,1);\n\\end{tikzpicture}'))
    await accept('dash')
    await type(', fill=')
    await popup().waitFor()
    expect(await labels()).toContain('gray')
    await key('Escape')
    expect(await text()).toContain('\\draw[dashed, fill=]')
  })

  test('names plot options', async () => {
    await open(body('\\begin{tikzpicture}\n\\begin{axis}[|]\n\\addplot coordinates {(0,0) (1,1)};\n\\end{axis}\n\\end{tikzpicture}'))
    await accept('xlab')
    await type('time')
    expect(await text()).toContain('\\begin{axis}[xlabel=time]')
  })
})

describe('pairs', () => {
  test('closes and types over a dollar', async () => {
    await open(body('|'))
    await type('$x$ y')
    expect(await text()).toBe(body('$x$ y'))
  })

  test('opens display dollars', async () => {
    await open(body('|'))
    await type('$$x')
    expect(await text()).toBe(body('$$x$$'))
  })

  test('leaves an escaped dollar alone', async () => {
    await open(body('|'))
    await type('\\$5')
    expect(await text()).toBe(body('\\$5'))
  })

  test('wraps a selection in dollars', async () => {
    await open(body('|abc'))
    await key('Shift+ArrowRight')
    await key('Shift+ArrowRight')
    await key('Shift+ArrowRight')
    await type('$')
    expect(await text()).toBe(body('$abc$'))
  })

  test('closes a brace before a dollar', async () => {
    await open(body('$x^|$'))
    await type('{2')
    expect(await text()).toBe(body('$x^{2}$'))
  })

  test('closes escaped delimiters', async () => {
    await open(body('|'))
    await type('\\[x')
    await key('Escape')
    expect(await text()).toBe(body('\\[x\\]'))
    await open(body('$|$'))
    await type('\\{x')
    await key('Escape')
    expect(await text()).toBe(body('$\\{x\\}$'))
  })

  test('types over the closing delimiter', async () => {
    await open(body('|'))
    await type('\\[x\\]')
    await key('Escape')
    await type(' y')
    expect(await text()).toBe(body('\\[x\\] y'))
  })

  test('leaves a line break with a length alone', async () => {
    await open(body('|'))
    await type('a \\\\[')
    await key('Escape')
    expect(await text()).toBe(body('a \\\\['))
  })

  test('backspace removes an empty pair', async () => {
    await open(body('|'))
    await type('\\[')
    await key('Escape')
    await key('Backspace')
    expect(await text()).toBe(body(''))
  })

  test('enter opens display math', async () => {
    await open(body('|'))
    await type('\\[')
    await key('Escape')
    await key('Enter')
    await type('x')
    expect(await text()).toBe(body('\\[\n  x\n\\]'))
  })
})

describe('enter', () => {
  test('finishes an environment', async () => {
    await open(body('|'))
    await type('\\begin{center}')
    await key('Escape')
    await key('Enter')
    await type('x')
    expect(await text()).toBe(body('\\begin{center}\n  x\n\\end{center}'))
  })

  test('adds the first item of a list', async () => {
    await open(body('|'))
    await type('\\begin{enumerate}')
    await key('Escape')
    await key('Enter')
    await type('x')
    expect(await text()).toBe(body('\\begin{enumerate}\n  \\item x\n\\end{enumerate}'))
  })

  test('does not indent the document', async () => {
    await open('|')
    await type('\\begin{document}')
    await key('Escape')
    await key('Enter')
    await type('x')
    expect(await text()).toBe('\\begin{document}\nx\n\\end{document}')
  })

  test('does not add a second end', async () => {
    await open(body('\\begin{center}|\nx\n\\end{center}'))
    await key('Enter')
    expect((await text()).split('\\end{center}').length).toBe(2)
  })

  test('continues a list and ends it on an empty item', async () => {
    await open(body('\\begin{itemize}\n  \\item a|\n\\end{itemize}'))
    await key('Enter')
    await type('b')
    await key('Enter')
    expect(await text()).toBe(body('\\begin{itemize}\n  \\item a\n  \\item b\n  \\item \n\\end{itemize}'))
    await key('Enter')
    expect(await text()).toBe(body('\\begin{itemize}\n  \\item a\n  \\item b\n\n\\end{itemize}'))
  })

  test('keeps indent inside an environment', async () => {
    await open(body('\\begin{center}\n  a|\n\\end{center}'))
    await key('Enter')
    await type('b')
    expect(await text()).toBe(body('\\begin{center}\n  a\n  b\n\\end{center}'))
  })

  test('moves a typed end back to its begin', async () => {
    await open(body('\\begin{center}\n  a\n  |'))
    await type('\\end{center')
    await key('Escape')
    await type('}')
    expect(await text()).toBe(body('\\begin{center}\n  a\n\\end{center}'))
  })
})

describe('shortcuts', () => {
  test('bold and italic wrap text', async () => {
    await open(body('|abc'))
    await key('Shift+ArrowRight')
    await key('Shift+ArrowRight')
    await key('Shift+ArrowRight')
    await key('ControlOrMeta+b')
    expect(await text()).toBe(body('\\textbf{abc}'))
    await key('ControlOrMeta+i')
    expect(await text()).toBe(body('\\textbf{\\textit{abc}}'))
  })

  test('pressing again removes the wrapper', async () => {
    await open(body('|abc'))
    await key('Shift+ArrowRight')
    await key('Shift+ArrowRight')
    await key('Shift+ArrowRight')
    await key('ControlOrMeta+b')
    await key('ControlOrMeta+b')
    expect(await text()).toBe(body('abc'))
  })

  test('math gets math fonts', async () => {
    await open(body('$|x$'))
    await key('Shift+ArrowRight')
    await key('ControlOrMeta+b')
    expect(await text()).toBe(body('$\\mathbf{x}$'))
  })

  test('bold with nothing selected leaves the cursor inside', async () => {
    await open(body('|'))
    await key('ControlOrMeta+b')
    await type('x')
    expect(await text()).toBe(body('\\textbf{x}'))
  })

  test('toggles a comment', async () => {
    await open(body('abc|'))
    await key('ControlOrMeta+/')
    expect(await text()).toBe(body('% abc'))
    await key('ControlOrMeta+/')
    expect(await text()).toBe(body('abc'))
  })

  test('save writes straight away', async () => {
    await open(body('|'))
    await type('x')
    await key('ControlOrMeta+s')
    expect(await s.page.evaluate(() => localStorage.getItem('quicktex-document'))).toBe(body('x'))
  })

  test('selects the next match for several cursors', async () => {
    await open(body('|cat cat'))
    await key('Shift+ArrowRight')
    await key('Shift+ArrowRight')
    await key('Shift+ArrowRight')
    await key('ControlOrMeta+d')
    await type('dog')
    expect(await text()).toBe(body('dog dog'))
  })

  test('undo reverses a completion in one step', async () => {
    await open(body('$|$'))
    await type(' ')
    await s.page.waitForTimeout(600)
    await accept('\\fra')
    await key('ControlOrMeta+z')
    expect(await text()).toBe(body('$ \\fra$'))
  })
})

describe('folding', () => {
  const lines = (count: number) => Array.from({ length: count }, (_, i) => `line${i}`).join('\n')

  test('folds an environment', async () => {
    await open(body(`\\begin{center}\n${lines(4)}\n\\end{center}\nafter|`))
    await s.page.locator('.cm-foldGutter [title=Fold]').first().click()
    await s.page.waitForSelector('.cm-foldPlaceholder')
    const shown = await content().innerText()
    expect(shown).not.toContain('line2')
    expect(shown).toContain('\\end{center}')
    expect(shown).toContain('after')
    expect(await text()).toContain('line2')
  })

  test('unfolds again', async () => {
    await open(body(`\\begin{center}\n${lines(4)}\n\\end{center}|`))
    await s.page.locator('.cm-foldGutter [title=Fold]').first().click()
    await s.page.locator('.cm-foldGutter [title=Unfold]').last().click()
    expect(await content().innerText()).toContain('line2')
  })

  test('folds a section up to the next one', async () => {
    await open(body(`\\section{A}\n${lines(3)}\n\\subsection{Inner}\nx\n\n\\section{B}\nkept|`))
    await s.page.locator('.cm-foldGutter [title=Fold]').first().click()
    const shown = await content().innerText()
    expect(shown).not.toContain('line1')
    expect(shown).not.toContain('Inner')
    expect(shown).toContain('\\section{B}')
    expect(shown).toContain('kept')
  })

  test('skips nested environments of the same name', async () => {
    await open(body(`\\begin{itemize}\n\\item a\n\\begin{itemize}\n\\item b\n\\end{itemize}\n\\item c\n\\end{itemize}\nafter|`))
    await s.page.locator('.cm-foldGutter [title=Fold]').first().click()
    const shown = await content().innerText()
    expect(shown).not.toContain('\\item c')
    expect(shown).toContain('after')
  })

  test('does not offer the document itself', async () => {
    await open(body('one\ntwo|'))
    expect(await s.page.locator('.cm-foldGutter [title=Fold]').count()).toBe(0)
  })
})

describe('matching environments', () => {
  const marks = () => s.page.locator('.cm-matchingTag').allInnerTexts()

  test('marks the pair from the begin', async () => {
    await open(body('\\begin{cen|ter}\na\n\\end{center}'))
    await s.page.waitForSelector('.cm-matchingTag')
    expect(await marks()).toEqual(['\\begin{center}', '\\end{center}'])
  })

  test('marks the pair from the end across nesting', async () => {
    await open(body('\\begin{center}\n\\begin{center}\n\\end{center}\n\\end{cen|ter}'))
    await s.page.waitForSelector('.cm-matchingTag')
    const first = await s.page.locator('.cm-line:has(.cm-matchingTag)').first().innerText()
    expect(first).toBe('\\begin{center}')
    expect(await s.page.locator('.cm-line:has(.cm-matchingTag)').count()).toBe(2)
    const lines = await s.page.locator('.cm-line').allInnerTexts()
    expect(lines.indexOf('\\begin{center}')).toBe(2)
  })

  test('marks a begin with no end', async () => {
    await open(`\\begin{ce|nter}\n`)
    await s.page.waitForSelector('.cm-missingTag')
  })

  test('marks nothing elsewhere', async () => {
    await open(body('te|xt'))
    expect(await s.page.locator('.cm-matchingTag').count()).toBe(0)
  })
})

describe('outline', () => {
  const tex = body('\\section{First}\na\n\\subsection{Second}\nb\n\\section{Third}\nc|')
  const button = () => s.page.getByRole('button', { name: 'Outline' })
  const items = () => s.page.locator('li button')

  test('lists sections and marks the current one', async () => {
    await open(tex)
    await button().click()
    expect(await items().allInnerTexts()).toEqual(['First', 'Second', 'Third'])
    expect(await s.page.locator('[aria-current=location]').innerText()).toBe('Third')
    clean()
  })

  test('jumps to a section', async () => {
    await open(tex)
    await button().click()
    await items().nth(1).click()
    await type('X')
    expect(await text()).toContain('X\\subsection{Second}')
    expect(await items().count()).toBe(0)
  })

  test('works from the keyboard', async () => {
    await open(tex)
    await button().focus()
    await key('Enter')
    await s.page.waitForTimeout(50)
    await key('ArrowDown')
    await s.page.waitForTimeout(50)
    expect(await s.page.evaluate(() => document.activeElement?.textContent?.trim())).toBe('First')
    await key('Escape')
    expect(await items().count()).toBe(0)
    expect(await s.page.evaluate(() => document.activeElement?.textContent)).toContain('Outline')
  })

  test('closes when you click away', async () => {
    await open(tex)
    await button().click()
    await content().click()
    expect(await items().count()).toBe(0)
  })

  test('says what to do with no sections', async () => {
    await open(body('plain|'))
    await button().click()
    expect(await items().count()).toBe(0)
    expect(await s.page.getByText('to see it here').count()).toBeGreaterThan(0)
    clean()
  })
})

describe('status', () => {
  test('counts words', async () => {
    await open(body('one two three|'))
    expect(await status()).toContain('3 words')
    await type(' four')
    await s.page.waitForTimeout(500)
    expect(await status()).toContain('4 words')
  })

  test('says one word', async () => {
    await open(body('one|'))
    expect(await status()).toContain('1 word,')
  })
})

describe('the popup', () => {
  test('follows the dark theme', async () => {
    await s.context.close()
    s = await session(browser, { colorScheme: 'dark' })
    await open(body('$|$'))
    await suggest('\\alph')
    const colors = await popup().evaluate((node) => {
      const box = getComputedStyle(node)
      const item = getComputedStyle(node.querySelector('li')!)
      return { background: box.backgroundColor, text: item.color }
    })
    expect(colors.background).not.toBe('rgb(255, 255, 255)')
    expect(colors.text).not.toBe(colors.background)
  })

  test('fits a phone', async () => {
    await s.context.close()
    s = await session(browser, { viewport: { width: 375, height: 700 } })
    await open(body('$|$'))
    await suggest('\\alph')
    const box = (await popup().boundingBox())!
    expect(box.x).toBeGreaterThanOrEqual(0)
    expect(box.x + box.width).toBeLessThanOrEqual(375)
  })
})
