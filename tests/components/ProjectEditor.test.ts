import { createPinia, setActivePinia } from 'pinia'
import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import ProjectEditor from '@/components/projects/ProjectEditor.vue'
import { useProjectsStore } from '@/stores/projects'

const createProject = vi.hoisted(() => vi.fn())

vi.mock('@/lib/productionIpc', () => ({
  archiveProject: vi.fn(),
  createProject,
  deleteProject: vi.fn(),
  listProjects: vi.fn(),
  saveProjectWithStages: vi.fn(),
  setProjectStage: vi.fn(),
  updateProject: vi.fn(),
}))

describe('ProjectEditor', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    createProject.mockReset()
  })

  it('creates all eight scheduled stages without manual main-stage or progress inputs', async () => {
    const store = useProjectsStore()
    store.openEditor(null)
    createProject.mockImplementation(async (input) => ({
      id: 'p1',
      ...input,
      fileExists: true,
      archived: false,
      createdAt: '2026-07-12T00:00:00Z',
      updatedAt: '2026-07-12T00:00:00Z',
      stages: input.stages.map((stage, position) => ({
        id: `${stage.stageKey}-id`,
        position,
        updatedAt: '2026-07-12T00:00:00Z',
        ...stage,
      })),
    }))
    const wrapper = mount(ProjectEditor)

    await wrapper.get('[data-field="code"]').setValue('L36')
    await wrapper.get('[data-field="version"]').setValue('v1')
    await wrapper.get('[data-field="name"]').setValue('三丽鸥短片')
    await wrapper.get('[data-field="file-path"]').setValue('C:\\work\\L36')
    await wrapper.get('[data-field="release-date"]').setValue('2026-07-31')
    await wrapper.get('form').trigger('submit')
    await flushPromises()

    expect(wrapper.find('.project-fields select').exists()).toBe(false)
    expect(wrapper.find('[data-stage-progress]').exists()).toBe(false)
    expect(createProject).toHaveBeenCalledWith(
      expect.objectContaining({
        code: 'L36',
        stages: expect.arrayContaining([
          expect.objectContaining({ stageKey: 'storyboard', progress: 0 }),
          expect.objectContaining({ stageKey: 'final_composite' }),
        ]),
      }),
    )
    expect(createProject.mock.calls[0][0]).not.toHaveProperty('mainStageKey')
    expect(createProject.mock.calls[0][0].stages).toHaveLength(8)
    expect(store.projects).toHaveLength(1)
    expect(store.projectEditorOpen).toBe(false)
  })
})
