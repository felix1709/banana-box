export type ProviderKind = 'reverse-image' | 'storyboard'
export type StructuredMode = 'json_schema' | 'strict_json'

export interface AiProvider {
  id: string
  kind: ProviderKind
  displayName: string
  baseUrl: string
  modelsUrl: string
  chatCompletionsUrl: string
  defaultModel: string | null
  availableModels: string[]
  probedModel: string | null
  structuredMode: StructuredMode | null
  interactiveCompatible: boolean | null
  boundHost: string | null
  needsCredentials: boolean
  configRevision: number
  capabilityRevision: number
}

export interface SaveAiProviderInput {
  provider: Omit<
    AiProvider,
    | 'availableModels'
    | 'probedModel'
    | 'structuredMode'
    | 'interactiveCompatible'
    | 'boundHost'
    | 'needsCredentials'
    | 'configRevision'
    | 'capabilityRevision'
  > & {
    confirmCrossOrigin?: boolean
  }
  apiKey?: string
}

export interface CheckAiProviderConnectionResult {
  ok: boolean
  message: string
  models: string[]
}

export interface ReverseImagePromptInput {
  providerId: string
  model: string
  imagePath: string
}

export interface ReverseImagePromptResult {
  prompt: string
}
