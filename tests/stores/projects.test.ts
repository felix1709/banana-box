import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { STAGE_DEFINITIONS, type Project } from '@/domain/production'
import { useProjectsStore } from '@/stores/projects'

vi.mock('@/lib/productionIpc', () => ({
  archiveProject: vi.fn(),
  createProject: vi.fn(),
  deleteProject: vi.fn(),
  listProjects: vi.fn(),
  saveProjectWithStages: vi.fn(),
  setProjectStage: vi.fn(),
  updateProject: vi.fn(),
}))

describe('projects store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('places a project only in its selected main-stage column', () => {
    const store = useProjectsStore()
    store.hydrate([project({ id: 'p1', mainStageKey: 'effects' })])

    expect(store.projectsByStage.effects.map((item) => item.id)).toEqual(['p1'])
    expect(Object.values(store.projectsByStage).flat().filter((item) => item.id === 'p1')).toHaveLength(1)
  })

  it('filters by code or name, stage, release date, and archive state', () => {
    const store = useProjectsStore()
    store.hydrate([
      project({
        id: 'p1',
        code: 'L36',
        name: '三丽鸥',
        releaseDate: '2026-07-31',
        mainStageKey: 'storyboard',
      }),
      project({
        id: 'p2',
        code: 'L50',
        name: '录像带',
        releaseDate: '2026-08-02',
        mainStageKey: 'effects',
        archived: true,
      }),
    ])
    store.filters = {
      query: '三丽',
      stageKey: 'storyboard',
      releaseDate: '2026-07-31',
      archived: false,
    }

    expect(store.filteredProjects.map((item) => item.id)).toEqual(['p1'])
  })
})

function project(overrides: Partial<Project> = {}): Project {
  const base: Project = {
    id: 'p1',
    code: 'L36',
    version: 'v1',
    name: '三丽鸥',
    filePath: 'C:\\work\\L36',
    fileExists: true,
    releaseDate: '2026-07-31',
    mainStageKey: 'storyboard',
    archived: false,
    createdAt: '2026-07-11T08:00:00Z',
    updatedAt: '2026-07-11T08:00:00Z',
    stages: STAGE_DEFINITIONS.map((stage, position) => ({
      id: `${stage.key}-id`,
      stageKey: stage.key,
      position,
      startDate: `2026-07-${String(position + 1).padStart(2, '0')}`,
      endDate: `2026-07-${String(position + 8).padStart(2, '0')}`,
      progress: 0,
      updatedAt: '2026-07-11T08:00:00Z',
    })),
  }
  return Object.assign(base, overrides)
}
