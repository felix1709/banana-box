export const STAGE_DEFINITIONS = [
  { key: 'storyboard', label: '分镜', color: '#F4C430', textColor: '#17212B', contrastRatio: 9.92 },
  { key: 'first_cut', label: '初版', color: '#1D4ED8', textColor: '#FFFFFF', contrastRatio: 6.7 },
  { key: 'refinement', label: '精修', color: '#0F766E', textColor: '#FFFFFF', contrastRatio: 5.47 },
  { key: 'middle_cut', label: '中版', color: '#15803D', textColor: '#FFFFFF', contrastRatio: 5.02 },
  { key: 'effects', label: '特效', color: '#C2410C', textColor: '#FFFFFF', contrastRatio: 5.18 },
  { key: 'art_titles', label: '美术字', color: '#BE123C', textColor: '#FFFFFF', contrastRatio: 6.29 },
  { key: 'music', label: '音乐', color: '#6D28D9', textColor: '#FFFFFF', contrastRatio: 7.1 },
  {
    key: 'final_composite',
    label: '合成终版',
    color: '#334155',
    textColor: '#FFFFFF',
    contrastRatio: 10.35,
  },
] as const

export type StageKey = (typeof STAGE_DEFINITIONS)[number]['key']
export type StageStatus = 'not_started' | 'in_progress' | 'completed'

export function stageStatus(progress: number): StageStatus {
  if (progress === 0) return 'not_started'
  if (progress === 100) return 'completed'
  return 'in_progress'
}

export interface ProjectStage {
  id: string
  stageKey: StageKey
  position: number
  startDate: string
  endDate: string
  progress: number
  updatedAt: string
}

export interface Project {
  id: string
  code: string
  version: string
  name: string
  filePath: string
  fileExists: boolean
  releaseDate: string
  mainStageKey: StageKey
  archived: boolean
  ownerUserId: string
  isPublic: boolean
  lastActivitySummary: string
  lastActivityActorName: string
  createdAt: string
  updatedAt: string
  stages: ProjectStage[]
}

export interface ProjectStageInput {
  stageKey: StageKey
  startDate: string
  endDate: string
  progress: number
}

export interface CreateProjectInput {
  code: string
  version: string
  name: string
  filePath: string
  releaseDate: string
  ownerUserId?: string
  stages: ProjectStageInput[]
}

export interface UpdateProjectInput extends Omit<CreateProjectInput, 'stages'> {
  projectId: string
}

export interface SetProjectStageInput extends ProjectStageInput {
  projectId: string
  actorUserId?: string
  actorName?: string
}

export interface SaveProjectWithStagesInput extends Omit<CreateProjectInput, 'stages'> {
  projectId: string
  archived: boolean
  stages: ProjectStageInput[]
}

export interface ProjectFilter {
  query: string
  stageKey: StageKey | 'all'
  releaseDate: string
  archived: boolean | 'all'
}

export interface DailyTask {
  id: string
  title: string
  progress: number
  note: string
  investedMinutes: number
  reminderTime: string
  reminderContent: string
  position: number
  sourceTaskId: string | null
  sourceSnapshotHash: string | null
  createdAt: string
  updatedAt: string
}

export interface CreateDailyTaskInput {
  localDate: string
  code: string
  projectId: string | null
  title: string
  progress: number
  note: string
  investedMinutes: number
  reminderTime?: string
  reminderContent?: string
}

export interface UpdateDailyTaskInput {
  taskId: string
  title: string
  progress: number
  note: string
  investedMinutes: number
  reminderTime: string
  reminderContent: string
}

export interface DailyTaskGroup {
  id: string
  code: string
  projectId: string | null
  position: number
  tasks: DailyTask[]
}

export interface DailyTaskDay {
  id: string
  localDate: string
  settledAt: string | null
  reportSnapshot: string | null
  groups: DailyTaskGroup[]
}

export interface DailyReportResult {
  text: string
  taskCount: number
}

export type CarryConflictResolution = 'keep_target' | 'overwrite_target'

export interface CarrySelection {
  sourceTaskId: string
  carry: boolean
  resolution: CarryConflictResolution | null
}

export interface CarryConflict {
  sourceTaskId: string
  targetTaskId: string
  targetDate: string
}

export interface SettlementResult {
  settled: boolean
  reportSnapshot: string
  settledAt: string | null
  conflicts: CarryConflict[]
  day: DailyTaskDay
}
