export interface AtlasEntry {
  id: string
  title: string
  dimension: string
  tags: string[]
  image: string
  thumbnail: string
  score: number
  status: 'confirmed' | 'pending_review'
  file: string
  categoryIds: string[]
}

export interface AtlasCategory {
  id: string
  name: string
  entryIds: string[]
  createdAt: string
}
