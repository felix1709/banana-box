const canvas = document.getElementById('canvas')
const status = document.getElementById('status')
const rotateButton = document.getElementById('rotate')
const cropButton = document.getElementById('crop')
const resetButton = document.getElementById('reset')
const copyButton = document.getElementById('copy')
const sendButton = document.getElementById('send')
const wrap = document.querySelector('.canvas-wrap')

const ctx = canvas.getContext('2d')

let originalDataUrl = ''
let sourceImage = null
let fullCanvas = null
let angle = 0
let selection = null
let scale = 1
let offsetX = 0
let offsetY = 0
let dragging = false
let dragStart = null

chrome.storage.local.get('atlasCapture', async ({ atlasCapture }) => {
  if (!atlasCapture || !atlasCapture.dataUrl) {
    status.textContent = '没有可预览的截图'
    return
  }
  originalDataUrl = atlasCapture.dataUrl
  try {
    sourceImage = await loadImage(originalDataUrl)
    buildFullCanvas()
    draw()
  } catch {
    status.textContent = '截图加载失败'
  }
})

function loadImage(src) {
  return new Promise((resolve, reject) => {
    const image = new Image()
    image.onload = () => resolve(image)
    image.onerror = reject
    image.src = src
  })
}

function buildFullCanvas() {
  const baseW = sourceImage.naturalWidth
  const baseH = sourceImage.naturalHeight
  const swap = angle % 180 !== 0
  const width = swap ? baseH : baseW
  const height = swap ? baseW : baseH

  fullCanvas = document.createElement('canvas')
  fullCanvas.width = width
  fullCanvas.height = height
  const fctx = fullCanvas.getContext('2d')
  fctx.save()
  fctx.translate(width / 2, height / 2)
  fctx.rotate((angle * Math.PI) / 180)
  fctx.drawImage(sourceImage, -baseW / 2, -baseH / 2)
  fctx.restore()
}

function draw() {
  const maxWidth = Math.max(240, wrap.clientWidth - 32)
  const maxHeight = Math.max(160, wrap.clientHeight - 32)
  const imageWidth = fullCanvas.width
  const imageHeight = fullCanvas.height
  scale = Math.min(maxWidth / imageWidth, maxHeight / imageHeight, 1)
  const displayWidth = imageWidth * scale
  const displayHeight = imageHeight * scale

  canvas.width = maxWidth
  canvas.height = maxHeight
  offsetX = (maxWidth - displayWidth) / 2
  offsetY = (maxHeight - displayHeight) / 2

  ctx.clearRect(0, 0, canvas.width, canvas.height)
  ctx.drawImage(fullCanvas, offsetX, offsetY, displayWidth, displayHeight)
  drawSelection()
}

function drawSelection() {
  if (!selection) return
  ctx.save()
  ctx.strokeStyle = '#66f7d3'
  ctx.lineWidth = 2
  ctx.fillStyle = 'rgba(102, 247, 211, 0.14)'
  ctx.fillRect(selection.x, selection.y, selection.width, selection.height)
  ctx.strokeRect(selection.x, selection.y, selection.width, selection.height)
  ctx.restore()
}

function canvasPoint(event) {
  const rect = canvas.getBoundingClientRect()
  return {
    x: event.clientX - rect.left,
    y: event.clientY - rect.top,
  }
}

canvas.addEventListener('mousedown', (event) => {
  dragging = true
  dragStart = canvasPoint(event)
  selection = { x: dragStart.x, y: dragStart.y, width: 0, height: 0 }
})

canvas.addEventListener('mousemove', (event) => {
  if (!dragging) return
  const point = canvasPoint(event)
  selection = {
    x: Math.min(dragStart.x, point.x),
    y: Math.min(dragStart.y, point.y),
    width: Math.abs(point.x - dragStart.x),
    height: Math.abs(point.y - dragStart.y),
  }
  draw()
})

canvas.addEventListener('mouseup', () => {
  if (!dragging) return
  dragging = false
  if (!selection || selection.width < 10 || selection.height < 10) {
    selection = null
  }
  draw()
})

rotateButton.addEventListener('click', () => {
  if (!sourceImage) return
  angle = (angle + 90) % 360
  selection = null
  buildFullCanvas()
  draw()
  status.textContent = `已旋转 ${angle}°`
})

cropButton.addEventListener('click', () => {
  if (!fullCanvas) return
  if (!selection) {
    status.textContent = '请先在画面上拖动选择裁剪区域'
    return
  }

  const sourceX = Math.max(0, Math.min(fullCanvas.width, (selection.x - offsetX) / scale))
  const sourceY = Math.max(0, Math.min(fullCanvas.height, (selection.y - offsetY) / scale))
  const sourceWidth = Math.max(1, Math.min(fullCanvas.width - sourceX, selection.width / scale))
  const sourceHeight = Math.max(1, Math.min(fullCanvas.height - sourceY, selection.height / scale))

  const cropped = document.createElement('canvas')
  cropped.width = Math.round(sourceWidth)
  cropped.height = Math.round(sourceHeight)
  const croppedCtx = cropped.getContext('2d')
  croppedCtx.drawImage(
    fullCanvas,
    sourceX,
    sourceY,
    sourceWidth,
    sourceHeight,
    0,
    0,
    cropped.width,
    cropped.height,
  )

  sourceImage = cropped
  angle = 0
  selection = null
  buildFullCanvas()
  draw()
  status.textContent = '已应用裁剪'
})

resetButton.addEventListener('click', async () => {
  if (!originalDataUrl) return
  sourceImage = await loadImage(originalDataUrl)
  angle = 0
  selection = null
  buildFullCanvas()
  draw()
  status.textContent = '已重置'
})

copyButton.addEventListener('click', () => {
  if (!fullCanvas) return
  fullCanvas.toBlob((blob) => {
    if (!blob) {
      status.textContent = '复制失败'
      return
    }
    navigator.clipboard
      .write([new ClipboardItem({ 'image/png': blob })])
      .then(() => {
        status.textContent = '已复制到剪贴板'
      })
      .catch(() => {
        status.textContent = '复制失败，请检查浏览器剪贴板权限'
      })
  }, 'image/png')
})

sendButton.addEventListener('click', () => {
  if (!fullCanvas) return
  sendButton.disabled = true
  status.textContent = '正在传送到 Banana Box…'
  const dataUrl = fullCanvas.toDataURL('image/png')
  chrome.runtime.sendMessage(
    { type: 'ATLAS_INGEST_ITEMS', items: [{ imageBase64: dataUrl }] },
    (response) => {
      sendButton.disabled = false
      if (chrome.runtime.lastError) {
        status.textContent = '传送失败，请重试'
        return
      }
      if (response && response.failed === 0) {
        status.textContent = '已传送到 Banana Box，正在分析入库'
        window.setTimeout(() => window.close(), 900)
      } else {
        status.textContent = '传送失败，请确认 Banana Box 服务已开启'
      }
    },
  )
})
