import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import StoryboardPage from '@/components/storyboard/StoryboardPage.vue'

describe('StoryboardPage', () => {
  it('collects structured creative settings and exposes copyable Markdown output', async () => {
    const wrapper = mount(StoryboardPage)

    expect((wrapper.get('[data-field="story-model"]').element as HTMLSelectElement).value).toBe('glm-5.2')
    await wrapper.get('[data-field="story-input"]').setValue('女孩在雨夜追逐末班车')
    await wrapper.get('[data-action="generate-storyboard"]').trigger('click')

    expect(wrapper.get('[data-storyboard-output]').text()).toContain('女孩在雨夜追逐末班车')
    expect(wrapper.get('[data-action="copy-storyboard-markdown"]').exists()).toBe(true)
  })
})
