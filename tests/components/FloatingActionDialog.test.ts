import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it } from 'vitest'
import FloatingActionDialog from '@/components/FloatingActionDialog.vue'
import { useUiStore } from '@/stores/ui'

describe('FloatingActionDialog', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('shows reverse and compression actions for images', async () => {
    const ui = useUiStore()
    ui.openFloatingActionDialog({
      filePath: 'C:/tmp/photo.png',
      fileName: 'photo.png',
      fileType: 'image',
    })
    const wrapper = mount(FloatingActionDialog)

    expect(wrapper.text()).toContain('photo.png')
    expect(wrapper.find('[data-action="reverse-image"]').text()).toContain('反推提示词')
    expect(wrapper.find('[data-action="compress-image"]').text()).toContain('压缩图片')

    await wrapper.find('[data-action="reverse-image"]').trigger('click')

    expect(ui.activeTool).toBe('reverse-image')
    expect(ui.reverseImageSourcePath).toBe('C:/tmp/photo.png')
    expect(ui.floatingActionDialogOpen).toBe(false)

    ui.openFloatingActionDialog({
      filePath: 'C:/tmp/photo.png',
      fileName: 'photo.png',
      fileType: 'image',
    })
    await wrapper.vm.$nextTick()
    await wrapper.find('[data-action="compress-image"]').trigger('click')

    expect(ui.activeTool).toBe('compression')
    expect(ui.compressionSourcePath).toBe('C:/tmp/photo.png')
    expect(ui.floatingActionDialogOpen).toBe(false)
  })

  it('shows compression and a reserved reverse action for videos', async () => {
    const ui = useUiStore()
    ui.openFloatingActionDialog({
      filePath: 'C:/tmp/movie.mp4',
      fileName: 'movie.mp4',
      fileType: 'video',
    })
    const wrapper = mount(FloatingActionDialog)

    expect(wrapper.find('[data-action="compress-video"]').text()).toContain('压缩视频')
    expect(wrapper.find('[data-action="reverse-video"]').text()).toContain('视频反推')
    expect(wrapper.find('[data-action="reverse-video"]').attributes('disabled')).toBeDefined()
  })
})
