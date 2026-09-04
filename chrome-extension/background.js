const INGEST_URL = 'http://127.0.0.1:41773/v1/ingest'

chrome.runtime.onInstalled.addListener(() => {
  createContextMenu()
})

createContextMenu()

function createContextMenu() {
  chrome.contextMenus.removeAll(() => {
    chrome.contextMenus.create({
      id: 'atlas-ingest-image',
      title: '分析入库到参考图库',
      contexts: ['image'],
    })
  })
}

chrome.contextMenus.onClicked.addListener(async (info) => {
  if (info.menuItemId === 'atlas-ingest-image' && info.srcUrl) {
    const result = await ingestImages([info.srcUrl])
    if (result.failed === 0) {
      notify('banana-box', '图片已提交到参考图库')
    } else {
      notify('banana-box', '入库失败，请确认 Banana Box 已启动并在参考图库开启入库服务')
    }
  }
})

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (!message) return

  if (message.type === 'ATLAS_INGEST_IMAGES') {
    const urls = Array.isArray(message.urls) ? message.urls : []
    ingestImages(urls).then(sendResponse).catch((error) => {
      sendResponse({ ok: 0, failed: urls.length, error: String(error) })
    })
    return true
  }

  if (message.type === 'ATLAS_INGEST_ITEMS') {
    const items = Array.isArray(message.items) ? message.items : []
    ingestItems(items).then(sendResponse).catch((error) => {
      sendResponse({ ok: 0, failed: items.length, error: String(error) })
    })
    return true
  }

  if (message.type === 'ATLAS_CAPTURE_VISIBLE') {
    chrome.tabs.captureVisibleTab(_sender.tab.windowId, { format: 'png' }, (dataUrl) => {
      if (chrome.runtime.lastError || !dataUrl) {
        sendResponse({ ok: false, error: chrome.runtime.lastError?.message || 'capture_failed' })
      } else {
        sendResponse({ ok: true, dataUrl })
      }
    })
    return true
  }

  if (message.type === 'ATLAS_OPEN_PREVIEW') {
    chrome.windows.create({
      url: chrome.runtime.getURL('preview.html'),
      type: 'popup',
      width: 980,
      height: 720,
    })
    sendResponse({ ok: true })
    return false
  }

  return false
})

async function ingestImages(urls) {
  return ingestItems(urls.map((url) => ({ sourceUrl: url })))
}

async function ingestItems(items) {
  let ok = 0
  let failed = 0

  for (const item of items) {
    try {
      await ingestPayload(item)
      ok += 1
    } catch {
      failed += 1
    }
  }

  return { ok, failed, total: items.length }
}

async function ingestPayload(payload) {
  let response
  try {
    response = await fetch(INGEST_URL, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
    })
  } catch {
    throw new Error('无法连接本地入库服务，请先启动 Banana Box 并开启参考图库服务')
  }

  if (!response.ok) {
    throw new Error(`HTTP ${response.status}`)
  }
}

function notify(title, message) {
  chrome.notifications.create({
    type: 'basic',
    iconUrl: 'icons/icon128.png',
    title,
    message,
  })
}
