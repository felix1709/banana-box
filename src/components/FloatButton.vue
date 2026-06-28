<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'
import { emit } from '@tauri-apps/api/event'

const win = getCurrentWindow()
let startX = 0
let startY = 0
let dragging = false

// 手动区分单击/拖动：移动 >3px 算拖动 → startDragging；否则算单击 → toggle 主面板
function onMousedown(e: MouseEvent) {
  if (e.button !== 0) return
  startX = e.screenX
  startY = e.screenY
  dragging = false
}

async function onMousemove(e: MouseEvent) {
  if (e.buttons !== 1 || dragging) return
  if (Math.abs(e.screenX - startX) > 3 || Math.abs(e.screenY - startY) > 3) {
    dragging = true
    try {
      await win.startDragging()
    } catch {
      // ignore
    }
  }
}

function onMouseup() {
  if (!dragging) {
    void emit('toggle-panel', null)
  }
}
</script>

<template>
  <div
    class="float-btn"
    @mousedown="onMousedown"
    @mousemove="onMousemove"
    @mouseup="onMouseup"
  >
    🍌
  </div>
</template>

<style scoped>
.float-btn {
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 32px;
  cursor: pointer;
  user-select: none;
  background: transparent;
}
</style>
