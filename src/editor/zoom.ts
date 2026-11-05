export const minZoom = 0.25
export const maxZoom = 5

const presets = [0.25, 0.33, 0.5, 0.67, 0.8, 1, 1.25, 1.5, 2, 3, 4, 5]
const wheelLimit = 60
const wheelSpeed = 0.01
const lineHeight = 16
const pageHeight = 400

export function clampZoom(zoom: number) {
  return Number.isFinite(zoom) ? Math.min(maxZoom, Math.max(minZoom, zoom)) : 1
}

export function savedZoom(text: string | null) {
  const zoom = Number(text)
  return text && zoom > 0 ? clampZoom(zoom) : 1
}

export function nextPreset(zoom: number, direction: 1 | -1) {
  if (direction > 0) return presets.find((preset) => preset > zoom * 1.001) ?? maxZoom
  return presets.findLast((preset) => preset < zoom * 0.999) ?? minZoom
}

export function wheelFactor(deltaY: number, deltaMode: number) {
  const pixels = deltaMode === 1 ? deltaY * lineHeight : deltaMode === 2 ? deltaY * pageHeight : deltaY
  return Math.exp(-Math.max(-wheelLimit, Math.min(wheelLimit, pixels)) * wheelSpeed)
}

export function doubleClickZoom(zoom: number) {
  return zoom < 1.5 ? 2 : 1
}
