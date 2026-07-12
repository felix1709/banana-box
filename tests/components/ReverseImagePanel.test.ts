import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import ReverseImagePanel from '@/components/ReverseImagePanel.vue'
import { useUiStore } from '@/stores/ui'
import { importImageFromPath, readImageBytes, saveImage } from '@/lib/ipc'
import { listAiProviders, reverseImagePrompt } from '@/lib/provider-ipc'

vi.mock('@/lib/ipc', () => ({
  importImageFromPath: vi.fn(),
  saveImage: vi.fn(),
  saveLibrary: vi.fn().mockResolvedValue(undefined),
  readImageBytes: vi.fn().mockResolvedValue('blob:preview'),
}))

vi.mock('@/lib/provider-ipc', () => ({
  listAiProviders: vi.fn(),
  reverseImagePrompt: vi.fn(),
}))

const reverseImageProvider = {
  id: 'reverse-image' as const,
  kind: 'reverse-image' as const,
  displayName: 'Reverse Image',
  baseUrl: 'https://ai.leihuo.netease.com',
  modelsUrl: 'https://ai.leihuo.netease.com/v1/models',
  chatCompletionsUrl: 'https://ai.leihuo.netease.com/v1/chat/completions',
  defaultModel: 'doubao-seed-1-6-vision-250815',
  availableModels: ['doubao-seed-1-6-vision-250815'],
  probedModel: null,
  structuredMode: null,
  interactiveCompatible: null,
  boundHost: 'https://ai.leihuo.netease.com',
  needsCredentials: false,
  configRevision: 1,
  capabilityRevision: 1,
}

describe('ReverseImagePanel', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    vi.mocked(listAiProviders).mockResolvedValue([reverseImageProvider])
  })

  it('imports an image, reverses it, and opens the prompt editor with generated content', async () => {
    vi.mocked(saveImage).mockResolvedValue('images/reverse.png')
    vi.mocked(reverseImagePrompt).mockResolvedValue({
      prompt: 'a clean product photo prompt',
    })
    const wrapper = mount(ReverseImagePanel)
    const file = new File(['fake'], 'reverse.png', { type: 'image/png' })

    Object.defineProperty(wrapper.find('input[type="file"]').element, 'files', {
      value: [file],
      configurable: true,
    })
    await wrapper.find('input[type="file"]').trigger('change')
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    await wrapper.find('.reverse-button').trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(reverseImagePrompt).toHaveBeenCalledWith({
      providerId: 'reverse-image',
      model: 'doubao-seed-1-6-vision-250815',
      imagePath: 'images/reverse.png',
    })
    expect((wrapper.find('.reverse-result').element as HTMLTextAreaElement).value).toBe(
      'a clean product photo prompt',
    )

    await wrapper.find('.save-result-button').trigger('click')

    const ui = useUiStore()
    expect(ui.editorOpen).toBe(true)
    expect(ui.editorPrefill?.content).toBe('a clean product photo prompt')
    expect(ui.editorPrefill?.image).toBe('images/reverse.png')
  })

  it('prefills an image from an external path selected through the floating action dialog', async () => {
    vi.mocked(importImageFromPath).mockResolvedValue('images/floating.png')
    const ui = useUiStore()
    ui.openReverseImageWithSource('C:\\Users\\admin\\Desktop\\floating.png')

    const wrapper = mount(ReverseImagePanel)
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(importImageFromPath).toHaveBeenCalledWith({
      sourcePath: 'C:\\Users\\admin\\Desktop\\floating.png',
    })
    expect(wrapper.text()).toContain('floating.png')
  })

  it('imports a floating image when the reverse panel is already open', async () => {
    vi.mocked(importImageFromPath).mockResolvedValue('images/already-open.png')
    vi.mocked(readImageBytes).mockResolvedValue('blob:already-open')
    const ui = useUiStore()
    const wrapper = mount(ReverseImagePanel)
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    ui.openReverseImageWithSource('C:\\Users\\admin\\Desktop\\already-open.png')
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(importImageFromPath).toHaveBeenCalledWith({
      sourcePath: 'C:\\Users\\admin\\Desktop\\already-open.png',
    })
    expect(wrapper.text()).toContain('already-open.png')
    expect(wrapper.find('img.image-preview').attributes('src')).toBe('blob:already-open')
  })

  it('shows an imported image preview in the upload zone', async () => {
    vi.mocked(saveImage).mockResolvedValue('images/preview.png')
    vi.mocked(readImageBytes).mockResolvedValue('blob:preview-url')
    const wrapper = mount(ReverseImagePanel)
    const file = new File(['fake'], 'preview.png', { type: 'image/png' })

    Object.defineProperty(wrapper.find('input[type="file"]').element, 'files', {
      value: [file],
      configurable: true,
    })
    await wrapper.find('input[type="file"]').trigger('change')
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    const preview = wrapper.find('img.image-preview')
    expect(readImageBytes).toHaveBeenCalledWith('images/preview.png')
    expect(preview.exists()).toBe(true)
    expect(preview.attributes('src')).toBe('blob:preview-url')
  })

  it('clears the prompt result and image preview for the next reverse run', async () => {
    vi.mocked(saveImage).mockResolvedValue('images/clear.png')
    vi.mocked(readImageBytes).mockResolvedValue('blob:clear-url')
    vi.mocked(reverseImagePrompt).mockResolvedValue({ prompt: 'generated prompt' })
    const wrapper = mount(ReverseImagePanel)
    const file = new File(['fake'], 'clear.png', { type: 'image/png' })

    Object.defineProperty(wrapper.find('input[type="file"]').element, 'files', {
      value: [file],
      configurable: true,
    })
    await wrapper.find('input[type="file"]').trigger('change')
    await new Promise((resolve) => window.setTimeout(resolve, 0))
    await wrapper.find('.reverse-button').trigger('click')
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(wrapper.find('img.image-preview').exists()).toBe(true)
    expect((wrapper.find('.reverse-result').element as HTMLTextAreaElement).value).toBe(
      'generated prompt',
    )

    await wrapper.find('[data-action="clear-reverse"]').trigger('click')

    expect(wrapper.find('img.image-preview').exists()).toBe(false)
    expect(wrapper.text()).not.toContain('clear.png')
    expect((wrapper.find('.reverse-result').element as HTMLTextAreaElement).value).toBe('')
  })
})
