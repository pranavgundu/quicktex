const namespace = 'http://www.w3.org/2000/svg'

type Shown = { header: string; parts: Map<string, Element> }

function draw(host: HTMLElement, source: string): Shown | null {
  if (!source) {
    host.replaceChildren()
    return null
  }
  const lines = source.split('\n')
  host.innerHTML = lines.join('')
  const parts = new Map<string, Element>()
  const children = host.firstElementChild?.children ?? []
  for (let i = 0; i < children.length; i++) parts.set(lines[i + 1], children[i])
  return { header: lines[0], parts }
}

function update(host: HTMLElement, source: string, shown: Shown) {
  const lines = source.split('\n')
  const svg = host.firstElementChild as Element
  const wanted: (Element | null)[] = []
  const kept = new Set<Element>()
  const missing: string[] = []
  const end = lines.length - 1
  for (let i = 1; i < end; i++) {
    const found = shown.parts.get(lines[i])
    if (found && !kept.has(found)) {
      kept.add(found)
      wanted.push(found)
    } else {
      missing.push(lines[i])
      wanted.push(null)
    }
  }
  const scratch = document.createElementNS(namespace, 'svg')
  scratch.innerHTML = missing.join('')
  const parts = new Map<string, Element>()
  let cursor = svg.firstElementChild
  for (let i = 0; i < wanted.length; i++) {
    const element = wanted[i] ?? (scratch.firstElementChild as Element)
    while (cursor && cursor !== element && !kept.has(cursor)) {
      const next: Element | null = cursor.nextElementSibling
      cursor.remove()
      cursor = next
    }
    if (cursor === element) cursor = cursor.nextElementSibling
    else svg.insertBefore(element, cursor)
    parts.set(lines[i + 1], element)
  }
  while (cursor) {
    const next: Element | null = cursor.nextElementSibling
    cursor.remove()
    cursor = next
  }
  shown.parts = parts
}

export function showPage(host: HTMLElement, first: string) {
  let source = first
  let shown = draw(host, source)
  return {
    update(next: string) {
      if (next === source) return
      source = next
      if (shown && next.startsWith(shown.header)) update(host, next, shown)
      else shown = draw(host, next)
    },
  }
}
