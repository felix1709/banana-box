import { mount, type VueWrapper } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick } from 'vue'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import UserCloudMenu from '@/components/cloud/UserCloudMenu.vue'
import { useAuthStore } from '@/stores/auth'
import { useSyncStatusStore } from '@/stores/syncStatus'
import { useWorkspacesStore } from '@/stores/workspaces'

describe('UserCloudMenu', () => {
  let wrappers: VueWrapper[] = []

  beforeEach(() => {
    document.body.innerHTML = ''
    wrappers = []
    setActivePinia(createPinia())
  })

  afterEach(() => {
    for (const wrapper of wrappers) {
      wrapper.unmount()
    }
  })

  function mountMenu() {
    const wrapper = mount(UserCloudMenu)
    wrappers.push(wrapper)
    return wrapper
  }

  it('opens a user popover from an icon-only button', async () => {
    const auth = useAuthStore()
    auth.cloudAvailable = true
    const wrapper = mountMenu()

    const button = wrapper.get('[data-action="user-cloud-menu"]')
    expect(button.text().trim()).toBe('')
    expect(button.find('svg').exists()).toBe(true)

    await button.trigger('click')

    expect(document.body.textContent).toContain('账号登录')
    expect(document.body.textContent).not.toContain('注册账号')
    const popover = document.body.querySelector('.user-cloud-popover')
    expect(popover).toBeTruthy()
    expect(getComputedStyle(popover as Element).position).toBe('fixed')
    expect(Number(getComputedStyle(popover as Element).zIndex)).toBeGreaterThan(100)
  })

  it('contains workspace and sync state for a logged-in user', async () => {
    const auth = useAuthStore()
    auth.cloudAvailable = true
    auth.user = { id: 'user-1', email: 'a@example.com' } as never
    const workspaces = useWorkspacesStore()
    workspaces.workspaces = [{
      id: 'workspace-1',
      name: '个人空间',
      ownerId: 'user-1',
      createdAt: 'now',
      updatedAt: 'now',
    }]
    workspaces.activeWorkspaceId = 'workspace-1'
    const sync = useSyncStatusStore()
    sync.state = 'synced'
    sync.lastSyncedAt = '2026-07-13T15:00:00Z'

    const wrapper = mountMenu()
    await wrapper.get('[data-action="user-cloud-menu"]').trigger('click')

    expect(document.body.textContent).toContain('个人空间')
    expect(document.body.textContent).toContain('已同步')
    expect(document.body.querySelector('[data-action="auth-sign-out"]')).toBeTruthy()
  })

  it('shows only account exit and sync state for a non-admin signed-in user', async () => {
    const auth = useAuthStore()
    auth.cloudAvailable = true
    auth.user = { id: 'user-2', email: '000002@banana-box.local' } as never
    const workspaces = useWorkspacesStore()
    workspaces.workspaces = [{
      id: 'workspace-2',
      name: '000002 的个人空间',
      ownerId: 'user-2',
      createdAt: 'now',
      updatedAt: 'now',
    }]
    workspaces.activeWorkspaceId = 'workspace-2'
    const sync = useSyncStatusStore()
    sync.state = 'synced'
    sync.lastSyncedAt = '2026-07-13T15:00:00Z'

    const wrapper = mountMenu()
    await wrapper.get('[data-action="user-cloud-menu"]').trigger('click')

    expect(document.body.textContent).toContain('000002 的个人空间')
    expect(document.body.textContent).toContain('已同步')
    expect(document.body.querySelector('[data-action="auth-sign-out"]')).toBeTruthy()
    expect(document.body.querySelector('.invite-accept-panel')).toBeNull()
    expect(document.body.querySelector('.notifications-menu')).toBeNull()
  })

  it('closes the popover after a successful login', async () => {
    const auth = useAuthStore()
    auth.cloudAvailable = true
    const wrapper = mountMenu()

    await wrapper.get('[data-action="user-cloud-menu"]').trigger('click')
    expect(document.body.querySelector('.user-cloud-popover')).toBeTruthy()

    auth.user = { id: 'user-1', email: '000001@banana-box.local' } as never
    await nextTick()

    expect(document.body.querySelector('.user-cloud-popover')).toBeNull()
  })

  it('closes the popover when clicking outside it', async () => {
    const auth = useAuthStore()
    auth.cloudAvailable = true
    const wrapper = mountMenu()

    await wrapper.get('[data-action="user-cloud-menu"]').trigger('click')
    expect(document.body.querySelector('.user-cloud-popover')).toBeTruthy()

    document.body.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await nextTick()

    expect(document.body.querySelector('.user-cloud-popover')).toBeNull()
  })
})
