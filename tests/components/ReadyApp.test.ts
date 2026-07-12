import { mount } from '@vue/test-utils'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { getActivePinia, setActivePinia } from 'pinia'
import ReadyApp from '@/components/ReadyApp.vue'

vi.mock('@/App.vue', () => ({
  default: {
    name: 'App',
    template: '<div data-test="main-app">主应用</div>',
  },
}))

describe('ReadyApp', () => {
  afterEach(() => {
    setActivePinia(undefined)
  })

  it('creates Pinia only when the ready application mounts', () => {
    setActivePinia(undefined)

    const wrapper = mount(ReadyApp)

    expect(getActivePinia()).toBeDefined()
    expect(wrapper.find('[data-test="main-app"]').exists()).toBe(true)
  })
})
