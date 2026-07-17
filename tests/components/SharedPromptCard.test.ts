import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import SharedPromptCard from '@/components/shared/SharedPromptCard.vue'
import { useAuthStore } from '@/stores/auth'
import { useSharedLibraryStore } from '@/stores/sharedLibrary'
import { useUiStore } from '@/stores/ui'
import { readImageBytes } from '@/lib/ipc'
import type { SharedPrompt } from '@/types'

vi.mock('@/lib/ipc', () => ({
  copyToClipboard: vi.fn().mockResolvedValue(undefined),
  readImageBytes: vi.fn(),
}))

describe('SharedPromptCard', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('uses the normal prompt card collapsed and expanded layout', async () => {
    const wrapper = mount(SharedPromptCard, { props: { prompt: sharedPrompt() } })

    expect(wrapper.classes()).toContain('fixed-size-prompt-card')
    expect(wrapper.find('.actions').exists()).toBe(false)

    await wrapper.trigger('click')

    expect(wrapper.classes()).toContain('expanded')
    expect(wrapper.classes()).toContain('expanded-auto-height-card')
    expect(wrapper.classes()).toContain('flow-expanded-card')
    expect(wrapper.classes()).not.toContain('fixed-size-prompt-card')
    expect(wrapper.find('.card-main').classes()).toContain('flow-card-main')
    expect(wrapper.find('.text-pane').classes()).toContain('flow-text-pane')
    expect(wrapper.find('.content').classes()).toContain('full-content')
    expect(wrapper.find('.content').classes()).toContain('flow-content')
    expect(wrapper.find('.tags').classes()).toContain('full-tags')
    expect(wrapper.find('.actions').exists()).toBe(true)
  })

  it('toggles a local favorite star without expanding the card', async () => {
    const wrapper = mount(SharedPromptCard, { props: { prompt: sharedPrompt() } })

    await wrapper.get('.favorite-button').trigger('click')

    expect(wrapper.get('.favorite-button').classes()).toContain('active')
    expect(wrapper.classes()).toContain('collapsed')
  })

  it('shows and previews the shared prompt image', async () => {
    vi.mocked(readImageBytes).mockResolvedValue('blob:shared-preview')
    const wrapper = mount(SharedPromptCard, {
      props: { prompt: sharedPrompt({ image: 'images/shared.png' }) },
    })
    await new Promise((resolve) => window.setTimeout(resolve, 0))

    expect(wrapper.find('img.thumb').attributes('src')).toBe('blob:shared-preview')

    await wrapper.get('.thumb-zone').trigger('click')

    expect(useUiStore().previewImage).toBe('images/shared.png')
  })

  it('lets normal users copy and download but not delete', async () => {
    useAuthStore().user = { id: 'user-2', email: '000002@banana-box.local' } as never
    const wrapper = mount(SharedPromptCard, { props: { prompt: sharedPrompt() } })

    await wrapper.trigger('click')

    expect(wrapper.find('[data-action="copy-shared-prompt"]').exists()).toBe(true)
    expect(wrapper.find('[data-action="download-shared-prompt"]').exists()).toBe(true)
    expect(wrapper.find('[data-action="delete-shared-prompt"]').exists()).toBe(false)
  })

  it('lets account 000001 delete shared prompts', async () => {
    useAuthStore().user = { id: 'admin', email: '000001@banana-box.local' } as never
    const shared = useSharedLibraryStore()
    const remove = vi.spyOn(shared, 'deleteSharedPrompt').mockResolvedValue(undefined)
    const wrapper = mount(SharedPromptCard, { props: { prompt: sharedPrompt() } })

    await wrapper.trigger('click')
    await wrapper.get('[data-action="delete-shared-prompt"]').trigger('click')

    expect(remove).toHaveBeenCalledWith('shared-1')
  })
})

function sharedPrompt(overrides: Partial<SharedPrompt> = {}): SharedPrompt {
  return {
    id: 'shared-1',
    title: 'Shared Prompt',
    content: 'Use this prompt',
    tags: ['shared'],
    image: null,
    createdBy: 'user-1',
    createdByName: 'Felix',
    createdAt: '2026-07-17T00:00:00Z',
    updatedAt: '2026-07-17T00:00:00Z',
    ...overrides,
  }
}
