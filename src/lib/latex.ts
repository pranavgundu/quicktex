export function charIndex(text: string, bytes: number) {
  let seen = 0
  let index = 0
  while (index < text.length && seen < bytes) {
    const point = text.codePointAt(index)!
    seen += point < 0x80 ? 1 : point < 0x800 ? 2 : point < 0x10000 ? 3 : 4
    index += point > 0xffff ? 2 : 1
  }
  return index
}
