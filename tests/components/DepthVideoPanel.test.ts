import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import DepthVideoPanel from '@/components/DepthVideoPanel.vue'
import {
  convertVideoToDepthVideo,
  prepareDepthVideoEngine,
  prepareDepthVideoPython,
  suggestDepthVideoOutputPath,
} from '@/lib/ipc'
import { revealOutputPath } from '@/lib/outputReveal'
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
  prepareDepthVideoEngine: vi.fn(),
  prepareDepthVideoPython: vi.fn(),
  suggestDepthVideoOutputPath: vi.fn(),
}))

vi.mock('@/lib/outputReveal', () => ({
  revealOutputPath: vi.fn(),
}))

describe('DepthVideoPanel', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.resetAllMocks()
    window.localStorage.clear()
  })

  it('shows only one setup action for the depth-video environment', () => {
    const wrapper = mount(DepthVideoPanel)

    expect(wrapper.find('.prepare-depth-environment-button').exists()).toBe(true)
    expect(wrapper.find('.install-python-button').exists()).toBe(false)
    expect(wrapper.find('.pick-depth-engine-button').exists()).toBe(false)
    expect(wrapper.find('.prepare-depth-engine-button').exists()).toBe(false)
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
    vi.mocked(revealOutputPath).mockResolvedValue(true)
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
    expect(revealOutputPath).toHaveBeenCalledWith(
      'C:\\Users\\admin\\Desktop\\movie_depth_08081230.mp4',
    )
    expect(wrapper.text()).toContain('movie_depth_08081230.mp4')
  })

  it('lets users reopen the output folder after depth-video conversion succeeds', async () => {
    mocks.open.mockResolvedValue('C:\\Users\\admin\\Desktop\\movie.mp4')
    vi.mocked(suggestDepthVideoOutputPath).mockResolvedValue(
      'C:\\Users\\admin\\Desktop\\movie_depth_08081230.mp4',
    )
    mocks.save.mockResolvedValue('C:\\Users\\admin\\Desktop\\movie_depth_08081230.mp4')
    vi.mocked(convertVideoToDepthVideo).mockResolvedValue({
      outputPath: 'C:\\Users\\admin\\Desktop\\movie_depth_08081230.mp4',
    })
    vi.mocked(revealOutputPath).mockResolvedValue(true)
    const wrapper = mount(DepthVideoPanel)

    await wrapper.find('.pick-depth-video-button').trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 0))
    await wrapper.find('.convert-depth-video-button').trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 0))
    vi.mocked(revealOutputPath).mockClear()

    await wrapper.find('.open-output-folder-button').trigger('click')

    expect(revealOutputPath).toHaveBeenCalledWith(
      'C:\\Users\\admin\\Desktop\\movie_depth_08081230.mp4',
    )
  })

  it('prepares the full depth-video environment from one primary action', async () => {
    vi.mocked(prepareDepthVideoPython).mockResolvedValue({
      pythonVersion: '3.10',
      message: 'Python 3.10 环境已准备好',
    })
    vi.mocked(prepareDepthVideoEngine).mockResolvedValue({
      enginePath: 'C:\\Users\\admin\\AppData\\Roaming\\banana-box\\depth-video-engine\\banana-depth-video.cmd',
      engineDir: 'C:\\Users\\admin\\AppData\\Roaming\\banana-box\\depth-video-engine',
      message: '本地深度视频引擎已配置',
    })
    const wrapper = mount(DepthVideoPanel, {
      global: {
        stubs: {
          Teleport: true,
        },
      },
    })

    await wrapper.find('.prepare-depth-environment-button').trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(prepareDepthVideoPython).toHaveBeenCalled()
    expect(prepareDepthVideoEngine).toHaveBeenCalled()
    expect(window.localStorage.getItem('banana-box-depth-video-engine')).toBe(
      'C:\\Users\\admin\\AppData\\Roaming\\banana-box\\depth-video-engine\\banana-depth-video.cmd',
    )
    expect(wrapper.text()).toContain('本地深度视频引擎已配置')
  })

  it('shows a friendly Python install hint when automatic engine setup cannot find Python', async () => {
    vi.mocked(prepareDepthVideoPython).mockRejectedValue(new Error('PYTHON_NOT_FOUND'))
    const wrapper = mount(DepthVideoPanel)

    await wrapper.find('.prepare-depth-environment-button').trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(wrapper.text()).toContain('未找到 Python。请先安装 Python 3.10+')
  })

  it('shows a friendly Python version hint when automatic engine setup finds only unsupported Python versions', async () => {
    vi.mocked(prepareDepthVideoPython).mockResolvedValue({
      pythonVersion: '3.10',
      message: 'Python 3.10 环境已准备好',
    })
    vi.mocked(prepareDepthVideoEngine).mockRejectedValue(
      new Error('DEPTH_VIDEO_ENGINE_SETUP_FAILED\nPYTHON_VERSION_UNSUPPORTED: 3.12'),
    )
    const wrapper = mount(DepthVideoPanel)

    await wrapper.find('.prepare-depth-environment-button').trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(wrapper.text()).toContain('请点击“一键配置深度视频环境”')
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

  it('shows a friendly hint when the selected depth engine is not configured yet', async () => {
    mocks.open.mockResolvedValue('C:\\Users\\admin\\Desktop\\movie.mp4')
    vi.mocked(suggestDepthVideoOutputPath).mockResolvedValue(
      'C:\\Users\\admin\\Desktop\\movie_depth_08081230.mp4',
    )
    mocks.save.mockResolvedValue('C:\\Users\\admin\\Desktop\\movie_depth_08081230.mp4')
    vi.mocked(convertVideoToDepthVideo).mockRejectedValue(
      new Error('DEPTH_VIDEO_ENGINE_NOT_CONFIGURED'),
    )
    const wrapper = mount(DepthVideoPanel)

    await wrapper.find('.pick-depth-video-button').trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 0))
    await wrapper.find('.convert-depth-video-button').trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(wrapper.text()).toContain('请先在 Banana Box 中点击“下载并配置”')
  })
})
