<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'

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

function onClick() {
  dragging = false
}
</script>

<template>
  <div
    class="float-btn"
    @mousedown="onMousedown"
    @mousemove="onMousemove"
    @click="onClick"
  >
    🍌
  </div>
</template>

<style scoped>
:global(html),
:global(body),
:global(#app) {
  width: 100%;
  height: 100%;
  overflow: hidden;
}

.float-btn {
  width: 100vw;
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 32px;
  cursor: pointer;
  user-select: none;
  background: transparent;
  font-family: "Segoe UI Emoji", "Apple Color Emoji", "Noto Color Emoji", system-ui, sans-serif;
  line-height: 1;
}

.float-btn:hover {
  filter: drop-shadow(0 2px 5px rgba(15, 23, 42, 0.28));
}

.float-btn:active {
  transform: scale(0.96);
}
</style>
