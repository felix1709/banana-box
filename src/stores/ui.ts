// src/stores/ui.ts
// UI 状态：面板显隐、编辑器、设置、大图预览、toast。

import { defineStore } from 'pinia'

export type ActiveTool = 'prompts' | 'reverse-image' | 'compression'
export interface PromptEditorPrefill {
  title?: string
  content?: string
  categoryId?: string | null
  tags?: string[]
  image?: string | null
}
export interface FloatingActionFile {
  filePath: string
  fileName: string
  fileType: 'image' | 'video'
}

export const useUiStore = defineStore('ui', {
  state: () => ({
    activeTool: 'prompts' as ActiveTool,
    panelVisible: false as boolean,
    editorOpen: false as boolean,
    editingPromptId: null as string | null,
    editorPrefill: null as PromptEditorPrefill | null,
    floatingActionDialogOpen: false as boolean,
    floatingActionFile: null as FloatingActionFile | null,
    compressionSourcePath: '' as string,
    reverseImageSourcePath: '' as string,
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
    setActiveTool(tool: ActiveTool) {
      this.activeTool = tool
    },
    hidePanel() {
      this.panelVisible = false
    },
    openEditor(id: string | null, prefill: PromptEditorPrefill | null = null) {
      this.editingPromptId = id
      this.editorPrefill = prefill
      this.editorOpen = true
    },
    closeEditor() {
      this.editorOpen = false
      this.editingPromptId = null
      this.editorPrefill = null
    },
    openFloatingActionDialog(file: FloatingActionFile) {
      this.floatingActionFile = file
      this.floatingActionDialogOpen = true
    },
    closeFloatingActionDialog() {
      this.floatingActionFile = null
      this.floatingActionDialogOpen = false
    },
    openCompressionWithSource(sourcePath: string) {
      this.compressionSourcePath = sourcePath
      this.activeTool = 'compression'
      this.showPanel()
    },
    openReverseImageWithSource(sourcePath: string) {
      this.reverseImageSourcePath = sourcePath
      this.activeTool = 'reverse-image'
      this.showPanel()
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
