import { defineStore } from 'pinia'
import type { CloudConfig, CloudReadiness, SaveCloudConfigInput } from '@/types'
import { cloudReadiness, normalizeCloudConfigInput, validateCloudConfigInput } from '@/lib/cloud-config'
import { loadCloudConfig, saveCloudConfig } from '@/lib/ipc'

export const useCloudSessionStore = defineStore('cloudSession', {
  state: () => ({
    config: null as CloudConfig | null,
    loading: false,
    saving: false,
    error: '',
  }),
  getters: {
    readiness(state): CloudReadiness {
      if (state.error && state.config?.cloudEnabled) return 'invalid'
      return cloudReadiness(state.config)
    },
  },
  actions: {
    async load() {
      this.loading = true
      this.error = ''
      try {
        this.config = await loadCloudConfig()
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error)
      } finally {
        this.loading = false
      }
    },
    async save(input: SaveCloudConfigInput) {
      this.saving = true
      this.error = ''
      const normalized = normalizeCloudConfigInput(input)
      const validation = validateCloudConfigInput(normalized, {
        hasExistingAnonKey: Boolean(this.config?.hasAnonKey),
      })
      if (!validation.ok) {
        this.config = {
          supabaseUrl: normalized.supabaseUrl,
          hasAnonKey: false,
          cloudEnabled: normalized.cloudEnabled,
          updatedAt: null,
        }
        this.error = validation.code
        this.saving = false
        return
      }

      try {
        this.config = await saveCloudConfig(normalized)
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error)
      } finally {
        this.saving = false
      }
    },
  },
})
