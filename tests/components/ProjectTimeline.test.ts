import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it } from 'vitest'
import { STAGE_DEFINITIONS, type Project } from '@/domain/production'
import ProjectTimeline from '@/components/projects/ProjectTimeline.vue'

describe('ProjectTimeline', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('renders overlapping stage bars with separate progress markers', () => {
    const wrapper = mount(ProjectTimeline, { props: { project: project() } })

    expect(wrapper.find('[data-stage-bar="storyboard"]').exists()).toBe(true)
    expect(wrapper.find('[data-stage-bar="first_cut"]').exists()).toBe(true)
    expect(wrapper.find('[data-today-line]').exists()).toBe(true)
    const markers = wrapper.findAll('[data-progress-marker]')
    expect(markers).toHaveLength(8)
    expect(wrapper.findAll('[data-field="project-stage-progress"]')).toHaveLength(8)
    expect(markers[0].attributes('style')).not.toBe(markers[1].attributes('style'))
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
    ownerUserId: 'user-1',
    isPublic: true,
    lastActivitySummary: '',
    lastActivityActorName: '',
    createdAt: '2026-07-11T08:00:00Z',
    updatedAt: '2026-07-11T08:00:00Z',
    stages: STAGE_DEFINITIONS.map((stage, position) => ({
      id: stage.key,
      stageKey: stage.key,
      position,
      startDate: position === 0 ? '2026-07-01' : position === 1 ? '2026-07-05' : '2026-07-10',
      endDate: position === 0 ? '2026-07-10' : position === 1 ? '2026-07-14' : '2026-07-16',
      progress: position === 0 ? 80 : position === 1 ? 30 : 0,
      updatedAt: '2026-07-11T08:00:00Z',
    })),
  }
}
