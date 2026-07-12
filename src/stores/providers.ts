import { defineStore } from 'pinia'
import type { AiProvider, ProviderKind, SaveAiProviderInput } from '@/types/providers'
import * as providerIpc from '@/lib/provider-ipc'

function copyPublicProvider(provider: AiProvider): AiProvider {
  return {
    id: provider.id,
    kind: provider.kind,
    displayName: provider.displayName,
    baseUrl: provider.baseUrl,
    modelsUrl: provider.modelsUrl,
    chatCompletionsUrl: provider.chatCompletionsUrl,
    defaultModel: provider.defaultModel,
    availableModels: [...provider.availableModels],
    probedModel: provider.probedModel,
    structuredMode: provider.structuredMode,
    interactiveCompatible: provider.interactiveCompatible,
    boundHost: provider.boundHost,
    needsCredentials: provider.needsCredentials,
    configRevision: provider.configRevision,
    capabilityRevision: provider.capabilityRevision,
    temperature: provider.temperature,
    contextWindowTokens: provider.contextWindowTokens,
  }
}

export const useProviderStore = defineStore('providers', {
  state: () => ({
    providers: [] as AiProvider[],
  }),
  getters: {
    byId: (state) => (id: string) => state.providers.find((provider) => provider.id === id),
  },
  actions: {
    async load(kind: ProviderKind) {
      const loaded = (await providerIpc.listAiProviders(kind))
        .filter((provider) => provider.kind === kind)
        .map(copyPublicProvider)
      this.providers = [...this.providers.filter((provider) => provider.kind !== kind), ...loaded]
    },
    async save(input: SaveAiProviderInput): Promise<AiProvider> {
      const payload: SaveAiProviderInput = {
        provider: { ...input.provider },
        apiKey: input.apiKey,
      }
      try {
        const saved = copyPublicProvider(await providerIpc.saveAiProvider(payload))
        const index = this.providers.findIndex((provider) => provider.id === saved.id)
        if (index === -1) {
          this.providers.push(saved)
        } else {
          this.providers[index] = saved
        }
        return saved
      } finally {
        input.apiKey = ''
      }
    },
    async clearCredential(id: string) {
      await providerIpc.clearAiProviderCredential(id)
      const provider = this.providers.find((candidate) => candidate.id === id)
      if (provider) provider.needsCredentials = true
    },
  },
})
