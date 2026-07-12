# 提示词库新增按钮实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (- [ ]) syntax for tracking.

**Goal:** 将提示词库新增按钮替换为紧凑、低权重、可访问的 Lucide Plus 图标按钮。

**Architecture:** 只修改现有 AppSidebar 组件和其组件测试。按钮继续调用 ui.openEditor(null)，不改编辑器、库数据或导航状态。

**Tech Stack:** Vue 3、TypeScript、Lucide Vue、Vitest、Vue Test Utils。

---

## 文件结构

- Modify: src/components/AppSidebar.vue
- Modify: tests/components/AppSidebar.test.ts

### Task 1：为紧凑图标按钮编写失败测试

**Files:**
- Modify: tests/components/AppSidebar.test.ts

- [ ] **Step 1: 更新创建提示词测试**

把“red plus”测试改为断言：

~~~ts
expect(createButton.attributes('title')).toBe('新增提示词')
expect(createButton.attributes('aria-label')).toBe('新增提示词')
expect(createButton.find('svg.lucide-plus').exists()).toBe(true)
expect(createButton.text()).toBe('')
~~~

保留点击后 ui.editorOpen 为 true 和 editingPromptId 为 null 的断言。

- [ ] **Step 2: 运行 RED**

~~~powershell
pnpm test -- tests/components/AppSidebar.test.ts
~~~

Expected: 因当前按钮仍显示文字加号且中文语义未更新而失败。

### Task 2：实现方案 1

**Files:**
- Modify: src/components/AppSidebar.vue

- [ ] **Step 1: 替换文字图标**

从 @lucide/vue 导入 Plus，并把按钮内容替换为：

~~~html
<Plus :size="14" aria-hidden="true" />
~~~

将 title 和 aria-label 设为“新增提示词”。

- [ ] **Step 2: 实现紧凑样式**

create-prompt-button 固定为 width、min-height、flex-basis 都为 28px；使用透明背景、低饱和描边和无 box-shadow。移除渐变、20px 字号和发光阴影。hover 与 focus-visible 只改变背景与描边，不能改变 width、height 或 flex-basis。

- [ ] **Step 3: 运行 GREEN**

~~~powershell
pnpm test -- tests/components/AppSidebar.test.ts
pnpm typecheck
pnpm lint
~~~

Expected: 所有命令退出码为 0。

- [ ] **Step 4: 提交**

~~~powershell
git add src/components/AppSidebar.vue tests/components/AppSidebar.test.ts
git commit -m "style: compact prompt add action"
~~~
