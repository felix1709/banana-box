import { invoke } from '@tauri-apps/api/core'
import type {
  AiProvider,
  CheckAiProviderConnectionResult,
  ProviderKind,
  ReverseImagePromptInput,
  ReverseImagePromptResult,
  SaveAiProviderInput,
} from '@/types/providers'

export async function listAiProviders(kind: ProviderKind): Promise<AiProvider[]> {
  return await invoke<AiProvider[]>('list_ai_providers', { kind })
}

export async function saveAiProvider(value: SaveAiProviderInput): Promise<AiProvider> {
  return await invoke<AiProvider>('save_ai_provider', {
    input: {
      ...value.provider,
      confirmCrossOrigin: value.provider.confirmCrossOrigin ?? false,
    },
    apiKey: value.apiKey,
  })
}

export async function clearAiProviderCredential(providerId: string): Promise<void> {
  await invoke('clear_ai_provider_credential', { providerId })
}

export async function checkAiProviderConnection(
  providerId: string,
): Promise<CheckAiProviderConnectionResult> {
  return await invoke<CheckAiProviderConnectionResult>('check_ai_provider_connection', {
    providerId,
  })
}

export async function reverseImagePrompt(
  input: ReverseImagePromptInput,
): Promise<ReverseImagePromptResult> {
  return await invoke<ReverseImagePromptResult>('reverse_image_prompt', {
    providerId: input.providerId,
    model: input.model,
    imagePath: input.imagePath,
  })
}
