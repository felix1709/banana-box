import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it } from 'vitest'
import { useUiStore } from '@/stores/ui'

describe('ui store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('tracks the active top-level tool', () => {
    const ui = useUiStore()

    expect(ui.activeTool).toBe('prompts')

    ui.setActiveTool('reverse-image')
    expect(ui.activeTool).toBe('reverse-image')

    ui.setActiveTool('compression')
    expect(ui.activeTool).toBe('compression')

    ui.setActiveTool('daily-tasks')
    expect(ui.activeTool).toBe('daily-tasks')

    ui.setActiveTool('depth-video')
    expect(ui.activeTool).toBe('depth-video')
  })

  it('opens the prompt editor with optional prefilled content', () => {
    const ui = useUiStore()

    ui.openEditor(null, {
      content: 'generated prompt',
      image: 'images/source.png',
    })

    expect(ui.editorOpen).toBe(true)
    expect(ui.editingPromptId).toBeNull()
    expect(ui.editorPrefill).toEqual({
      content: 'generated prompt',
      image: 'images/source.png',
    })

    ui.closeEditor()
    expect(ui.editorPrefill).toBeNull()
  })

  it('opens and closes the floating file action dialog', () => {
    const ui = useUiStore()

    ui.openFloatingActionDialog({
      filePath: 'C:/tmp/photo.png',
      fileName: 'photo.png',
      fileType: 'image',
    })

    expect(ui.floatingActionDialogOpen).toBe(true)
    expect(ui.floatingActionFile).toEqual({
      filePath: 'C:/tmp/photo.png',
      fileName: 'photo.png',
      fileType: 'image',
    })

    ui.closeFloatingActionDialog()
    expect(ui.floatingActionDialogOpen).toBe(false)
    expect(ui.floatingActionFile).toBeNull()
  })

  it('stores a prefilled compression source path', () => {
    const ui = useUiStore()

    ui.openCompressionWithSource('C:/tmp/photo.png')

    expect(ui.activeTool).toBe('compression')
    expect(ui.compressionSourcePath).toBe('C:/tmp/photo.png')
  })

  it('stores a prefilled reverse image source path', () => {
    const ui = useUiStore()

    ui.openReverseImageWithSource('C:/tmp/photo.png')

    expect(ui.activeTool).toBe('reverse-image')
    expect(ui.reverseImageSourcePath).toBe('C:/tmp/photo.png')
  })

  it('stores a prefilled depth-video source path', () => {
    const ui = useUiStore()

    ui.openDepthVideoWithSource('C:/tmp/movie.mp4')

    expect(ui.activeTool).toBe('depth-video')
    expect(ui.depthVideoSourcePath).toBe('C:/tmp/movie.mp4')
  })
})
