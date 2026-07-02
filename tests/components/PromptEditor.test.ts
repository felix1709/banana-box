import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import PromptEditor from '@/components/PromptEditor.vue'
import { useLibraryStore } from '@/stores/library'
import { useUiStore } from '@/stores/ui'
import type { Library } from '@/types'

vi.mock('@/lib/ipc', () => ({
  saveImage: vi.fn(),
  saveLibrary: vi.fn().mockResolvedValue(undefined),
}))

function seed(): Library {
  return {
    version: 1,
    categories: [],
    prompts: [],
    settings: {
      hotkey: 'Ctrl+Shift+B',
      theme: 'auto',
      apiBaseUrl: 'https://ai.leihuo.netease.com',
      apiKey: '',
      reverseModel: 'doubao-seed-1-6-vision-250815',
      availableReverseModels: [
        'doubao-seed-1-6-vision-250815',
        'gpt-5.4-mini',
        'qwen3.5-omni-plus',
        'qwen3-vl-plus',
      ],
    },
  }
}

describe('PromptEditor', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    useLibraryStore().hydrate(seed())
  })

  it('prefills generated reverse prompt content for a new prompt', () => {
    const ui = useUiStore()
    ui.openEditor(null, {
      content: 'generated prompt from image',
      image: 'images/source.png',
    })

    const wrapper = mount(PromptEditor)

    expect((wrapper.find('textarea').element as HTMLTextAreaElement).value).toBe(
      'generated prompt from image',
    )
    expect(wrapper.text()).toContain('images/source.png')
  })
})
