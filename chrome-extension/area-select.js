(() => {
  if (document.getElementById('banana-box-area-overlay')) return

  const canvas = document.createElement('canvas')
  canvas.id = 'banana-box-area-overlay'
  Object.assign(canvas.style, {
    position: 'fixed',
    inset: '0',
    width: '100vw',
    height: '100vh',
    zIndex: '2147483647',
    cursor: 'crosshair',
    userSelect: 'none',
  })
  canvas.width = window.innerWidth
  canvas.height = window.innerHeight
  const ctx = canvas.getContext('2d')

  const hint = document.createElement('div')
  hint.textContent = '拖动框选要截图的区域，松开完成'
  Object.assign(hint.style, {
    position: 'fixed',
    top: '18px',
    left: '50%',
    transform: 'translateX(-50%)',
    zIndex: '2147483648',
    padding: '8px 12px',
    borderRadius: '8px',
    background: '#0d1b24',
    color: '#e8f6f2',
    border: '1px solid #66f7d3',
    font: '14px system-ui',
    pointerEvents: 'none',
  })

  document.documentElement.appendChild(canvas)
  document.documentElement.appendChild(hint)

  let start = null
  let selection = null

  function drawOverlay() {
    ctx.clearRect(0, 0, canvas.width, canvas.height)
    ctx.fillStyle = 'rgba(0, 0, 0, 0.38)'
    ctx.fillRect(0, 0, canvas.width, canvas.height)

    if (selection) {
      ctx.clearRect(selection.x, selection.y, selection.width, selection.height)
      ctx.strokeStyle = '#66f7d3'
      ctx.lineWidth = 2
      ctx.strokeRect(selection.x, selection.y, selection.width, selection.height)
    }
  }

  drawOverlay()

  function update(event) {
    if (!start) return
    selection = {
      x: Math.min(start.x, event.clientX),
      y: Math.min(start.y, event.clientY),
      width: Math.abs(event.clientX - start.x),
      height: Math.abs(event.clientY - start.y),
    }
    drawOverlay()
  }

  function finish(event) {
    if (!start) return
    update(event)
    start = null

    if (selection.width < 10 || selection.height < 10) {
      selection = null
      drawOverlay()
      return
    }

    capture(selection)
  }

  canvas.addEventListener('mousedown', (event) => {
    hint.remove()
    start = { x: event.clientX, y: event.clientY }
    update(event)
    event.preventDefault()
  })
  canvas.addEventListener('mousemove', update)
  canvas.addEventListener('mouseup', finish)

  async function capture(rect) {
    try {
      const captureResult = await sendMessage({ type: 'ATLAS_CAPTURE_VISIBLE' })
      if (!captureResult || !captureResult.ok || !captureResult.dataUrl) {
        throw new Error(captureResult?.error || 'capture_failed')
      }
      const insetRect = {
        left: rect.x + 2,
        top: rect.y + 2,
        width: Math.max(1, rect.width - 4),
        height: Math.max(1, rect.height - 4),
      }
      const dataUrl = await cropVisible(captureResult.dataUrl, insetRect)
      canvas.remove()
      chrome.storage.local.set({ atlasCapture: { dataUrl, title: document.title } }, () => {
        chrome.runtime.sendMessage({ type: 'ATLAS_OPEN_PREVIEW' })
      })
    } catch {
      canvas.remove()
      chrome.runtime.sendMessage({ type: 'ATLAS_OPEN_PREVIEW' })
    }
  }

  function cropVisible(dataUrl, rect) {
    return new Promise((resolve, reject) => {
      const image = new Image()
      image.onload = () => {
        const dpr = window.devicePixelRatio || 1
        const sx = Math.round(rect.left * dpr)
        const sy = Math.round(rect.top * dpr)
        const sw = Math.max(1, Math.round(rect.width * dpr))
        const sh = Math.max(1, Math.round(rect.height * dpr))
        const canvas = document.createElement('canvas')
        canvas.width = sw
        canvas.height = sh
        const ctx = canvas.getContext('2d')
        ctx.drawImage(image, sx, sy, sw, sh, 0, 0, sw, sh)
        resolve(canvas.toDataURL('image/png'))
      }
      image.onerror = reject
      image.src = dataUrl
    })
  }

  function sendMessage(message) {
    return new Promise((resolve) => chrome.runtime.sendMessage(message, resolve))
  }
})()
