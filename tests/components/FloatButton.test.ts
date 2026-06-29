import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import FloatButton from '@/components/FloatButton.vue'

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    startDragging: vi.fn().mockResolvedValue(undefined),
  }),
}))

describe('FloatButton', () => {
  it('shows a banana button', () => {
    const wrapper = mount(FloatButton)

    expect(wrapper.text()).toBe('🍌')
  })

  it('accepts click without changing the visual label', async () => {
    const wrapper = mount(FloatButton)

    await wrapper.trigger('click')

    expect(wrapper.text()).toBe('🍌')
  })
})
