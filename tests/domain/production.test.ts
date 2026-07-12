import { describe, expect, it } from 'vitest'
import { STAGE_DEFINITIONS, stageStatus } from '@/domain/production'

describe('production stage definitions', () => {
  it('keeps the confirmed eight stages including 中版', () => {
    expect(STAGE_DEFINITIONS.map((stage) => stage.label)).toEqual([
      '分镜',
      '初版',
      '精修',
      '中版',
      '特效',
      '美术字',
      '音乐',
      '合成终版',
    ])
  })

  it('keeps every fixed foreground/background pair at 4.5:1 or higher', () => {
    for (const stage of STAGE_DEFINITIONS) {
      expect(stage.contrastRatio).toBeGreaterThanOrEqual(4.5)
    }
  })

  it('derives status only from independent progress', () => {
    expect(stageStatus(0)).toBe('not_started')
    expect(stageStatus(1)).toBe('in_progress')
    expect(stageStatus(99)).toBe('in_progress')
    expect(stageStatus(100)).toBe('completed')
  })
})
