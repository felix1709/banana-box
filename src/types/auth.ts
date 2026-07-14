export interface AppProfile {
  id: string
  email: string
  displayName: string
  avatarUrl: string | null
  createdAt: string
  updatedAt: string
}

export type AuthMode = 'sign-in' | 'sign-up'
