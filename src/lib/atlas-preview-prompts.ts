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

function sectionLine(title: string, values: string[]) {
  const content = values.length > 0 ? values.join('，') : UNKNOWN
  return `${title}${content}`
}

export function buildGptPrompt(source: string) {
  const map = extractAnalysisMap(source)
  const sections = [
    sectionLine(
      '【主体描述】',
      pick(map, ['身份特征', '姿态神态', '穿着配饰', '画面大类']),
    ),
    sectionLine(
      '【风格】',
      pick(map, ['摄影风格', '整体氛围', '情绪传递', '整体质感', '色彩风格']),
    ),
    sectionLine(
      '【场景】',
      pick(map, ['背景类型', '场景空间', '氛围元素', '背景与主体的关联']),
    ),
    sectionLine(
      '【构图光线】',
      pick(map, ['景别', '镜头视角', '景深对焦', '空间层次', '视图布局', '光源属性', '光位光质', '光影特征', '整体影调']),
    ),
    '【比例】16:9 横构图',
    '【约束】画面无文字，发丝通透柔顺，布料物理真实',
  ]
  return sections.join('\n')
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
  ['碎花丝绸长裙', 'floral silk dress'],
  ['单只白色翅膀', 'single white wing'],
  ['发丝飞扬', 'flowing hair'],
  ['面料轻盈飘逸', 'lightweight flowing fabric'],
  ['五官柔和精致', 'delicate facial features'],
  ['身形窈窕', 'slender figure'],
  ['浅棕色', 'light brown hair'],
  ['草编包', 'woven straw bag'],
  ['斗笠', 'bamboo hat'],
  ['佩剑', 'wearing a sword'],
  ['武士风格', 'warrior style'],
  ['深色长袍', 'dark robe'],
  ['蕾丝上衣', 'lace top'],
  ['绸缎长裙', 'satin dress'],
  ['白色翅膀', 'white wing'],
  ['丝绸长裙', 'silk dress'],
  ['长裙', 'long dress'],
  ['裙摆', 'flowing dress'],
  ['仙女', 'celestial maiden'],
  ['白狼', 'white wolf'],
  ['兽耳', 'animal ears'],
  ['兽尾', 'animal tail'],
  ['漂浮', 'floating'],
  ['悬浮', 'floating'],
  ['手持花束', 'holding bouquet'],
  ['花束', 'bouquet'],
  ['手持', 'holding'],
  ['长发', 'long hair'],
  ['短发', 'short hair'],
  ['闭眼', 'closed eyes'],
  ['低眸', 'downcast eyes'],
  ['微笑', 'gentle smile'],
  ['放松表情', 'relaxed expression'],
  ['青年', 'young'],
  ['少女', 'young woman'],
  ['深空', 'deep space'],
  ['宇宙', 'cosmic space'],
  ['祭坛', 'golden cosmic altar'],
  ['户外自然场景', 'outdoor natural scene'],
  ['室内复古场景', 'vintage interior scene'],
  ['自然田园', 'pastoral countryside'],
  ['开阔草地', 'open grassland'],
  ['森林', 'forest'],
  ['水边', 'waterside'],
  ['水面', 'water surface'],
  ['花园', 'garden'],
  ['草地', 'grass field'],
  ['野花', 'wildflowers'],
  ['远山', 'distant mountains'],
  ['树木', 'trees'],
  ['天空', 'sky'],
  ['晴空', 'clear sky'],
  ['夜空', 'night sky'],
  ['岩石', 'rocks'],
  ['房间', 'interior room'],
  ['墙面', 'wall'],
  ['白鸽', 'white doves'],
  ['蕾丝伞', 'lace umbrella'],
  ['星星状光斑', 'starry bokeh'],
  ['水面倒影', 'water reflection'],
  ['雾气', 'mist'],
  ['烟雾', 'smoke'],
  ['雨丝', 'rain streaks'],
  ['尘埃', 'dust particles'],
  ['粒子', 'particles'],
  ['发光粒子', 'luminous particles'],
  ['光斑', 'bokeh'],
  ['丁达尔', 'volumetric light'],
  ['柔焦朦胧', 'soft focus'],
  ['胶片颗粒', 'film grain'],
  ['光晕眩光', 'lens flare'],
  ['动态模糊', 'motion blur'],
  ['暗角', 'vignette'],
  ['色散', 'chromatic aberration'],
  ['暖调黄昏光', 'warm golden hour light'],
  ['冷蓝轮廓光', 'cold blue rim light'],
  ['阴影过渡柔和', 'soft shadows'],
  ['明暗对比中等', 'balanced contrast'],
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
  ['复古胶片感', 'retro film color'],
  ['复古人像', 'retro portrait photography'],
  ['大师级人像', 'master portrait photography'],
  ['时尚大片', 'fashion editorial'],
  ['纪实摄影', 'documentary photography'],
  ['复古胶片', 'retro film photography'],
  ['创意摄影', 'creative photography'],
  ['国风水彩插画', 'Chinese watercolor illustration'],
  ['水彩', 'watercolor'],
  ['插画', 'illustration'],
  ['新中式', 'new chinese style'],
  ['中国风', 'chinese style'],
  ['古风', 'ancient chinese style'],
  ['科幻', 'sci-fi'],
  ['奇幻', 'fantasy'],
  ['优雅', 'elegant'],
  ['梦幻', 'dreamy'],
  ['浪漫', 'romantic mood'],
  ['神秘', 'mysterious mood'],
  ['史诗', 'epic mood'],
  ['自由', 'free-spirited mood'],
  ['温暖', 'warm mood'],
  ['清冷', 'cool serene mood'],
  ['温柔', 'gentle mood'],
  ['静谧', 'quiet mood'],
  ['热烈', 'passionate mood'],
  ['疏离', 'distant mood'],
  ['治愈', 'healing mood'],
  ['肃穆', 'solemn mood'],
  ['超写实', 'ultra-realistic'],
  ['真实', 'photorealistic'],
  ['电影', 'cinematic'],
  ['PBR', 'PBR material'],
  ['丝绸', 'silk'],
  ['绸缎', 'satin'],
  ['蕾丝', 'lace'],
  ['编织纹理', 'woven texture'],
  ['金属反光', 'metallic reflections'],
  ['水体折射', 'water refraction'],
  ['石材肌理', 'stone texture'],
  ['皮肤通透', 'translucent skin'],
  ['发丝清晰', 'detailed hair strands'],
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
  ['三分法', 'rule of thirds'],
  ['中心构图', 'centered composition'],
  ['框架式', 'framed composition'],
  ['对称', 'symmetrical composition'],
  ['单视图', 'single view'],
  ['三视图', 'three-view'],
  ['多视图', 'multi-view'],
  ['浅景深', 'shallow depth of field'],
  ['深景深', 'deep depth of field'],
  ['背景虚化', 'blurred background'],
  ['前景', 'foreground'],
  ['远景', 'distant view'],
]

const MJ_COLOR_WORDS: Array<[needle: string, en: string]> = [
  ['红色', 'red'],
  ['橙色', 'orange'],
  ['黄色', 'yellow'],
  ['绿色', 'green'],
  ['青色', 'cyan'],
  ['蓝色', 'blue'],
  ['紫色', 'purple'],
  ['粉色', 'pink'],
  ['棕色', 'brown'],
  ['白色', 'white'],
  ['黑色', 'black'],
  ['灰色', 'gray'],
  ['金色', 'golden'],
  ['银色', 'silver'],
]

function collectTerms(values: Array<string | undefined>) {
  const result: string[] = []
  for (const value of values) {
    if (!value) continue
    for (const [needle, en] of MJ_TERMS) {
      if (value.includes(needle)) result.push(en)
    }
  }
  return unique(result)
}

function collectColorTerms(value: string | undefined) {
  if (!value) return []
  return MJ_COLOR_WORDS.filter(([needle]) => value.includes(needle)).map(([, en]) => en)
}

function hasAny(values: Array<string | undefined>, needles: string[]) {
  return values.some((value) => needles.some((needle) => value?.includes(needle)))
}

function subjectPrefix(values: Array<string | undefined>) {
  if (hasAny(values, ['女性'])) return '1girl'
  if (hasAny(values, ['男性'])) return '1boy'
  if (hasAny(values, ['儿童', '幼童', '孩童'])) return '1child'
  if (hasAny(values, ['白狼', '狼'])) return '1wolf'
  if (hasAny(values, ['动物', '猫', '狗'])) return '1animal'
  return '1person'
}


function unique(values: string[]) {
  return Array.from(new Set(values.filter(Boolean)))
}

export function buildMjPrompt(source: string, tags: string[] = []) {
  const map = extractAnalysisMap(source)
  const ratio = parseAspectRatio(map['画幅比例'])

  const subjectValues = pick(map, ['身份特征', '画面大类', '穿着配饰', '姿态神态'])
  const subject = [subjectPrefix(subjectValues)]

  const keyFeatures = unique([
    ...collectTerms(subjectValues),
    ...collectTerms(pick(map, ['专属特征', '氛围元素'])),
    ...collectTerms(tags),
  ])

  const styleEnvironment = collectTerms(pick(map, [
    '摄影风格',
    '整体氛围',
    '情绪传递',
    '色彩风格',
    '背景类型',
    '场景空间',
    '背景与主体的关联',
  ]))

  const materialLighting = unique([
    'high detail',
    'PBR material',
    'soft glow',
    'cinematic lighting',
    ...collectTerms(pick(map, [
      '材质与质感',
      '光源属性',
      '光位光质',
      '光影特征',
      '整体影调',
      '整体色调',
      '色彩质感',
      '画面特效',
      '微观细节',
    ])),
    ...collectColorTerms(map['主要色彩']),
  ])

  const segments = unique([
    ...subject,
    ...keyFeatures,
    ...styleEnvironment,
    ...materialLighting,
    ratio,
  ])

  const suffix = `--ar ${ratio} --style raw --stylize 150 --v 8.1 --no text, watermark`
  return `${segments.join(', ')} ${suffix}`.replace(/\s+/g, ' ').trim()
}
