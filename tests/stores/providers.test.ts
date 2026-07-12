import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import { reverseImagePrompt } from '@/lib/provider-ipc'
import { useProviderStore } from '@/stores/providers'
import type { AiProvider, SaveAiProviderInput } from '@/types/providers'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

const reverseImageProvider: AiProvider = {
  id: 'reverse-image',
  kind: 'reverse-image',
  displayName: '图片反推',
  baseUrl: 'https://api.example.com',
  modelsUrl: 'https://api.example.com/v1/models',
  chatCompletionsUrl: 'https://api.example.com/v1/chat/completions',
  defaultModel: 'vision',
  availableModels: ['vision'],
  probedModel: null,
  structuredMode: null,
  interactiveCompatible: null,
  boundHost: 'https://api.example.com',
  needsCredentials: true,
  configRevision: 1,
  capabilityRevision: 1,
}

function saveInput(apiKey = 'TEST_ONLY_DO_NOT_PERSIST'): SaveAiProviderInput {
  return {
    provider: {
      id: 'reverse-image',
      kind: 'reverse-image',
      displayName: '图片反推',
      baseUrl: 'https://api.example.com',
      modelsUrl: 'https://api.example.com/v1/models',
      chatCompletionsUrl: 'https://api.example.com/v1/chat/completions',
      defaultModel: 'vision',
      confirmCrossOrigin: true,
    },
    apiKey,
  }
}

describe('provider store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('stores provider metadata without credential material', async () => {
    vi.mocked(invoke).mockResolvedValue([reverseImageProvider])
    const store = useProviderStore()

    await store.load('reverse-image')

    expect(invoke).toHaveBeenCalledWith('list_ai_providers', { kind: 'reverse-image' })
    expect(store.byId('reverse-image')?.needsCredentials).toBe(true)
    expect(JSON.stringify(store.providers)).not.toContain('apiKey')
    expect(JSON.stringify(store.providers)).not.toContain('credentialRef')
    expect(JSON.stringify(store.providers)).not.toContain('TEST_ONLY_DO_NOT_PERSIST')
  })

  it('sends a new API key in the one save IPC request and clears the input afterwards', async () => {
    vi.mocked(invoke).mockResolvedValue({
      ...reverseImageProvider,
      needsCredentials: false,
    })
    const store = useProviderStore()
    const value = saveInput()

    await store.save(value)

    expect(invoke).toHaveBeenCalledTimes(1)
    expect(invoke).toHaveBeenCalledWith('save_ai_provider', {
      input: {
        ...value.provider,
        confirmCrossOrigin: true,
      },
      apiKey: 'TEST_ONLY_DO_NOT_PERSIST',
    })
    expect(value.apiKey).toBe('')
    expect(JSON.stringify(store.providers)).not.toContain('apiKey')
    expect(JSON.stringify(store.providers)).not.toContain('credentialRef')
    expect(JSON.stringify(store.providers)).not.toContain('TEST_ONLY_DO_NOT_PERSIST')
  })

  it('defaults cross-origin confirmation to false in the save command payload', async () => {
    vi.mocked(invoke).mockResolvedValue(reverseImageProvider)
    const store = useProviderStore()
    const value = saveInput('')
    delete value.provider.confirmCrossOrigin

    await store.save(value)

    expect(invoke).toHaveBeenCalledWith('save_ai_provider', {
      input: {
        ...value.provider,
        confirmCrossOrigin: false,
      },
      apiKey: '',
    })
  })

  it('marks a provider as needing credentials after clearing its credential', async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce([reverseImageProvider])
      .mockResolvedValueOnce(undefined)
    const store = useProviderStore()
    await store.load('reverse-image')

    await store.clearCredential('reverse-image')

    expect(invoke).toHaveBeenLastCalledWith('clear_ai_provider_credential', {
      providerId: 'reverse-image',
    })
    expect(store.byId('reverse-image')?.needsCredentials).toBe(true)
  })

  it('sends reverse-image requests with a provider ID instead of an API key', async () => {
    vi.mocked(invoke).mockResolvedValue({ prompt: 'a banana note' })

    await reverseImagePrompt({
      providerId: 'reverse-image',
      model: 'vision',
      imagePath: 'images/source.png',
    })

    expect(invoke).toHaveBeenCalledWith('reverse_image_prompt', {
      providerId: 'reverse-image',
      model: 'vision',
      imagePath: 'images/source.png',
    })
  })
})
