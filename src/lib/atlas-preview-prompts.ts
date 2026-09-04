export type AtlasPreviewPromptMode = 'analysis' | 'gpt' | 'mj'

const UNKNOWN = '未明显体现'

function stripFrontmatter(source: string) {
  if (!source) return ''
  const trimmed = source.trimStart()
  if (!trimmed.startsWith('---\n')) return source
  const rest = trimmed.slice(4)
  const end = rest.indexOf('\n---')
  if (end === -1) return source
  return rest.slice(end + 4).trimStart()
}

function normalizeValue(value: string) {
  const cleaned = value.replace(/^\s*[-*]\s*/, '').trim()
  return cleaned || UNKNOWN
}

export function extractAnalysisMap(source: string): Record<string, string> {
  const body = stripFrontmatter(source)
  const result: Record<string, string> = {}
  for (const rawLine of body.split(/\r?\n/)) {
    const line = rawLine.trim()
    if (!line.startsWith('-')) continue
    const content = line.replace(/^\s*-\s*/, '')
    const separator = content.includes('：') ? '：' : content.includes(':') ? ':' : ''
    if (!separator) continue
    const [label, ...valueParts] = content.split(separator)
    const key = label.trim()
    const value = normalizeValue(valueParts.join(separator))
    if (key) result[key] = value
  }
  return result
}

function pick(map: Record<string, string>, keys: string[]) {
  return keys
    .map((key) => map[key])
    .filter((value): value is string => Boolean(value && value !== UNKNOWN))
}

function paragraph(title: string, values: string[]) {
  return `${title}\n${values.map((value) => `- ${value}`).join('\n')}`
}

export function buildGptPrompt(source: string) {
  const map = extractAnalysisMap(source)
  const sections = [
    paragraph(
      '基础画面属性',
      pick(map, ['画幅比例', '视图布局', '画面大类']),
    ),
    paragraph(
      '核心主体信息',
      pick(map, ['身份特征', '姿态神态', '穿着配饰']),
    ),
    paragraph(
      '构图与镜头语言',
      pick(map, ['景别', '镜头视角', '景深对焦', '空间层次']),
    ),
    paragraph(
      '光影体系',
      pick(map, ['光源属性', '光位光质', '光影特征', '整体影调']),
    ),
    paragraph(
      '色彩与色调',
      pick(map, ['整体色调', '色彩质感', '色彩风格', '主要色彩']),
    ),
    paragraph(
      '材质与质感',
      pick(map, ['皮肤质感', '毛发质感', '物体面料', '整体质感']),
    ),
    paragraph(
      '环境与背景',
      pick(map, ['背景类型', '场景空间', '氛围元素', '背景与主体的关联']),
    ),
    paragraph(
      '风格与情绪调性',
      pick(map, ['摄影风格', '整体氛围', '情绪传递']),
    ),
    paragraph(
      '特殊效果与细节',
      pick(map, ['画面特效', '微观细节', '专属特征']),
    ),
    paragraph(
      '反向约束维度',
      pick(map, ['画面原生瑕疵', 'AI常见通病', '需规避元素']),
    ),
  ]
  return sections.filter((section) => !section.endsWith(`\n- ${UNKNOWN}`)).join('\n\n')
}

function parseAspectRatio(value: string | undefined) {
  const match = value?.match(/\d+\s*[:：]\s*\d+/)
  if (match) return match[0].replace('：', ':')
  if (value?.includes('横版')) return '16:9'
  if (value?.includes('竖版')) return '3:4'
  if (value?.includes('正方形')) return '1:1'
  return '3:4'
}

const MJ_TERMS: Array<[needle: string, en: string]> = [
  ['面部特写', 'close-up portrait'],
  ['胸像', 'bust shot'],
  ['中景', 'medium shot'],
  ['全景', 'full shot'],
  ['全身景', 'full body shot'],
  ['标准镜头', 'standard lens'],
  ['长焦', 'telephoto lens'],
  ['广角', 'wide-angle lens'],
  ['平视', 'eye-level view'],
  ['仰视', 'low-angle view'],
  ['俯视', 'high-angle view'],
  ['自然光', 'natural light'],
  ['窗光', 'window light'],
  ['天光', 'skylight'],
  ['逆光', 'backlight'],
  ['影棚光', 'studio light'],
  ['环境漫射光', 'ambient diffuse light'],
  ['顺光', 'front light'],
  ['侧光', 'side light'],
  ['侧逆光', 'rim light'],
  ['伦勃朗光', 'Rembrandt lighting'],
  ['柔光', 'soft light'],
  ['硬光', 'hard light'],
  ['漫射光', 'diffuse light'],
  ['亮调', 'high-key'],
  ['中间调', 'middle-key'],
  ['暗调', 'low-key'],
  ['冷调', 'cool color palette'],
  ['暖调', 'warm color palette'],
  ['中性调', 'neutral color palette'],
  ['高饱和', 'high saturation'],
  ['低饱和', 'low saturation'],
  ['莫兰迪', 'Morandi muted colors'],
  ['胶片色', 'film color grading'],
  ['日系清透', 'Japanese fresh airy color'],
  ['德系厚重', 'German heavy color grade'],
  ['大师级人像', 'master portrait photography'],
  ['时尚大片', 'fashion editorial'],
  ['纪实摄影', 'documentary photography'],
  ['复古胶片', 'retro film photography'],
  ['创意摄影', 'creative photography'],
  ['清冷', 'cool serene mood'],
  ['温柔', 'gentle mood'],
  ['静谧', 'quiet mood'],
  ['热烈', 'passionate mood'],
  ['疏离', 'distant mood'],
  ['治愈', 'healing mood'],
  ['肃穆', 'solemn mood'],
  ['动态模糊', 'motion blur'],
  ['柔焦朦胧', 'soft focus'],
  ['光晕眩光', 'lens flare'],
  ['胶片颗粒', 'film grain'],
  ['色散', 'chromatic aberration'],
  ['暗角', 'vignette'],
  ['文字', 'no text'],
  ['水印', 'no watermark'],
  ['logo', 'no logo'],
  ['超写实', 'ultra-realistic'],
  ['真实', 'photorealistic'],
  ['电影', 'cinematic'],
]

function mjTerms(value: string | undefined) {
  if (!value) return []
  return MJ_TERMS.filter(([needle]) => value.includes(needle)).map(([, en]) => en)
}

function unique(values: string[]) {
  return Array.from(new Set(values.filter(Boolean)))
}

export function buildMjPrompt(source: string, tags: string[] = []) {
  const map = extractAnalysisMap(source)
  const ratio = parseAspectRatio(map['画幅比例'])
  const tagTerms = unique(
    tags.flatMap((tag) => mjTerms(tag)),
  )
  const subject = tagTerms.length ? tagTerms.join(', ') : 'subject'
  const shot = unique([
    ...mjTerms(map['景别']),
    ...mjTerms(map['镜头视角']),
    ...mjTerms(map['空间层次']),
  ])
  const lighting = unique([
    ...mjTerms(map['光源属性']),
    ...mjTerms(map['光位光质']),
    ...mjTerms(map['光影特征']),
    ...mjTerms(map['整体影调']),
  ])
  const color = unique([
    ...mjTerms(map['整体色调']),
    ...mjTerms(map['色彩质感']),
    ...mjTerms(map['色彩风格']),
  ])
  const style = unique([
    ...mjTerms(map['摄影风格']),
    ...mjTerms(map['整体氛围']),
    ...mjTerms(map['情绪传递']),
    ...mjTerms(map['整体质感']),
  ])
  const details = unique([
    ...mjTerms(map['画面特效']),
    ...mjTerms(map['微观细节']),
    ...mjTerms(map['专属特征']),
  ])
  const negative = unique([...mjTerms(map['需规避元素']), ...mjTerms(map['画面原生瑕疵']), ...mjTerms(map['AI常见通病'])])

  const segments = [
    'cinematic still',
    subject,
    shot.join(', '),
    lighting.join(', '),
    color.join(', '),
    style.join(', '),
    details.join(', '),
  ].filter(Boolean)

  const suffix = [
    negative.length ? `--no ${negative.join(', ')}` : '',
    `--ar ${ratio}`,
    '--style raw',
    '--v 8',
  ].filter(Boolean).join(' ')

  return `${segments.join(', ')} ${suffix}`.replace(/\s+/g, ' ').trim()
}
