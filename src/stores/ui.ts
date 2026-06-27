// src/stores/ui.ts
// UI 状态：面板显隐、编辑器、设置、大图预览、toast。

import { defineStore } from 'pinia'

export const useUiStore = defineStore('ui', {
  state: () => ({
    panelVisible: false as boolean,
    editorOpen: false as boolean,
    editingPromptId: null as string | null,
    settingsOpen: false as boolean,
    previewImage: null as string | null,
    toast: '' as string,
  }),
  actions: {
    togglePanel() {
      this.panelVisible = !this.panelVisible
    },
    showPanel() {
      this.panelVisible = true
    },
    hidePanel() {
      this.panelVisible = false
    },
    openEditor(id: string | null) {
      this.editingPromptId = id
      this.editorOpen = true
    },
    closeEditor() {
      this.editorOpen = false
      this.editingPromptId = null
    },
    openSettings() {
      this.settingsOpen = true
    },
    closeSettings() {
      this.settingsOpen = false
    },
    preview(img: string | null) {
      this.previewImage = img
    },
    showToast(msg: string) {
      this.toast = msg
      setTimeout(() => {
        this.toast = ''
      }, 1500)
    },
  },
})
