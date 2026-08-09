import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import DepthVideoPanel from '@/components/DepthVideoPanel.vue'
import { convertVideoToDepthVideo, suggestDepthVideoOutputPath } from '@/lib/ipc'
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
  convertVideoToDepthVideo: vi.fn(),
  suggestDepthVideoOutputPath: vi.fn(),
}))

describe('DepthVideoPanel', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('converts a selected video into a locally generated depth video and saves it', async () => {
    mocks.open.mockResolvedValue('C:\\Users\\admin\\Desktop\\movie.mp4')
    vi.mocked(suggestDepthVideoOutputPath).mockResolvedValue(
      'C:\\Users\\admin\\Desktop\\movie_depth_08081230.mp4',
    )
    mocks.save.mockResolvedValue('C:\\Users\\admin\\Desktop\\movie_depth_08081230.mp4')
    vi.mocked(convertVideoToDepthVideo).mockResolvedValue({
      outputPath: 'C:\\Users\\admin\\Desktop\\movie_depth_08081230.mp4',
    })
    const wrapper = mount(DepthVideoPanel)

    await wrapper.find('.pick-depth-video-button').trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 0))
    await wrapper.find('.convert-depth-video-button').trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(wrapper.text()).toContain('movie.mp4')
    expect(suggestDepthVideoOutputPath).toHaveBeenCalledWith({
      sourcePath: 'C:\\Users\\admin\\Desktop\\movie.mp4',
    })
    expect(mocks.save).toHaveBeenCalledWith({
      defaultPath: 'C:\\Users\\admin\\Desktop\\movie_depth_08081230.mp4',
      filters: [{ name: '视频文件', extensions: ['mp4'] }],
    })
    expect(convertVideoToDepthVideo).toHaveBeenCalledWith({
      sourcePath: 'C:\\Users\\admin\\Desktop\\movie.mp4',
      outputPath: 'C:\\Users\\admin\\Desktop\\movie_depth_08081230.mp4',
    })
    expect(wrapper.text()).toContain('movie_depth_08081230.mp4')
  })

  it('uses a depth-video source path prefilled from the floating action dialog', () => {
    const ui = useUiStore()
    ui.openDepthVideoWithSource('C:\\Users\\admin\\Desktop\\movie.mp4')

    const wrapper = mount(DepthVideoPanel)

    expect(wrapper.text()).toContain('movie.mp4')
  })

  it('shows a local engine error without losing the selected video', async () => {
    mocks.open.mockResolvedValue('C:\\Users\\admin\\Desktop\\movie.mp4')
    vi.mocked(suggestDepthVideoOutputPath).mockResolvedValue(
      'C:\\Users\\admin\\Desktop\\movie_depth_08081230.mp4',
    )
    mocks.save.mockResolvedValue('C:\\Users\\admin\\Desktop\\movie_depth_08081230.mp4')
    vi.mocked(convertVideoToDepthVideo).mockRejectedValue(
      new Error('DEPTH_VIDEO_ENGINE_MISSING'),
    )
    const wrapper = mount(DepthVideoPanel)

    await wrapper.find('.pick-depth-video-button').trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 0))
    await wrapper.find('.convert-depth-video-button').trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(wrapper.text()).toContain('movie.mp4')
    expect(wrapper.text()).toContain('本地深度视频引擎不可用')
  })
})
