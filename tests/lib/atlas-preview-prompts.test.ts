import { describe, expect, it } from 'vitest'
import {
  buildGptPrompt,
  buildMjPrompt,
  extractAnalysisMap,
} from '@/lib/atlas-preview-prompts'

const source = `---
id: a
---

## 分析
- 画幅比例：16:9
- 视图布局：单视图
- 画面大类：人像
- 景别：面部特写
- 镜头视角：平视
- 光源属性：自然光
- 整体色调：冷调
- 摄影风格：大师级人像
- 需规避元素：文字、水印
`

describe('atlas preview prompt helpers', () => {
  it('extracts bullet fields from the analysis markdown', () => {
    const map = extractAnalysisMap(source)
    expect(map['画幅比例']).toBe('16:9')
    expect(map['画面大类']).toBe('人像')
  })

  it('builds a multi-line Chinese GPT prompt', () => {
    const output = buildGptPrompt(source)
    expect(output).toContain('【主体描述】人像')
    expect(output).toContain('【风格】大师级人像')
    expect(output).toContain('【场景】未明显体现')
    expect(output).toContain('【构图光线】面部特写，平视，单视图，自然光')
    expect(output).toContain('【比例】16:9 横构图')
    expect(output).toContain('【约束】画面无文字，发丝通透柔顺，布料物理真实')
    expect(output.split('\n')).toHaveLength(6)
  })

  it('builds an English MJ V8 prompt with aspect ratio', () => {
    const output = buildMjPrompt(source, ['人像'])
    expect(output).toContain('--ar 16:9')
    expect(output).toContain('--v 8')
    expect(output).toContain('master portrait photography')
    expect(/[\u4e00-\u9fff]/.test(output)).toBe(false)
  })
})
