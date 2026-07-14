// src/types/index.ts
// 核心数据类型，与 Rust 端 library.rs 的 struct 对齐（serde camelCase）

export type {
  CloudConfig,
  CloudConfigValidationResult,
  CloudReadiness,
  CloudRuntimeConfig,
  SaveCloudConfigInput,
} from './cloud'

export type { AppProfile, AuthMode } from './auth'
export type { Workspace, WorkspaceMember, WorkspaceRole } from './workspace'
export type {
  AppNotification,
  CollaborationInvite,
  CommentItem,
  CommentTargetType,
  InviteScopeType,
  NotificationKind,
  PresenceUser,
} from './collaboration'

export type {
  AiProvider,
  CheckAiProviderConnectionResult,
  ProviderKind,
  ReverseImagePromptInput,
  ReverseImagePromptResult,
  SaveAiProviderInput,
  StructuredMode,
} from './providers'

export interface Category {
  id: string
  name: string
  color: string
  order: number
}

export interface Prompt {
  id: string
  title: string
  content: string
  categoryId: string | null
  tags: string[]
  image: string | null // 相对路径，如 images/abc.png
  favorite: boolean
  order: number
  createdAt: number
  updatedAt: number
}

export interface Settings {
  hotkey: string
  theme: 'auto' | 'light' | 'dark'
}

export interface Library {
  version: number
  categories: Category[]
  prompts: Prompt[]
  settings: Settings
}
