<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { UserCircle } from '@lucide/vue'
import { useAuthStore } from '@/stores/auth'
import { useWorkspacesStore } from '@/stores/workspaces'
import LoginPanel from '@/components/auth/LoginPanel.vue'
import WorkspaceSwitcher from '@/components/workspaces/WorkspaceSwitcher.vue'
import InviteAcceptPanel from '@/components/collaboration/InviteAcceptPanel.vue'
import SyncStatusIndicator from '@/components/cloud/SyncStatusIndicator.vue'
import NotificationsMenu from '@/components/collaboration/NotificationsMenu.vue'

const auth = useAuthStore()
const workspaces = useWorkspacesStore()
const open = ref(false)
const trigger = ref<HTMLButtonElement | null>(null)
const popover = ref<HTMLElement | null>(null)
const popoverStyle = ref<Record<string, string>>({})

const label = computed(() => {
  if (workspaces.error) return '用户与云端：需要处理'
  if (auth.user) return '用户与云端'
  if (auth.cloudAvailable) return '登录或注册'
  return '本地模式'
})

async function positionPopover() {
  await nextTick()
  const rect = trigger.value?.getBoundingClientRect()
  if (!rect) return
  const width = Math.min(310, window.innerWidth - 24)
  const left = Math.max(12, Math.min(window.innerWidth - width - 12, rect.right - width))
  popoverStyle.value = {
    position: 'fixed',
    zIndex: '240',
    top: `${rect.bottom + 8}px`,
    left: `${left}px`,
    width: `${width}px`,
  }
}

async function toggle() {
  open.value = !open.value
  if (open.value) await positionPopover()
}

function closeWhenClickingOutside(event: MouseEvent) {
  if (!open.value) return
  const target = event.target
  if (!(target instanceof Node)) return
  if (trigger.value?.contains(target)) return
  if (popover.value?.contains(target)) return
  open.value = false
}

onMounted(() => {
  document.addEventListener('click', closeWhenClickingOutside)
})

onBeforeUnmount(() => {
  document.removeEventListener('click', closeWhenClickingOutside)
})

watch(
  () => auth.user?.id ?? '',
  (userId, previousUserId) => {
    if (open.value && userId && !previousUserId) {
      open.value = false
    }
  },
)
</script>

<template>
  <div class="user-cloud-menu">
    <button
      ref="trigger"
      class="user-cloud-trigger"
      data-action="user-cloud-menu"
      type="button"
      :title="label"
      :aria-label="label"
      :aria-expanded="open"
      @click="toggle"
    >
      <UserCircle
        :size="17"
        aria-hidden="true"
      />
    </button>

    <Teleport to="body">
      <section
        v-if="open"
        ref="popover"
        class="user-cloud-popover"
        aria-label="用户与云端"
        :style="popoverStyle"
      >
        <header>
          <strong>用户与云端</strong>
          <NotificationsMenu v-if="auth.user && auth.isCloudAdmin" />
        </header>
        <WorkspaceSwitcher />
        <SyncStatusIndicator />
        <LoginPanel v-if="auth.cloudAvailable && !auth.user" />
        <InviteAcceptPanel v-if="auth.user && auth.isCloudAdmin" />
        <p
          v-if="workspaces.error"
          class="user-cloud-error"
          role="alert"
        >
          {{ workspaces.error }}
        </p>
      </section>
    </Teleport>
  </div>
</template>

<style scoped>
.user-cloud-menu {
  position: relative;
  display: inline-flex;
}

.user-cloud-trigger {
  display: grid;
  width: 28px;
  min-height: 28px;
  place-items: center;
  padding: 0;
}

.user-cloud-popover {
  position: fixed;
  z-index: 240;
  max-height: min(480px, calc(100vh - 86px));
  overflow: auto;
  display: grid;
  gap: 9px;
  padding: 10px;
  border: 1px solid var(--bb-border-strong);
  border-radius: var(--bb-radius-md);
  background: rgba(5, 14, 22, 0.98);
  box-shadow: var(--bb-shadow-floating);
}

.user-cloud-popover header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.user-cloud-popover header strong {
  font-size: 13px;
}

.user-cloud-error {
  margin: 0;
  color: #ffb6c0;
  font-size: 11px;
  line-height: 1.5;
}
</style>
