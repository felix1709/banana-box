export const BANANA_FRAME_COUNT = 12
export const BANANA_CLOSED_FRAME = 0
export const BANANA_OPEN_FRAME = 11
export const BANANA_REVEAL_FRAME = 6
export const BANANA_TOTAL_MS = 360

export interface BananaAnimationState {
  startFrame: number
  targetFrame: number
  startedAtMs: number
  durationMs: number
  revealAtMs: number
}

function clampFrame(frame: number) {
  return Math.min(BANANA_OPEN_FRAME, Math.max(BANANA_CLOSED_FRAME, Math.round(frame)))
}

export function frameAt(state: BananaAnimationState, nowMs: number) {
  if (state.durationMs === 0) return state.targetFrame
  const progress = Math.min(1, Math.max(0, (nowMs - state.startedAtMs) / state.durationMs))

  return clampFrame(state.startFrame + (state.targetFrame - state.startFrame) * progress)
}

export function retarget(
  previous: BananaAnimationState | null,
  targetFrame: number,
  nowMs: number,
  reducedMotion = false,
): BananaAnimationState {
  const startFrame = previous ? frameAt(previous, nowMs) : BANANA_CLOSED_FRAME
  const target = clampFrame(targetFrame)
  const distance = Math.abs(target - startFrame)
  const durationMs = reducedMotion ? 0 : Math.round((BANANA_TOTAL_MS * distance) / BANANA_OPEN_FRAME)

  return {
    startFrame,
    targetFrame: target,
    startedAtMs: nowMs,
    durationMs,
    revealAtMs: Math.round((BANANA_TOTAL_MS * BANANA_REVEAL_FRAME) / BANANA_OPEN_FRAME),
  }
}
