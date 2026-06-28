<script setup lang="ts">
import { ref } from 'vue'
import { v4 as uuid } from 'uuid'
import { open } from '@tauri-apps/plugin-dialog'
import { useLibraryStore } from '@/stores/library'
import { useUiStore } from '@/stores/ui'
import {
  exportLibrary,
  importLibrary,
  readImportDir,
  downloadImage,
} from '@/lib/ipc'
import { parseFile } from '@/lib/parse'
import type { Prompt } from '@/types'

const lib = useLibraryStore()
const ui = useUiStore()
const hotkey = ref(lib.library.settings.hotkey)
const importing = ref(false)

function saveHotkey() {
  lib.library.settings.hotkey = hotkey.value
  lib.persist()
  ui.showToast('已保存')
}

async function onExport() {
  await exportLibrary()
  ui.showToast('已导出')
}

async function onImport() {
  const imported = await importLibrary()
  if (!imported) return
  lib.hydrate(imported)
  ui.showToast('已导入')
}

// 批量导入：选目录 → 解析 .md/.txt → 下载飞书图 → 合并到当前库
async function onBatchImport() {
  const dir = await open({ directory: true, multiple: false })
  if (!dir || Array.isArray(dir)) return
  importing.value = true
  ui.showToast('解析中…')
  try {
    const files = await readImportDir(dir as string)
    const parsed = files.map((f) => parseFile(f.filename, f.content))
    const cats = [...lib.library.categories]
    const prompts: Prompt[] = []
    const pending: { prompt: Prompt; url: string }[] = []
    const now = Math.floor(Date.now() / 1000)
    const colors = [
      '#ef4444',
      '#f59e0b',
      '#facc15',
      '#22c55e',
      '#3b82f6',
      '#8b5cf6',
      '#ec4899',
      '#6b7280',
    ]
    parsed.forEach((pf, i) => {
      const catId = uuid()
      cats.push({
        id: catId,
        name: pf.category,
        color: colors[i % colors.length],
        order: cats.length,
      })
      pf.prompts.forEach((pp) => {
        const p: Prompt = {
          id: uuid(),
          title: pp.title,
          content: pp.content,
          categoryId: catId,
          tags: pp.tags,
          image: null,
          createdAt: now,
          updatedAt: now,
        }
        prompts.push(p)
        if (pp.imageUrl) pending.push({ prompt: p, url: pp.imageUrl })
      })
    })

    let done = 0
    for (const item of pending) {
      try {
        item.prompt.image = await downloadImage(item.url)
      } catch {
        // 忽略单张失败
      }
      done++
      ui.showToast(`下载图片 ${done}/${pending.length}`)
    }

    lib.library.categories = cats
    lib.library.prompts.push(...prompts)
    lib.persist()
    ui.showToast(`导入 ${prompts.length} 条 / 图 ${pending.length}`)
  } catch (e) {
    ui.showToast('导入失败')
  } finally {
    importing.value = false
  }
}
</script>

<template>
  <div
    class="mask"
    @click.self="ui.closeSettings()"
  >
    <div class="dialog">
      <h3>设置</h3>
      <label>
        全局快捷键
        <input v-model="hotkey">
      </label>
      <button @click="saveHotkey">
        保存快捷键
      </button>
      <hr>
      <button
        :disabled="importing"
        @click="onBatchImport"
      >
        {{ importing ? '导入中…' : '📂 批量导入提示词' }}
      </button>
      <button @click="onExport">
        导出 (.zip)
      </button>
      <button @click="onImport">
        导入 (.zip)
      </button>
      <div class="actions">
        <button @click="ui.closeSettings()">
          关闭
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.3);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 20;
}
.dialog {
  background: #fff;
  padding: 16px;
  border-radius: 8px;
  width: 360px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
input {
  padding: 6px;
  border: 1px solid #ddd;
  border-radius: 4px;
}
.actions {
  display: flex;
  justify-content: flex-end;
}
</style>
