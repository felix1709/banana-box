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
  background: rgba(0, 0, 0, 0.3);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 20;
}
.dialog {
  background: #fff;
  padding: 16px;
  border-radius: 8px;
  width: 300px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.msg {
  margin: 0;
}
.actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
.danger {
  font-weight: bold;
  color: #c00;
}
</style>
