# 当日任务紧凑交互实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (- [ ]) syntax for tracking.

**Goal:** 将当日任务改造成紧凑卡片与可拖拽进度条，并修复日报复制图标的状态恢复。

**Architecture:** 只改 Vue 页面与 Vitest 组件测试；既有 Pinia、IPC 和 SQLite 数据结构保持不变。进度滑块只更新本地对象，保存按钮继续一次性写入现有 store，日报复制使用可清理的 1.2 秒定时器。

**Tech Stack:** Vue 3、TypeScript、Pinia、Lucide Vue、Vitest、Vue Test Utils。

---

## 文件结构

- Modify: src/components/daily/DailyTasksPage.vue
- Modify: tests/components/DailyTasksPage.test.ts

### Task 1：编写失败的页面交互测试

**Files:**
- Modify: tests/components/DailyTasksPage.test.ts

- [ ] **Step 1: 给 range 进度、卡片动作与复制复原添加断言**

在 describe 中增加 afterEach 恢复 timer。在任务编辑测试中把进度输入的断言改为：

~~~ts
const progress = wrapper.get('[data-task-id="t1"] [data-field="task-progress"]')
expect(progress.attributes('type')).toBe('range')

await progress.setValue('80')
expect(wrapper.get('[data-task-id="t1"] [data-progress-value]').text()).toBe('80%')
expect(wrapper.get('[data-task-id="t1"] .task-card-actions').exists()).toBe(true)
~~~

增加日报复制状态测试：

~~~ts
it('restores the report copy icon after 1.2 seconds', async () => {
  vi.useFakeTimers()
  const store = useDailyTasksStore()
  vi.spyOn(store, 'selectDate').mockResolvedValue()
  getDailyReport.mockResolvedValue({ text: '@日报', taskCount: 0 })
  copyToClipboard.mockResolvedValue(undefined)

  const wrapper = mount(DailyTasksPage)
  await wrapper.get('[data-action="copy-daily-report"]').trigger('click')
  expect(wrapper.get('[data-action="copy-daily-report"]').attributes('data-copy-state')).toBe('copied')

  await vi.advanceTimersByTimeAsync(1200)
  expect(wrapper.get('[data-action="copy-daily-report"]').attributes('data-copy-state')).toBe('ready')
})
~~~

增加失败测试：让 copyToClipboard reject，断言 data-copy-state 保持 ready 且 ui.toast 为“复制日报失败”。

- [ ] **Step 2: 运行 RED**

~~~powershell
pnpm test -- tests/components/DailyTasksPage.test.ts
~~~

Expected: 新增 range、卡片类名和 data-copy-state 断言失败。

### Task 2：实现紧凑卡片和复制恢复

**Files:**
- Modify: src/components/daily/DailyTasksPage.vue

- [ ] **Step 1: 实现可清理的日报复制反馈**

导入 onBeforeUnmount 和 useUiStore；新增：

~~~ts
const ui = useUiStore()
let reportCopyResetTimer: ReturnType<typeof window.setTimeout> | null = null

function resetReportCopyState() {
  if (reportCopyResetTimer !== null) window.clearTimeout(reportCopyResetTimer)
  reportCopyResetTimer = null
  reportCopied.value = false
}

function showReportCopied() {
  resetReportCopyState()
  reportCopied.value = true
  reportCopyResetTimer = window.setTimeout(resetReportCopyState, 1200)
}
~~~

复制成功调用 showReportCopied。catch 中调用 resetReportCopyState 和 ui.showToast('复制日报失败')。日期 watcher 调用 resetReportCopyState，onBeforeUnmount 调用 resetReportCopyState。复制按钮添加 :data-copy-state，值为 copied 或 ready。

- [ ] **Step 2: 用 range 控件替换所有数字进度输入**

新建区和每张任务卡片使用：

~~~html
<input
  v-model.number="task.progress"
  class="task-progress-range"
  data-field="task-progress"
  type="range"
  min="0"
  max="100"
  :style="{ '--task-progress': task.progress + '%' }"
  :disabled="Boolean(daily.day?.settledAt)"
>
<output data-progress-value>{{ task.progress }}%</output>
<span class="progress-hint">拖动调整完成进度百分比</span>
~~~

新建区使用 draft.progress 的同一结构和 data-field="new-task-progress"。保留任务名称、备注、创建和更新的既有数据字段。

- [ ] **Step 3: 重排为非嵌套的单任务卡片**

将 daily-group 的外框改为无卡片外框的分组标题。每条 daily-task 使用 task-card 类，首行包含编号文本、任务名、进度控件和百分比；备注放入 card 底部。把保存与删除移动为：

~~~html
<div v-if="!daily.day?.settledAt" class="task-card-actions">
  <button data-action="save-task" type="button" title="保存任务" aria-label="保存任务">...</button>
  <button data-action="delete-task" type="button" title="删除任务" aria-label="删除任务">...</button>
</div>
~~~

按钮默认 opacity 为 0.45；.daily-task:hover 与 :focus-within 时 opacity 为 1。按钮必须保留标题和 aria-label。

- [ ] **Step 4: 实现方案 1 样式**

使用 4px 浅色轨道、主色填充、10px 圆形滑块。range 轨道采用 --task-progress 设置线性填充，thumb 不改变元素布局。卡片使用现有小圆角 token、紧凑 10px 内边距与轻量阴影；备注为小字号。窄屏下首行换为两行，不让百分比或图标覆盖标题。

- [ ] **Step 5: 运行 GREEN**

~~~powershell
pnpm test -- tests/components/DailyTasksPage.test.ts
pnpm typecheck
pnpm lint
~~~

Expected: 测试、类型检查和 lint 全部通过。

- [ ] **Step 6: 提交**

~~~powershell
git add src/components/daily/DailyTasksPage.vue tests/components/DailyTasksPage.test.ts
git commit -m "feat: refine daily task progress controls"
~~~

### Task 3：真实页面验收

**Files:**
- Verify only: src/components/daily/DailyTasksPage.vue

- [ ] **Step 1: 启动调试模式**

~~~powershell
pnpm tauri dev --config src-tauri/tauri.dev-1423.conf.json
~~~

- [ ] **Step 2: 验收**

创建一项任务，拖动 0%、45%、100%，确认数字实时更新。保存后切换日期再回来，确认数值持久化。复制日报，确认对勾 1.2 秒后复原；人为断开剪贴板调用时确认图标不变且 Toast 出现。检查已结算日期无可编辑滑块或操作按钮。
