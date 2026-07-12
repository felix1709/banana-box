import { describe, expect, it } from 'vitest'
import {
  BANANA_CLOSED_FRAME,
  BANANA_FRAME_COUNT,
  BANANA_OPEN_FRAME,
  BANANA_REVEAL_FRAME,
  BANANA_TOTAL_MS,
  frameAt,
  retarget,
} from '@/lib/bananaAnimation'

describe('banana animation state', () => {
  it('uses 12 frames over 360ms and reaches frame 6 at the reveal point', () => {
    expect(BANANA_FRAME_COUNT).toBe(12)
    expect(BANANA_TOTAL_MS).toBe(360)
    const state = retarget(null, BANANA_OPEN_FRAME, 0)

    expect(frameAt(state, 0)).toBe(BANANA_CLOSED_FRAME)
    expect(frameAt(state, state.revealAtMs)).toBe(BANANA_REVEAL_FRAME)
    expect(frameAt(state, 360)).toBe(BANANA_OPEN_FRAME)
  })

  it('reverses from the currently displayed frame without jumping', () => {
    const opening = retarget(null, BANANA_OPEN_FRAME, 0)
    const current = frameAt(opening, 180)
    const closing = retarget(opening, BANANA_CLOSED_FRAME, 180)

    expect(closing.startFrame).toBe(current)
    expect(frameAt(closing, 180)).toBe(current)
    expect(frameAt(closing, 540)).toBe(BANANA_CLOSED_FRAME)
  })

  it('collapses immediately to the target when reduced motion is active', () => {
    const state = retarget(null, BANANA_OPEN_FRAME, 10, true)

    expect(frameAt(state, 10)).toBe(BANANA_OPEN_FRAME)
  })
})
