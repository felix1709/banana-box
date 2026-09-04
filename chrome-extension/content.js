(() => {
  const minSize = 120
  const maxImages = 50
  const seen = new Set()
  const urls = []

  for (const image of document.images) {
    try {
      const raw = image.currentSrc || image.src || ''
      if (!raw || raw.startsWith('data:')) continue

      const url = new URL(raw, window.location.href).href
      if (!url.startsWith('http://') && !url.startsWith('https://')) continue
      if (image.naturalWidth > 0 && image.naturalWidth < minSize) continue
      if (seen.has(url)) continue

      seen.add(url)
      urls.push(url)
      if (urls.length >= maxImages) break
    } catch {
      // 单个图片地址无效时跳过，继续处理下一张
    }
  }

  if (urls.length === 0) {
    chrome.runtime.sendMessage({ type: 'ATLAS_INGEST_IMAGES', urls: [] }).catch(() => {})
    return
  }

  chrome.runtime.sendMessage({ type: 'ATLAS_INGEST_IMAGES', urls }).catch(() => {})
})()
