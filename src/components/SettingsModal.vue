<script setup lang="ts">
import { ref } from 'vue'
import { useLibraryStore } from '@/stores/library'
import { useUiStore } from '@/stores/ui'
import { exportLibrary, importLibrary } from '@/lib/ipc'

const lib = useLibraryStore()
const ui = useUiStore()
const hotkey = ref(lib.library.settings.hotkey)

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
  // 覆盖式导入：命令已把 library.json + 图片解包到数据目录
  lib.hydrate(imported)
  ui.showToast('已导入')
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
