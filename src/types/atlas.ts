export interface AtlasEntry {
  id: string
  title: string
  dimension: string
  tags: string[]
  image: string
  score: number
  status: 'confirmed' | 'pending_review'
  file: string
}
