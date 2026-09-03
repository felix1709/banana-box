chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({
    id: 'atlas-ingest-image',
    title: '分析入库到 AAA-Aesthetic-Atlas',
    contexts: ['image'],
  })
})

chrome.contextMenus.onClicked.addListener(async (info) => {
  if (info.menuItemId === 'atlas-ingest-image' && info.srcUrl) {
    await fetch('http://127.0.0.1:41773/v1/ingest', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ sourceUrl: info.srcUrl }),
    })
  }
})
