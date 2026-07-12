import { invoke } from '@tauri-apps/api/core'

export interface MigrationSummary {
  promptsMigrated: number
  favoritesDefaulted: number
  ordersRebuilt: number
  backupPath: string
  warnings: string[]
}

export interface StartupReadyStatus {
  state: 'ready'
  migrationSummary: MigrationSummary | null
}

export interface StartupRecoveryStatus {
  state: 'recovery'
  message: string
  backupPaths: string[]
}

export type StartupStatus = StartupReadyStatus | StartupRecoveryStatus

export async function getStartupStatus(): Promise<StartupStatus> {
  return await invoke<StartupStatus>('get_startup_status', {})
}

export async function acknowledgeMigrationSummary(): Promise<void> {
  await invoke('acknowledge_migration_summary', {})
}
