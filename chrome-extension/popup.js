const captureButton = document.getElementById('capture')
const areaCaptureButton = document.getElementById('areaCapture')
const status = document.getElementById('status')

captureButton.addEventListener('click', async () => {
  captureButton.disabled = true
  status.textContent = '正在抓取当前网页图片…'

  try {
    const tab = await activeWebTab()
    const health = await fetch('http://127.0.0.1:41773/v1/health')
    if (!health.ok) {
      throw new Error('请先在 Banana Box 参考图库中开启入库服务')
    }

    await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      files: ['content.js'],
    })

    status.textContent = '已提交抓取，图片正在入库。可在参考图库中刷新查看。'
  } catch (error) {
    status.textContent = `失败：${error?.message || error}`
  } finally {
    captureButton.disabled = false
  }
})

areaCaptureButton.addEventListener('click', async () => {
  areaCaptureButton.disabled = true
  try {
    const tab = await activeWebTab()
    await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      files: ['area-select.js'],
    })
    window.close()
  } catch (error) {
    status.textContent = `失败：${error?.message || error}`
    areaCaptureButton.disabled = false
  }
})

async function activeWebTab() {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true })
  if (!tab || tab.id === undefined) {
    throw new Error('当前页面不可用')
  }
  if (!isInjectablePage(tab.url)) {
    throw new Error('请先打开普通网页（http/https）再操作，Chrome 内置页面无法使用')
  }
  return tab
}

function isInjectablePage(url) {
  if (!url) return false
  try {
    const parsed = new URL(url)
    return parsed.protocol === 'http:' || parsed.protocol === 'https:'
  } catch {
    return false
  }
}
