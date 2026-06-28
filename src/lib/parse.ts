// src/lib/parse.ts
// 解析提示词库目录的 .md/.txt 文件，提取提示词 + 图片链接。

export interface ParsedPrompt {
  title: string
  content: string
  tags: string[]
  imageUrl?: string
}

export interface ParsedFile {
  category: string
  prompts: ParsedPrompt[]
}

// 文件名 → 分类名（去扩展名 + 去括号说明 + 去尾部符号）
function categoryFromFilename(filename: string): string {
  const base = filename.replace(/\.(md|txt)$/, '')
  return base
    .replace(/[（(].*$/, '')
    .replace(/[-_\s]+$/, '')
    .trim()
}

export function parseFile(filename: string, content: string): ParsedFile {
  const category = categoryFromFilename(filename) || '未分类'

  if (filename.includes('长镜头')) {
    return {
      category,
      prompts: [{ title: '长镜头提示词', content: content.trim(), tags: ['长镜头'] }],
    }
  }

  // 含图片链接的自由文本（如场景&人物模板）：按 ![Image](url) 分段
  if (/!\[.*?\]\(https?:\/\//.test(content)) {
    return { category, prompts: parseImageBlocks(content) }
  }

  // 13 个分类规整文件：按 ## 案例 分段
  if (content.includes('## 案例')) {
    return { category, prompts: parseCaseBlocks(content) }
  }

  // 其他自由文本：整体一条
  return { category, prompts: [{ title: category, content: content.trim(), tags: [] }] }
}

// 规整案例：## 案例 N：标题 + 表格 + ### 提示词 + ```代码块```
function parseCaseBlocks(content: string): ParsedPrompt[] {
  const result: ParsedPrompt[] = []
  const blocks = content.split(/^## 案例/m)
  for (const block of blocks) {
    if (!block.trim()) continue
    let title = ''
    const titleLine = block.match(/^\s*\d*[：:]\s*(.+)/)
    if (titleLine) title = titleLine[1].trim()
    const tableTitle = block.match(/\|\s*标题\s*\|\s*(.+?)\s*\|/)
    if (tableTitle) title = tableTitle[1].trim()
    const codeMatch = block.match(/###\s*提示词\s*```[\s\S]*?```([\s\S]*?)```/)
    const codeMatch2 = block.match(/###\s*提示词[\s\S]*?```([\s\S]*?)```/)
    const code = (codeMatch2 ? codeMatch2[1] : codeMatch ? codeMatch[1] : '').trim()
    if (!title || !code) continue
    const tags: string[] = []
    const styleMatch = block.match(/\|\s*风格\s*\|\s*(.+?)\s*\|/)
    const sceneMatch = block.match(/\|\s*场景\s*\|\s*(.+?)\s*\|/)
    if (styleMatch)
      tags.push(...styleMatch[1].split(/[、,，]/).map((s) => s.trim()).filter(Boolean))
    if (sceneMatch)
      tags.push(...sceneMatch[1].split(/[、,，]/).map((s) => s.trim()).filter(Boolean))
    result.push({ title, content: code, tags })
  }
  return result
}

// 自由文本 + 图片：按 ![...](url) 分段，每段一个提示词 + 图
function parseImageBlocks(content: string): ParsedPrompt[] {
  const result: ParsedPrompt[] = []
  const parts = content.split(/!\[.*?\]\((https?:\/\/[^\s)]+)\)/)
  // parts 交替：文本, url, 文本, url, ...
  for (let i = 0; i + 1 < parts.length; i += 2) {
    const text = parts[i].trim()
    const imageUrl = parts[i + 1]
    if (!text) continue
    const lines = text.split('\n').map((l) => l.trim()).filter(Boolean)
    const heading = lines.find((l) => l.startsWith('## '))
    const title = heading
      ? heading.replace(/^##\s*/, '')
      : lines[0].replace(/^#+\s*/, '').slice(0, 30) || '未命名'
    result.push({ title, content: text, tags: [], imageUrl })
  }
  return result
}
