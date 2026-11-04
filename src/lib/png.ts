const padding = 250
const pixelsPerEm = 96
const maxSide = 8000

export async function svgToPng(svg: string) {
  const box = /viewBox="0 (-?\d+) (\d+) (\d+)"/.exec(svg)
  if (!box) throw new Error('SVG has no viewBox')

  const top = Number(box[1]) - padding
  const across = Number(box[2]) + 2 * padding
  const down = Number(box[3]) + 2 * padding
  const scale = Math.min(pixelsPerEm / 1000, maxSide / across, maxSide / down)
  const width = Math.ceil(across * scale)
  const height = Math.ceil(down * scale)

  const sized = svg.replace(
    /viewBox="[^"]*" width="[^"]*" height="[^"]*"/,
    `viewBox="${-padding} ${top} ${across} ${down}" width="${width}" height="${height}"`,
  )
  const image = new Image()
  image.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(sized)}`
  await image.decode()

  const canvas = document.createElement('canvas')
  canvas.width = width
  canvas.height = height
  const context = canvas.getContext('2d')
  if (!context) throw new Error('Canvas is unavailable')
  context.fillStyle = '#fff'
  context.fillRect(0, 0, width, height)
  context.drawImage(image, 0, 0, width, height)

  return new Promise<Blob>((resolve, reject) => {
    canvas.toBlob((blob) => (blob ? resolve(blob) : reject(new Error('PNG encoding failed'))), 'image/png')
  })
}
