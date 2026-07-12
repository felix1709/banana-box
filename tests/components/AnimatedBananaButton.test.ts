import { mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import AnimatedBananaButton from '@/components/AnimatedBananaButton.vue'

describe('AnimatedBananaButton', () => {
  beforeEach(() => {
    vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
      callback(performance.now() + 360)
      return 1
    })
    vi.stubGlobal('cancelAnimationFrame', vi.fn())
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('renders the closed frame first and the open frame after retargeting', async () => {
    const wrapper = mount(AnimatedBananaButton, { props: { open: false } })

    expect(wrapper.attributes('data-frame')).toBe('0')

    await wrapper.setProps({ open: true })

    expect(wrapper.attributes('data-frame')).toBe('11')
    expect(wrapper.emitted('frame')?.at(-1)).toEqual([11])
  })

  it('exposes a stable 64px hit surface without changing the sprite bounds', () => {
    const wrapper = mount(AnimatedBananaButton, { props: { open: false } })

    expect(wrapper.classes()).toContain('animated-banana')
    expect(wrapper.find('.banana-sprite').exists()).toBe(true)
    expect(wrapper.attributes('data-frame')).toBe('0')
  })
})
