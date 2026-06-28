<script setup lang="ts">
import { ref, watch } from 'vue'

const props = defineProps<{
  visible: boolean
  initialName?: string
  initialColor?: string
}>()
const emit = defineEmits<{
  confirm: [payload: { name: string; color: string }]
  close: []
}>()

const PRESET = [
  '#ef4444',
  '#f59e0b',
  '#facc15',
  '#22c55e',
  '#3b82f6',
  '#8b5cf6',
  '#ec4899',
  '#6b7280',
]
const name = ref(props.initialName ?? '')
const color = ref(props.initialColor ?? PRESET[0])

watch(
  () => props.visible,
  (v) => {
    if (v) {
      name.value = props.initialName ?? ''
      color.value = props.initialColor ?? PRESET[0]
    }
  },
)

function onConfirm() {
  const n = name.value.trim()
  if (!n) return
  emit('confirm', { name: n, color: color.value })
}
</script>

<template>
  <div
    v-if="visible"
    class="mask"
    @click.self="emit('close')"
  >
    <div class="dialog">
      <h3>分类</h3>
      <input
        v-model="name"
        type="text"
        placeholder="分类名称"
        @keydown.enter="onConfirm"
      >
      <div class="colors">
        <button
          v-for="c in PRESET"
          :key="c"
          class="swatch"
          :class="{ active: color === c }"
          :style="{ background: c }"
          @click="color = c"
        />
        <input
          v-model="color"
          type="color"
          class="picker"
        >
      </div>
      <div class="actions">
        <button @click="emit('close')">
          取消
        </button>
        <button
          class="primary"
          @click="onConfirm"
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
  width: 320px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
input[type='text'] {
  padding: 6px;
  border: 1px solid #ddd;
  border-radius: 4px;
}
.colors {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
}
.swatch {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  border: 2px solid #fff;
  box-shadow: 0 0 0 1px #ddd;
  cursor: pointer;
}
.swatch.active {
  box-shadow: 0 0 0 2px #333;
}
.picker {
  width: 32px;
  height: 32px;
  border: none;
  background: none;
  cursor: pointer;
}
.actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
.primary {
  font-weight: bold;
}
</style>
