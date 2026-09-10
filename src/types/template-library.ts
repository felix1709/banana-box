export interface TemplateCategory {
  id: string
  name: string
  value: string
}

export interface TemplateCase {
  id: number
  title: string
  category: string
  image: string
  tags: string[]
  promptEn: string
  promptZh: string
  sourceLabel: string
  sourceUrl: string
}

export interface TemplateLibrary {
  version: number
  categories: TemplateCategory[]
  cases: TemplateCase[]
}
