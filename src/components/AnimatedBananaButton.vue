<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue'
import {
  BANANA_CLOSED_FRAME,
  BANANA_OPEN_FRAME,
  frameAt,
  retarget,
  type BananaAnimationState,
} from '@/lib/bananaAnimation'

const props = withDefaults(
  defineProps<{
    open: boolean
    unread?: boolean
  }>(),
  {
    unread: false,
  },
)

const emit = defineEmits<{
  frame: [value: number]
}>()

const frame = ref(props.open ? BANANA_OPEN_FRAME : BANANA_CLOSED_FRAME)
let animation: BananaAnimationState | null = retarget(
  null,
  frame.value,
  performance.now(),
  true,
)
let animationFrameId: number | null = null

function reducedMotionEnabled() {
  return window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false
}

function setFrame(value: number) {
  if (frame.value === value) return
  frame.value = value
  emit('frame', value)
}

function tick(nowMs: number) {
  if (!animation) return

  const nextFrame = frameAt(animation, nowMs)
  setFrame(nextFrame)
  if (nextFrame === animation.targetFrame) {
    animationFrameId = null
    return
  }

  animationFrameId = requestAnimationFrame(tick)
}

function animateTo(open: boolean) {
  if (animationFrameId !== null) {
    cancelAnimationFrame(animationFrameId)
    animationFrameId = null
  }

  const nowMs = performance.now()
  animation = retarget(
    animation,
    open ? BANANA_OPEN_FRAME : BANANA_CLOSED_FRAME,
    nowMs,
    reducedMotionEnabled(),
  )
  setFrame(frameAt(animation, nowMs))

  if (frame.value !== animation.targetFrame) {
    animationFrameId = requestAnimationFrame(tick)
  }
}

watch(() => props.open, animateTo)

onBeforeUnmount(() => {
  if (animationFrameId !== null) {
    cancelAnimationFrame(animationFrameId)
  }
})
</script>

<template>
  <button
    class="animated-banana"
    type="button"
    :class="{ 'has-unread': unread }"
    :data-frame="frame"
    :style="{ '--banana-frame': frame }"
    aria-label="打开或收起 Banana Box"
  >
    <span
      class="banana-sprite"
      aria-hidden="true"
    />
    <span
      v-if="unread"
      class="unread-dot"
      aria-label="有未读提醒"
    />
  </button>
</template>

<style scoped>
.animated-banana {
  position: relative;
  display: inline-grid;
  width: 64px;
  height: 64px;
  padding: 0;
  border: 0;
  background: transparent;
  place-items: center;
  flex: 0 0 64px;
  cursor: pointer;
}

.animated-banana:hover:not(:disabled) {
  border: 0;
  background: transparent;
  box-shadow: none;
}

.banana-sprite {
  display: block;
  width: 52px;
  height: 52px;
  margin: 6px;
  background-image: url('@/assets/banana/banana-peel-sprite.webp');
  background-repeat: no-repeat;
  background-size: 1200% 100%;
  background-position: calc(var(--banana-frame) * 100% / 11) 0;
}

.unread-dot {
  position: absolute;
  top: 7px;
  right: 7px;
  width: 6px;
  height: 6px;
  border: 2px solid #101c24;
  border-radius: 50%;
  background: #ffd85a;
}
</style>
