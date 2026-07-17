import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { STAGE_DEFINITIONS, type Project } from '@/domain/production'
import ProjectTimeline from '@/components/projects/ProjectTimeline.vue'

describe('ProjectTimeline', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-07-14T08:00:00Z'))
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('renders read-only stage bars without extra progress tick marks', () => {
    const wrapper = mount(ProjectTimeline, { props: { project: project() } })

    expect(wrapper.findAll('[data-stage-row]')).toHaveLength(STAGE_DEFINITIONS.length)
    expect(wrapper.find('[data-stage-bar="storyboard"]').exists()).toBe(true)
    expect(wrapper.find('[data-stage-bar="first_cut"]').exists()).toBe(true)
    expect(wrapper.find('[data-stage-bar="effects"]').exists()).toBe(false)
    expect(wrapper.find('[data-today-line]').exists()).toBe(true)
    expect(wrapper.find('[data-today-line]').text()).toContain('7.14')
    expect(wrapper.get('[data-axis-boundary="start"]').text()).toContain('7月1日')
    expect(wrapper.get('[data-axis-boundary="release"]').text()).toContain('7月31日')
    expect(wrapper.get('[data-axis-boundary="start"]').classes()).toContain('boundary')
    expect(wrapper.get('[data-axis-boundary="release"]').classes()).toContain('boundary')
    expect(wrapper.findAll('[data-progress-marker]')).toHaveLength(0)
    expect(wrapper.findAll('[data-field="project-stage-progress"]')).toHaveLength(0)
    expect(wrapper.text()).toContain('100%')
  })

  it('pins the today line to the last day for completed projects and supports horizontal drag panning', async () => {
    vi.setSystemTime(new Date('2026-08-20T08:00:00Z'))
    const wrapper = mount(ProjectTimeline, { props: { project: project() } })
    const scroll = wrapper.get('[data-timeline-scroll]').element as HTMLElement

    expect(wrapper.find('[data-today-line]').text()).toContain('7.31')
    expect(wrapper.findAll('[data-stage-row]')).toHaveLength(STAGE_DEFINITIONS.length)
    expect(wrapper.findAll('[data-stage-bar]')).toHaveLength(STAGE_DEFINITIONS.length)

    scroll.scrollLeft = 40
    await wrapper.get('[data-timeline-scroll]').trigger('mousedown', { clientX: 200 })
    window.dispatchEvent(new MouseEvent('mousemove', { clientX: 160 }))
    window.dispatchEvent(new MouseEvent('mouseup'))

    expect(scroll.scrollLeft).toBeGreaterThan(40)
  })

  it('lets a collaborator request a stage schedule change with a reason', async () => {
    const wrapper = mount(ProjectTimeline, {
      props: {
        project: project(),
        canRequestScheduleChange: true,
      },
    })

    await wrapper.get('[data-action="open-schedule-request"]').trigger('click')
    await wrapper.get('[data-field="schedule-request-stage"]').setValue('storyboard')
    await wrapper.get('[data-field="schedule-request-start"]').setValue('2026-07-03')
    await wrapper.get('[data-field="schedule-request-end"]').setValue('2026-07-12')
    await wrapper.get('[data-field="schedule-request-reason"]').setValue('分镜素材比预期晚两天到齐')
    await wrapper.get('[data-action="submit-schedule-request"]').trigger('submit')

    expect(wrapper.emitted('request-schedule-change')?.[0]).toEqual([{
      stageKey: 'storyboard',
      requestedStartDate: '2026-07-03',
      requestedEndDate: '2026-07-12',
      reason: '分镜素材比预期晚两天到齐',
    }])
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
      startDate: position === 0 ? '2026-07-01' : position === 1 ? '2026-07-05' : `2026-07-${String(18 + position).padStart(2, '0')}`,
      endDate: position === 0 ? '2026-07-10' : position === 1 ? '2026-07-14' : `2026-07-${String(19 + position).padStart(2, '0')}`,
      progress: position === 0 ? 80 : position === 1 ? 30 : 0,
      updatedAt: '2026-07-11T08:00:00Z',
    })),
  }
}
