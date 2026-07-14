import { describe, expect, it, vi } from 'vitest'
import { subscribeWorkspaceRealtime } from '@/lib/realtime/workspaceRealtime'

describe('workspace realtime helper', () => {
  it('subscribes to collaboration tables and calls refresh on changes', () => {
    const channel = {
      on: vi.fn(() => channel),
      subscribe: vi.fn(() => channel),
      unsubscribe: vi.fn(),
    }
    const client = { channel: vi.fn(() => channel) }
    const onChange = vi.fn()

    const subscription = subscribeWorkspaceRealtime(client as never, 'workspace-1', onChange)

    expect(client.channel).toHaveBeenCalledWith('workspace:workspace-1')
    expect(channel.on).toHaveBeenCalledTimes(4)
    subscription.unsubscribe()
    expect(channel.unsubscribe).toHaveBeenCalled()
  })
})
