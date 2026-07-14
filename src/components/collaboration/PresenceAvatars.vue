<script setup lang="ts">
import { computed } from 'vue'
import { usePresenceStore } from '@/stores/presence'

const presence = usePresenceStore()
const initials = computed(() =>
  presence.onlineUsers.slice(0, 4).map((user) => (user.email || user.userId).slice(0, 1).toUpperCase()),
)
</script>

<template>
  <div
    class="presence-avatars"
    title="在线协作人"
    aria-label="在线协作人"
  >
    <span
      v-for="(initial, index) in initials"
      :key="`${initial}-${index}`"
    >
      {{ initial }}
    </span>
    <b v-if="presence.onlineUsers.length > 4">+{{ presence.onlineUsers.length - 4 }}</b>
  </div>
</template>

<style scoped>
.presence-avatars {
  display: inline-flex;
  align-items: center;
  min-height: 28px;
}

.presence-avatars span,
.presence-avatars b {
  display: grid;
  width: 24px;
  height: 24px;
  place-items: center;
  margin-left: -5px;
  border: 1px solid var(--bb-border-strong);
  border-radius: 50%;
  background: var(--bb-surface);
  color: var(--bb-primary-strong);
  font-size: 10px;
  font-weight: 700;
}

.presence-avatars span:first-child {
  margin-left: 0;
}
</style>
