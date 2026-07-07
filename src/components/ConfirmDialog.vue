<script setup lang="ts">
defineProps<{ visible: boolean; message: string }>()
const emit = defineEmits<{
  confirm: []
  cancel: []
}>()
</script>

<template>
  <div
    v-if="visible"
    class="mask"
    @click.self="emit('cancel')"
  >
    <div class="dialog">
      <p class="msg">
        {{ message }}
      </p>
      <div class="actions">
        <button @click="emit('cancel')">
          取消
        </button>
        <button
          class="danger"
          @click="emit('confirm')"
        >
          确定
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.mask {
  position: fixed;
  inset: 0;
  background: rgba(15, 23, 42, 0.36);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 20;
  backdrop-filter: blur(2px);
}
.dialog {
  width: 300px;
  max-width: calc(100vw - 24px);
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px;
  border: 1px solid var(--bb-border);
  border-radius: var(--bb-radius-lg);
  background: var(--bb-surface);
  box-shadow: var(--bb-shadow-dialog);
}
.msg {
  margin: 0;
  color: var(--bb-text);
  line-height: 1.5;
  overflow-wrap: anywhere;
}
.actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
.danger {
  border-color: var(--bb-danger);
  background: var(--bb-danger);
  color: #fff;
  font-weight: 600;
}
.danger:hover {
  border-color: #b91c1c;
  background: #b91c1c;
}
</style>
