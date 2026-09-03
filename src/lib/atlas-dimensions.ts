export interface AtlasDimension {
  id: string
  label: string
}

export const ATLAS_DIMENSIONS: AtlasDimension[] = [
  { id: 'scene-concept', label: '场景概念' },
  { id: 'style', label: '风格' },
  { id: 'character-pose', label: '角色姿态' },
  { id: 'composition', label: '构图' },
  { id: 'lighting-atmosphere', label: '灯光氛围' },
  { id: 'fx-effects', label: '特效' },
  { id: 'color-texture', label: '色彩材质' },
  { id: 'emotion-mood', label: '情绪状态' },
  { id: 'camera-lens', label: '镜头' },
  { id: 'model-constraints', label: '模型约束' },
]

export function atlasDimensionLabel(dimension: string): string {
  return ATLAS_DIMENSIONS.find((item) => item.id === dimension)?.label ?? dimension
}
