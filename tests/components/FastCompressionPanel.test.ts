import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import FastCompressionPanel from '@/components/FastCompressionPanel.vue'
import { compressMedia, suggestCompressedOutputPath } from '@/lib/ipc'
import { createPinia, setActivePinia } from 'pinia'
import { useUiStore } from '@/stores/ui'

const mocks = vi.hoisted(() => ({
  open: vi.fn(),
  save: vi.fn(),
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: mocks.open,
  save: mocks.save,
}))

vi.mock('@/lib/ipc', () => ({
  compressMedia: vi.fn(),
  suggestCompressedOutputPath: vi.fn(),
}))

describe('FastCompressionPanel', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('imports a file and compresses it with a target size in MB', async () => {
    mocks.open.mockResolvedValue('C:\\Users\\admin\\Desktop\\photo.png')
    vi.mocked(suggestCompressedOutputPath).mockResolvedValue(
      'C:\\Users\\admin\\Desktop\\photo_06301205.jpg',
    )
    mocks.save.mockResolvedValue('C:\\Users\\admin\\Desktop\\photo_06301205.jpg')
    vi.mocked(compressMedia).mockResolvedValue({
      outputPath: 'C:\\Users\\admin\\Desktop\\photo_06301205.jpg',
    })
    const wrapper = mount(FastCompressionPanel)

    await wrapper.find('.pick-file-button').trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 0))
    await wrapper.find('.target-mb-input').setValue('2')
    await wrapper.find('.compress-button').trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(wrapper.text()).toContain('photo.png')
    expect(suggestCompressedOutputPath).toHaveBeenCalledWith({
      sourcePath: 'C:\\Users\\admin\\Desktop\\photo.png',
    })
    expect(mocks.save).toHaveBeenCalledWith({
      defaultPath: 'C:\\Users\\admin\\Desktop\\photo_06301205.jpg',
    })
    expect(compressMedia).toHaveBeenCalledWith({
      sourcePath: 'C:\\Users\\admin\\Desktop\\photo.png',
      targetMb: 2,
      outputPath: 'C:\\Users\\admin\\Desktop\\photo_06301205.jpg',
    })
    expect(wrapper.text()).toContain('photo_06301205.jpg')
  })

  it('shows a visual progress bar while compression is running', async () => {
    let resolveCompression: ((value: { outputPath: string }) => void) | null = null
    mocks.open.mockResolvedValue('C:\\Users\\admin\\Desktop\\movie.mp4')
    vi.mocked(suggestCompressedOutputPath).mockResolvedValue(
      'C:\\Users\\admin\\Desktop\\movie_07010930.mp4',
    )
    mocks.save.mockResolvedValue('C:\\Users\\admin\\Desktop\\movie_07010930.mp4')
    vi.mocked(compressMedia).mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveCompression = resolve
        }),
    )
    const wrapper = mount(FastCompressionPanel)

    await wrapper.find('.pick-file-button').trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 0))
    await wrapper.find('.compress-button').trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    const progress = wrapper.find('[role="progressbar"]')
    expect(progress.exists()).toBe(true)
    expect(progress.attributes('aria-valuenow')).toBeDefined()
    expect(wrapper.text()).toContain('压缩中')

    resolveCompression?.({ outputPath: 'C:\\Users\\admin\\Desktop\\movie_07010930.mp4' })
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(wrapper.find('[role="progressbar"]').attributes('aria-valuenow')).toBe('100')
  })

  it('uses a compression source path prefilled from the floating action dialog', () => {
    const ui = useUiStore()
    ui.openCompressionWithSource('C:\\Users\\admin\\Desktop\\movie.mp4')

    const wrapper = mount(FastCompressionPanel)

    expect(wrapper.text()).toContain('movie.mp4')
  })
})
