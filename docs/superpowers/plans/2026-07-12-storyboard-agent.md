# 故事板 Agent 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (- [ ]) syntax for tracking.

**Goal:** 将静态故事板表单升级为使用独立 Provider、每会话 Skill、SQLite 持久化和 OpenAI 兼容 SSE 的 Agent 对话工作台。

**Architecture:** Rust 的 storyboard 模块拥有模型、仓储、请求快照、SSE 解码、取消令牌和 Tauri 命令；前端通过 Pinia 管理会话并订阅事件。Provider 的 API Key 始终留在系统凭据，Markdown 在前端以禁用 HTML 的渲染器展示。现有 storyboard 表作为基础，新增每会话 Skill 关系和 Provider 生成配置。

**Tech Stack:** Rust、Tauri 2、rusqlite、reqwest SSE、Vue 3、Pinia、Vitest、Vue Test Utils、markdown-it。

---

## 文件结构

- Create: src-tauri/migrations/0003_storyboard_agent.sql
- Create: src-tauri/src/storyboard/model.rs
- Create: src-tauri/src/storyboard/repository.rs
- Create: src-tauri/src/storyboard/service.rs
- Create: src-tauri/src/storyboard/mod.rs
- Create: src-tauri/src/storyboard/tests.rs
- Create: src/domain/storyboard.ts
- Create: src/lib/storyboardIpc.ts
- Create: src/stores/storyboard.ts
- Create: src/components/storyboard/ConversationList.vue
- Create: src/components/storyboard/SkillLibraryPanel.vue
- Create: src/components/storyboard/StoryboardChatMessage.vue
- Create: src/components/storyboard/StoryboardComposer.vue
- Create: src/components/storyboard/SafeMarkdown.vue
- Modify: src-tauri/src/providers.rs
- Modify: src-tauri/src/commands/provider_commands.rs
- Modify: src-tauri/src/lib.rs
- Modify: src-tauri/src/migration.rs
- Modify: src-tauri/src/startup.rs
- Modify: src/types/providers.ts
- Modify: src/lib/provider-ipc.ts
- Modify: src/stores/providers.ts
- Modify: src/components/SettingsModal.vue
- Modify: src/components/storyboard/StoryboardPage.vue
- Modify: package.json
- Modify: pnpm-lock.yaml
- Modify: tests/stores/providers.test.ts
- Modify: tests/components/SettingsModal.test.ts
- Modify: tests/components/StoryboardPage.test.ts

### Task 1：扩展独立故事板 Provider 配置

**Files:**
- Create: src-tauri/migrations/0003_storyboard_agent.sql
- Modify: src-tauri/src/migration.rs
- Modify: src-tauri/src/providers.rs
- Modify: src/types/providers.ts
- Modify: src/lib/provider-ipc.ts
- Modify: src/stores/providers.ts
- Modify: tests/stores/providers.test.ts

- [ ] **Step 1: 为 Provider 生成配置写失败测试**

在 providers.rs 的保存测试中构造 Storyboard SaveProviderInput，断言 temperature 和 contextWindowTokens 在保存、读取、不同 Provider 切换时保持独立。前端测试中断言公开 Provider 不含 apiKey 或 credentialRef，但包含：

~~~ts
temperature: 0.7,
contextWindowTokens: 16000,
~~~

增加越界值断言：temperature 为 2.1 返回 PROVIDER_TEMPERATURE_INVALID；contextWindowTokens 为 511 返回 PROVIDER_CONTEXT_WINDOW_INVALID。

- [ ] **Step 2: 添加数据库迁移**

0003_storyboard_agent.sql 必须包含：

~~~sql
ALTER TABLE ai_providers
  ADD COLUMN temperature REAL NOT NULL DEFAULT 0.7
  CHECK (temperature >= 0.0 AND temperature <= 2.0);

ALTER TABLE ai_providers
  ADD COLUMN context_window_tokens INTEGER NOT NULL DEFAULT 16000
  CHECK (context_window_tokens BETWEEN 512 AND 128000);

CREATE TABLE storyboard_thread_skills (
  thread_id TEXT NOT NULL REFERENCES storyboard_threads(id) ON DELETE CASCADE,
  skill_id TEXT NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
  skill_version_id TEXT NOT NULL REFERENCES skill_versions(id) ON DELETE RESTRICT,
  enabled_at TEXT NOT NULL,
  PRIMARY KEY (thread_id, skill_id)
);
CREATE INDEX idx_storyboard_thread_skills_thread
  ON storyboard_thread_skills(thread_id, enabled_at);
~~~

迁移注册必须让已是 v2 的数据库升级到 v3，并保留 v1 到 v2 逻辑。更新 migration.rs 与 startup.rs 的 schema 版本断言。

- [ ] **Step 3: 扩展 Provider DTO、输入验证与 IPC**

AiProvider、RawProvider、SaveProviderInput 和 SQL SELECT/UPDATE 都增加 temperature 与 context_window_tokens。仅 Storyboard Provider 可接收这两个配置；ReverseImage 读取默认值但设置界面不显示它们。校验函数：

~~~rust
fn validate_storyboard_generation_config(
    kind: ProviderKind,
    temperature: Option<f64>,
    context_window_tokens: Option<i64>,
) -> Result<(f64, i64), String>
~~~

Storyboard 缺省为 0.7 和 16000。TypeScript 使用 temperature 与 contextWindowTokens 的 camelCase 字段。

- [ ] **Step 4: 运行 GREEN**

~~~powershell
$env:CARGO_TARGET_DIR = (Join-Path (Get-Location) 'tmp\cargo-storyboard')
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml providers migration
pnpm test -- tests/stores/providers.test.ts
~~~

Expected: Provider 相关 Rust 测试与 Pinia 测试通过。

- [ ] **Step 5: 提交**

~~~powershell
git add src-tauri/migrations/0003_storyboard_agent.sql src-tauri/src/migration.rs src-tauri/src/startup.rs src-tauri/src/providers.rs src/types/providers.ts src/lib/provider-ipc.ts src/stores/providers.ts tests/stores/providers.test.ts
git commit -m "feat: configure storyboard provider defaults"
~~~

### Task 2：实现故事板本地模型与仓储

**Files:**
- Create: src-tauri/src/storyboard/model.rs
- Create: src-tauri/src/storyboard/repository.rs
- Create: src-tauri/assets/skills/storyboard-prompt-optimizer.md
- Create: src-tauri/src/storyboard/tests.rs
- Create: src-tauri/src/storyboard/mod.rs
- Modify: src-tauri/src/lib.rs

- [ ] **Step 1: 编写仓储失败测试**

测试创建线程后默认拥有 builtin storyboard-prompt-optimizer，启用状态仅属于该线程；创建第二线程不会读取第一线程的额外 Skill。测试写入 user_text、assistant_markdown 和 streaming 请求后，重新读取仍按 position 排序。测试删除线程级联删除消息、请求和 Skill 映射。

- [ ] **Step 2: 打包并初始化内置 Skill**

将 C:\\Users\\Felix\\.codex\\skills\\storyboard-prompt-optimizer\\SKILL.md 的已确认内容复制为 src-tauri/assets/skills/storyboard-prompt-optimizer.md。新建 ensure_builtin_skill(db)；它使用 include_str!("../../assets/skills/storyboard-prompt-optimizer.md")，在单个立即事务中创建 source=builtin 的 Skill、版本 1 和当前版本引用。重复启动不能创建第二个 Skill 或版本。

为该函数添加测试：首次初始化有一个默认 Skill；二次初始化后 Skill 与版本总数不变；内容哈希不同的本地 Skill 可以创建新版本，但 builtin Skill 不能通过用户输入覆盖。

- [ ] **Step 3: 定义 Rust DTO 与验证边界**

model.rs 定义并序列化 ThreadDto、MessageDto、SkillDto、CreateThreadInput、CreateSkillInput、SendMessageInput、SetThreadSkillsInput 与 ExportThreadResult。最大限制固定为：线程标题 120 字符、用户消息 32 KiB、Skill 名称 80 字符、Skill 内容 128 KiB、每线程最多 12 个启用 Skill。每次上下文按最新消息向前裁剪，保留的 UTF-8 字节数不超过 contextWindowTokens 乘以 4；请求体同时携带 max_tokens 为 contextWindowTokens。

- [ ] **Step 4: 实现事务性仓储**

repository.rs 提供：

~~~rust
pub fn create_thread(db: &Database, input: CreateThreadInput) -> Result<ThreadDto, String>;
pub fn list_threads(db: &Database) -> Result<Vec<ThreadDto>, String>;
pub fn load_thread(db: &Database, id: &str) -> Result<ThreadDetailDto, String>;
pub fn delete_thread(db: &Database, id: &str) -> Result<(), String>;
pub fn clear_thread(db: &Database, id: &str) -> Result<ThreadDetailDto, String>;
pub fn set_thread_model(db: &Database, thread_id: &str, model: &str) -> Result<ThreadDto, String>;
pub fn set_thread_skills(db: &Database, thread_id: &str, skill_ids: Vec<String>) -> Result<ThreadDetailDto, String>;
pub fn list_skills(db: &Database) -> Result<Vec<SkillDto>, String>;
pub fn create_local_skill(db: &Database, input: CreateSkillInput) -> Result<SkillDto, String>;
pub fn export_thread_markdown(db: &Database, thread_id: &str) -> Result<String, String>;
~~~

新线程从数据库的 builtin Skill 版本创建映射。create_local_skill 对同一 Skill 内容哈希去重，并在内容变化时建立新版本。每次发起请求前把启用 Skill 的 id、版本、内容和 Provider 配置写入 agent_requests.snapshot_json。

- [ ] **Step 5: 运行 GREEN**

~~~powershell
$env:CARGO_TARGET_DIR = (Join-Path (Get-Location) 'tmp\cargo-storyboard')
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml storyboard
~~~

Expected: 故事板仓储测试全部通过。

### Task 3：实现 SSE 请求、取消与 Tauri 事件

**Files:**
- Create: src-tauri/src/storyboard/service.rs
- Modify: src-tauri/src/storyboard/mod.rs
- Modify: src-tauri/src/commands/provider_commands.rs
- Modify: src-tauri/src/lib.rs
- Modify: src-tauri/src/app_state.rs

- [ ] **Step 1: 为 SSE 解码和失败状态写测试**

在 storyboard/tests.rs 使用分块的 SSE 样本验证 data 行跨网络块到达时仍能解出 choices[0].delta.content；data: [DONE] 完成请求；无 content 的 role chunk 被忽略；非 JSON data 返回 STORYBOARD_STREAM_INVALID。测试取消令牌将请求状态写成 cancelled，网络错误写成 failed，已写入的 user 消息不被删除。

- [ ] **Step 2: 建立不可变请求与运行时控制**

service.rs 创建 StoryboardRuntime，内部以 Mutex HashMap 保存 requestId 对应 CancellationToken。启动请求必须先检查 ProviderKind::Storyboard、凭据、模型属于当前 Provider 的 availableModels、线程没有 streaming 请求；随后持久化 user 与 streaming assistant 消息、request snapshot 与 active request。

请求体固定使用：

~~~json
{
  "model": "选择的模型",
  "stream": true,
  "temperature": 0.7,
  "messages": [
    { "role": "system", "content": "已启用 Skill 拼接内容" },
    { "role": "user", "content": "用户输入" }
  ]
}
~~~

系统内容来自该请求 snapshot，不能在流式过程中重新读取 Skill。

- [ ] **Step 3: 添加命令与前端事件协议**

新增 start_storyboard_request、cancel_storyboard_request、list_storyboard_threads、create_storyboard_thread、load_storyboard_thread、delete_storyboard_thread、clear_storyboard_thread、set_storyboard_thread_model、set_storyboard_thread_skills、list_storyboard_skills、create_storyboard_skill、export_storyboard_thread。start 命令立即返回 requestId，后台任务向 main 窗口发送 storyboard-stream 事件：

~~~rust
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct StoryboardStreamEvent {
    request_id: String,
    thread_id: String,
    message_id: String,
    kind: StoryboardStreamKind,
    delta: String,
    error_code: Option<String>,
}
~~~

kind 只允许 delta、completed、failed、cancelled。每次 delta 追加数据库后再 emit。Runtime 在 completed、failed 或 cancelled 后删除令牌。

- [ ] **Step 4: 注册与验证**

在 lib.rs 注册 storyboard 模块、manage StoryboardRuntime 和所有命令。运行：

~~~powershell
$env:CARGO_TARGET_DIR = (Join-Path (Get-Location) 'tmp\cargo-storyboard')
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml storyboard provider_http
~~~

Expected: SSE、取消、数据库状态与既有 HTTP 流测试都通过。

### Task 4：实现设置页的 Provider 切换

**Files:**
- Modify: src/components/SettingsModal.vue
- Modify: tests/components/SettingsModal.test.ts

- [ ] **Step 1: 添加失败测试**

模拟 reverse-image 与 storyboard Provider。断言 API 页的 Provider 选择器切换后展示各自的 URL、模型和连接状态；Storyboards 显示温度与上下文长度，ReverseImage 不显示；保存 Storyboards 不改 reverse-image 输入。

- [ ] **Step 2: 重构设置页面状态**

将 reverseModel、availableReverseModels 和只针对反推的函数重命名为 selectedProviderId、selectedProvider、availableModels 与通用 loadProvider、applyProvider、saveProvider、checkProviderConnection。Provider 下拉项目固定为反推图片和故事板 Agent。保存 Storyboard 时带入 temperature 与 contextWindowTokens。

模型下拉优先选择 defaultModel；如果模型列表包含 glm-5.2 且没有有效默认模型，选择 glm-5.2；否则选择第一个模型。

- [ ] **Step 3: 运行 GREEN**

~~~powershell
pnpm test -- tests/components/SettingsModal.test.ts tests/stores/providers.test.ts
pnpm typecheck
~~~

Expected: 设置与 Provider 前端测试通过。

### Task 5：实现前端会话 Store、IPC 与安全 Markdown

**Files:**
- Create: src/domain/storyboard.ts
- Create: src/lib/storyboardIpc.ts
- Create: src/stores/storyboard.ts
- Create: src/components/storyboard/SafeMarkdown.vue
- Modify: package.json
- Modify: pnpm-lock.yaml

- [ ] **Step 1: 添加依赖和失败测试**

安装 markdown-it 与 @types/markdown-it。创建 SafeMarkdown 测试，输入普通标题、列表和 fenced code block，断言生成对应只读元素；输入 raw script、javascript 链接和 img 标签，断言结果中没有 script、img 或 javascript href。

- [ ] **Step 2: 实现安全渲染与 Store**

SafeMarkdown 使用 MarkdownIt({ html: false, linkify: false, breaks: true })，并覆写 validateLink 仅允许 https、http、mailto。代码块必须用 overflow-x:auto 容器包裹。

Storyboard Pinia store 保存 threads、skills、activeThreadId、activeThread、loading、sending、lastError。它在 load 中读取历史、Skill 库和 ProviderKind storyboard；startSend 调用 IPC 后监听 storyboard-stream，按 requestId 向 message.contentMarkdown 追加 delta。failed 或 cancelled 更新消息状态并结束 sending；createSkill 创建后刷新 Skill 库；dispose 解绑事件。

- [ ] **Step 3: 运行 GREEN**

~~~powershell
pnpm test -- tests/components/storyboard/SafeMarkdown.test.ts tests/stores/storyboard.test.ts
~~~

Expected: Markdown 安全测试与流事件 Store 测试通过。

### Task 6：重建故事板 Agent 页面

**Files:**
- Modify: src/components/storyboard/StoryboardPage.vue
- Create: src/components/storyboard/ConversationList.vue
- Create: src/components/storyboard/SkillLibraryPanel.vue
- Create: src/components/storyboard/StoryboardChatMessage.vue
- Create: src/components/storyboard/StoryboardComposer.vue
- Modify: tests/components/StoryboardPage.test.ts

- [ ] **Step 1: 编写失败组件测试**

替换静态表单测试，断言页面存在 data-storyboard-conversations、data-storyboard-skills、data-storyboard-messages、data-storyboard-model、data-action=new-storyboard-thread、data-action=send-storyboard-message 和 data-action=export-storyboard-thread。模拟 Store 验证新建、选择、删除、清空、模型切换、Skill 开关、发送、复制和导出都调用正确 action。

- [ ] **Step 2: 建立两栏页面和对话组件**

ConversationList 提供新建图标按钮、会话行和删除图标；SkillLibraryPanel 提供新增 Skill 入口、当前会话 toggle；StoryboardChatMessage 使用 SafeMarkdown，用户消息右对齐深色，Agent 消息左对齐浅色，每条都有复制图标；StoryboardComposer 是多行 textarea 与发送图标按钮。

StoryboardPage 加载时调用 storyboard.load，卸载时调用 storyboard.dispose。顶部模型 select 读取 providers.byId('storyboard')?.availableModels；选择后调用 setThreadModel。清空与导出采用图标按钮和中文 tooltip。发送期间禁用重复发送并显示取消图标。

- [ ] **Step 3: 实现交互式首条引导**

当活动线程没有消息时，渲染 Agent 选择项“故事类型”“视觉方向”“节奏”。点击“其他”显示内嵌输入框，提交后调用同一 send。默认启用的 storyboard-prompt-optimizer 在标题区以可关闭标签显示，点击快捷标签将名称插入 composer。

- [ ] **Step 4: 运行 GREEN**

~~~powershell
pnpm test -- tests/components/StoryboardPage.test.ts tests/components/storyboard
pnpm typecheck
pnpm lint
~~~

Expected: 组件、类型检查和 lint 全部通过。

### Task 7：端到端验证与提交

**Files:**
- Verify only: 设置、故事板页面、Rust 命令和 Provider 配置

- [ ] **Step 1: 全量自动化验证**

~~~powershell
pnpm test
$env:CARGO_TARGET_DIR = (Join-Path (Get-Location) 'tmp\cargo-storyboard')
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml
~~~

Expected: 前端与 Rust 测试全通过。

- [ ] **Step 2: 调试验收**

启动 pnpm tauri dev --config src-tauri/tauri.dev-1423.conf.json。先在设置页保存故事板 Provider 并检测模型，再创建会话，确认 glm-5.2 默认选择。发送一条消息，确认流式气泡逐步出现；切换模型只影响下一条；关闭并重开应用确认会话恢复。测试无 API Key 与网络失败，确认错误在聊天流中可见且可重试。

- [ ] **Step 3: 提交**

~~~powershell
git add src-tauri src/components/storyboard src/components/SettingsModal.vue src/domain/storyboard.ts src/lib/storyboardIpc.ts src/stores/storyboard.ts src/types/providers.ts src/lib/provider-ipc.ts src/stores/providers.ts tests/components/SettingsModal.test.ts tests/components/StoryboardPage.test.ts tests/components/storyboard tests/stores/storyboard.test.ts package.json pnpm-lock.yaml
git commit -m "feat: add storyboard agent workspace"
~~~
