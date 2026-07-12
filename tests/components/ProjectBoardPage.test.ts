import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { STAGE_DEFINITIONS, type Project } from '@/domain/production'
import ProjectBoardPage from '@/components/projects/ProjectBoardPage.vue'
import { useProjectsStore } from '@/stores/projects'

vi.mock('@/lib/productionIpc', () => ({
  archiveProject: vi.fn(),
  createProject: vi.fn(),
  deleteProject: vi.fn(),
  listProjects: vi.fn().mockResolvedValue([]),
  saveProjectWithStages: vi.fn(),
  setProjectStage: vi.fn(),
  updateProject: vi.fn(),
}))

describe('ProjectBoardPage', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('renders eight fixed stage columns and places each project once', async () => {
    const store = useProjectsStore()
    store.hydrate([project()])
    const wrapper = mount(ProjectBoardPage)

    expect(wrapper.findAll('[data-stage-column]')).toHaveLength(8)
    expect(wrapper.get('[data-stage-column="middle_cut"]').text()).toContain('中版')
    expect(wrapper.findAll('[data-project-id="p1"]')).toHaveLength(1)
    expect(wrapper.get('[data-project-id="p1"]').text()).toContain('65%')
    expect(wrapper.get('[data-stage-column="storyboard"] header').attributes('style')).toContain(
      '#F4C430',
    )

    await wrapper.get('[data-action="edit-selected-project"]').trigger('click')
    expect(store.projectEditorOpen).toBe(true)
    expect(store.editorProjectId).toBe('p1')
  })
})

function project(): Project {
  return {
    id: 'p1',
    code: 'L36',
    version: 'v1',
    name: '三丽鸥短片',
    filePath: 'C:\\work\\L36',
    fileExists: true,
    releaseDate: '2026-07-31',
    mainStageKey: 'storyboard',
    archived: false,
    createdAt: '2026-07-11T08:00:00Z',
    updatedAt: '2026-07-11T08:00:00Z',
    stages: STAGE_DEFINITIONS.map((stage, position) => ({
      id: stage.key,
      stageKey: stage.key,
      position,
      startDate: `2026-07-${String(position + 1).padStart(2, '0')}`,
      endDate: `2026-07-${String(position + 8).padStart(2, '0')}`,
      progress: stage.key === 'storyboard' ? 65 : 0,
      updatedAt: '2026-07-11T08:00:00Z',
    })),
  }
}
