# Banana Box v1 Unified Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 按已确认设计规格，在正式 GitHub 克隆中一次性交付 Banana Box v1.0.0 的香蕉动画、蕉签提醒、项目管理、当日任务/日报和 Storyboard Agent，并完成安全升级、测试、Tauri updater 签名与发布准备。

**Architecture:** 以 foundation 为唯一基础层，先固定 SQLite v1 schema、启动迁移、Provider/凭据和备份安全；再按依赖完成制作管理、桌面交互和 Storyboard；最后在一个 release candidate 上做跨域回归与 updater 签名安装。所有计划共享一个设计规格和一套稳定命名，不允许各功能创建重复数据库、凭据服务、路由或 Tauri 注册入口。

**Tech Stack:** Windows 11, Git/GitHub, Tauri 2, Rust 2021, SQLite/rusqlite, Vue 3, Pinia, TypeScript, Vite, Vitest, pnpm, Cargo, Gstack.

---

## Source Of Truth

- Product/design specification: [`../specs/2026-07-11-banana-box-v1-design.md`](../specs/2026-07-11-banana-box-v1-design.md)
- Foundation and security: [`2026-07-11-banana-box-v1-foundation.md`](./2026-07-11-banana-box-v1-foundation.md)
- Banana and reminder windows: [`2026-07-11-banana-box-v1-desktop-interaction.md`](./2026-07-11-banana-box-v1-desktop-interaction.md)
- Projects, daily tasks, settlement, report: [`2026-07-11-banana-box-v1-production-management.md`](./2026-07-11-banana-box-v1-production-management.md)
- Storyboard Agent and Skill: [`2026-07-11-banana-box-v1-storyboard-agent.md`](./2026-07-11-banana-box-v1-storyboard-agent.md)
- Cross-module QA and release: [`2026-07-11-banana-box-v1-integration-release.md`](./2026-07-11-banana-box-v1-integration-release.md)

If a subplan contradicts the design specification, stop and resolve the document conflict before writing code. Do not silently choose one interpretation.

## Fixed Cross-plan Names

| Area | Contract |
| --- | --- |
| Branch | `codex/v1-major-update` |
| Formal clone | `C:\Users\Felix\Downloads\banana-box-workspace` |
| SQLite | `banana.db`, schema version 1, `src-tauri/migrations/0001_v1.sql` |
| DB service | `src-tauri/src/db/mod.rs::Database` |
| Provider service | `src-tauri/src/providers.rs::ProviderService` |
| Provider transport | one shared `src-tauri/src/provider_http.rs::ProviderHttpClient` |
| Credential service | `src-tauri/src/secrets.rs::CredentialStore` |
| Image service | one shared `src-tauri/src/image_store.rs::ImageStore` |
| Startup | `StartupCoordinator`, `StartupGate`, final `AppServices { db, providers, provider_http, operations, images }` |
| App pages | `prompts`, `reverse-image`, `compression`, `storyboard`, `projects`, `daily-tasks` |
| Windows | `main`, `floatbtn`, `reminder` |
| Main window size | default `1080×720`, minimum `760×560` |
| Custom IPC authorization | main owns business commands; floatbtn/reminder have explicit Rust label allowlists; mismatch = `FORBIDDEN_WINDOW` |
| Project stores | `useProjectsStore`, `useDailyTasksStore` |
| Storyboard store | `useStoryboardStore` |
| Reminder task navigation | `daily_tasks::navigation::navigate_to_daily_tasks`, event `open-daily-tasks` |

### Task 0: Establish The Execution Baseline

**Files:**
- Confirmed design specification in its own immutable documentation commit
- Documentation-only planning bundle: this index plus the five linked v1 implementation plans

- [ ] Before assigning feature work, ensure the six approved plan documents are one dedicated commit and the worktree is clean. The planning coordinator performs this once; a later executor records the existing commit instead of creating a duplicate:

  ```powershell
  Set-Location C:\Users\Felix\Downloads\banana-box-workspace
  if ((git rev-parse --show-toplevel).Trim() -ne 'C:/Users/Felix/Downloads/banana-box-workspace') { throw 'not in the formal clone' }
  if ((git branch --show-current).Trim() -ne 'codex/v1-major-update') { throw 'wrong implementation branch' }
  if ((git remote get-url origin).Trim() -ne 'https://github.com/felix1709/banana-box.git') { throw 'wrong origin' }
  $designFile = 'docs/superpowers/specs/2026-07-11-banana-box-v1-design.md'
  $DESIGN_SHA = (git log -1 --format=%H -- $designFile).Trim()
  if ($DESIGN_SHA -ne '4a7da8a4a8b0156cab8c938756eb9d50be6bdf41') { throw 'confirmed design commit differs' }
  if (git status --porcelain -- $designFile) { throw 'confirmed design file is dirty' }
  $planFiles = @(
    'docs/superpowers/plans/2026-07-11-banana-box-v1-foundation.md',
    'docs/superpowers/plans/2026-07-11-banana-box-v1-production-management.md',
    'docs/superpowers/plans/2026-07-11-banana-box-v1-desktop-interaction.md',
    'docs/superpowers/plans/2026-07-11-banana-box-v1-storyboard-agent.md',
    'docs/superpowers/plans/2026-07-11-banana-box-v1-integration-release.md',
    'docs/superpowers/plans/2026-07-11-banana-box-v1-implementation-index.md'
  )
  $dirtyPlans = @(git status --porcelain -- $planFiles)
  if ($dirtyPlans) {
    git add -- $planFiles
    git diff --cached --check
    git commit -m "docs: add Banana Box v1 implementation plans"
  }
  $PLAN_SHA = (git log -1 --format=%H -- $planFiles).Trim()
  if ($PLAN_SHA -notmatch '^[0-9a-f]{40}$') { throw 'planning bundle is not committed' }
  if (git status --porcelain) { throw 'worktree must be clean before feature execution' }
  ```

  Expected: `DESIGN_SHA` identifies the separately confirmed specification, `PLAN_SHA` identifies the reviewed six-file planning bundle, none is dirty, and later broad `git add docs` cannot accidentally absorb planning edits into feature commits.

- [ ] Read the full design specification and all five linked plans before assigning implementation tasks. Use `superpowers:using-git-worktrees` if multiple implementation agents will edit concurrently; never let two agents modify the same worktree/shared file at once.

- [ ] Verify the formal repository and branch:

  ```powershell
  Set-Location C:\Users\Felix\Downloads\banana-box-workspace
  git rev-parse --show-toplevel
  git branch --show-current
  git remote get-url origin
  git status --short
  git log -1 --oneline
  ```

  Expected: root is the formal clone, branch is `codex/v1-major-update`, origin is `https://github.com/felix1709/banana-box.git`, and the only uncommitted files (if any) are explicitly understood.

- [ ] Execute Integration/Release Task 0 before assigning feature work. Prove `git ls-remote`/fetch separately from GitHub CLI; on this machine Git HTTPS is working but `gh.exe` is absent. Request user approval before installing `GitHub.cli`, complete browser authentication, verify push/admin access, and prove the different-collaborator `production-release` reviewer gate is supported. If any prerequisite fails, resolve it or explicitly revise the confirmed release design now, not after implementation.

- [ ] Prove the v0.2.2 baseline passes before v1 code:

  ```powershell
  pnpm install --frozen-lockfile
  pnpm check
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml
  pnpm build
  ```

  Expected: all commands exit `0`. If baseline fails, capture and fix/approve that separately before attributing it to v1.

- [ ] Record the known v0.2.2 rustfmt-only deviation: `cargo fmt --check` fails on two long assertions in `src-tauri/src/lib.rs`. Foundation Task 1 owns the isolated format-only commit before any behavior/dependency change; do not classify it as a v1 regression.

- [ ] Verify the exact baseline SHA is `299dde2db3274a9c2ed844698795a6d4ed317126` and the confirmed design SHA is `4a7da8a4a8b0156cab8c938756eb9d50be6bdf41`. Create no second v1 feature branch unless using an isolated worktree with a named merge point.

### Task 1: Execute Foundation And Security First

**Plan:** [`2026-07-11-banana-box-v1-foundation.md`](./2026-07-11-banana-box-v1-foundation.md)

- [ ] Use `superpowers:test-driven-development` for every foundation task. Complete the Prompt favorite/order fix, safe legacy import/export, SQLite v1 schema, Provider/credential service, startup gate, two-phase migration/recovery, and backup boundaries.

- [ ] Run every focused red/green command in the plan; do not batch all tests only at the end.

- [ ] At the foundation checkpoint run:

  ```powershell
  pnpm check
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" fmt --manifest-path src-tauri/Cargo.toml -- --check
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml
  ```

  Expected: PASS. The frontend no longer persists or sends plaintext API keys; startup recovery tests cover all six paths, including interrupted fresh initialization.

- [ ] Use `superpowers:requesting-code-review`. Block the next wave on any finding involving data loss, migration atomicity, credential exposure, ZIP traversal, schema mismatch, or normal IPC bypassing recovery mode.

- [ ] Report the verified foundation commit SHA in the task update; do not create a public release/tag.

### Task 2: Execute Production Management Before Reminder Wiring

**Plan:** [`2026-07-11-banana-box-v1-production-management.md`](./2026-07-11-banana-box-v1-production-management.md)

- [ ] Implement projects, eight fixed stages, overlapping dates, timeline, progress-derived state, primary-stage selection, daily groups/tasks, exact Markdown report, settlement, reopening, and idempotent carry-forward.

- [ ] Preserve the fixed stage sequence exactly:

  ```text
  分镜 -> 初版 -> 精修 -> 中版 -> 特效 -> 美术字 -> 音乐 -> 合成终版
  ```

- [ ] Prove exact report output contains all 0–100% tasks, including unfinished L50 at 50%; use string equality, not snapshots that can be casually updated.

- [ ] Finish `daily_tasks::navigation::navigate_to_daily_tasks` and its `open-daily-tasks` event before the reminder action is connected.

- [ ] Run the subplan's full frontend/Rust gate, review all multi-table transactions, and report the verified production checkpoint SHA in the task update.

### Task 3: Execute Desktop Interaction Against The Real Daily-task Navigation

**Plan:** [`2026-07-11-banana-box-v1-desktop-interaction.md`](./2026-07-11-banana-box-v1-desktop-interaction.md)

- [ ] Implement the 12-frame 360 ms banana in a 64×64 transparent window, visible art no larger than 52 px, reversible from the current frame, with persisted/clamped display position.

- [ ] Implement the approved compact B reminder style and its initial/snooze delivery state machine. It is application-runtime only: each workday may create exactly one 18:00 initial delivery, while that delivery's one already-created snooze follows its absolute due instant even when 30 minutes crosses midnight or reaches Saturday; weekends never create a new initial phase.

- [ ] Wire “去结算” only through the completed `daily_tasks::navigation::navigate_to_daily_tasks`; do not fork navigation behavior.

- [ ] Verify panel state consistency across banana, tray, global shortcut, file drop, focus loss, and pinning. Verify reminder auto-show does not steal focus and old fencing tokens cannot ACK/render.

- [ ] Run the subplan's desktop/DPI/accessibility gate, review window/event races, and report the verified desktop checkpoint SHA in the task update.

### Task 4: Execute Storyboard Agent On The Stable Foundation

**Plan:** [`2026-07-11-banana-box-v1-storyboard-agent.md`](./2026-07-11-banana-box-v1-storyboard-agent.md)

- [ ] Implement independent Storyboard Provider configuration and model discovery. Select `glm-5.2` only when the Provider returns that exact model; otherwise require an explicit choice.

- [ ] Bundle only the required `storyboard-prompt-optimizer` Markdown files and Banana manifest. Enforce immutable versions, manual activation/update, strict import limits, protocol version 1, and request snapshot hashes.

- [ ] Implement the eight-state protocol, 1–3 structured questions, 2–3 mutually exclusive preset options, application-owned “其他” input, two confirmations, safe Markdown rendering, and stable raw block copy.

- [ ] Implement one active request per thread, ordered events, cancellation, restart interruption, structured repair once, original-snapshot retry, and current-configuration retry against a local mock Provider only.

- [ ] Run the subplan's security and full workflow gate. Confirm no paid API call, tool execution, attachment, file reading, Shell, or web search exists, and report the verified Storyboard checkpoint SHA in the task update.

### Task 5: Execute Unified Integration And Release Preparation

**Plan:** [`2026-07-11-banana-box-v1-integration-release.md`](./2026-07-11-banana-box-v1-integration-release.md)

- [ ] Reconcile all shared files against the fixed contracts table; never resolve an `App.vue`, `lib.rs`, schema, config, or capability conflict by discarding another feature wholesale.

- [ ] Complete deterministic v0.2.2 upgrade/recovery fixtures, reminder/daily-task navigation tests, production and Storyboard backup round trips, CSP/capability/security tests, and CI gates.

- [ ] Run the complete automated release gate from one commit:

  ```powershell
  pnpm install --frozen-lockfile
  pnpm check
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" fmt --manifest-path src-tauri/Cargo.toml -- --check
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml
  pnpm build
  ```

  Expected: all commands exit `0` with no skipped release-blocking suite.

- [ ] Run Gstack `browse`, `qa`, and `design-review` and finish the acceptance matrix. Fix every P0/P1 and rerun affected flows plus the full gate.

- [ ] Bump all versions to `1.0.0`, prepare changelog/release notes, create the release candidate commit, then build/test signed Windows installers without committing artifacts or secrets.

- [ ] Present release evidence and stop for **Approval A**, which authorizes only branch/PR push, immutable tag creation, and a non-public GitHub CI Draft. After downloading and re-verifying that Draft's exact hashes/install/updater evidence, stop again for independent **Approval B**, which alone authorizes public publication. Publish only those unchanged approved assets, then run Gstack `canary`.

## Execution Strategy

Use one of these only after the user chooses execution:

1. **Subagent-driven development (recommended):** one implementation agent handles one plan task at a time in the main worktree, a second agent reviews the diff/tests, and the coordinator resolves feedback before the next task. Parallelize read-only review or genuinely disjoint isolated worktrees, not shared registration files.
2. **Inline execution:** the primary agent runs every checkbox sequentially with the same red/green/commit gates. This is slower but avoids worktree integration overhead.

For either strategy:

- Apply `superpowers:verification-before-completion` before claiming each milestone or the final release candidate.
- Use `superpowers:receiving-code-review` to verify review feedback technically before applying it.
- Use Gstack only after deterministic tests are green; browser QA complements tests rather than replacing them.
- Stop on ambiguous requirements, unexpected user changes, or conflicts with the approved specification.

## Unified Success Criteria

- [ ] All five user-facing capabilities are present in one v1.0.0 candidate.
- [ ] Every approved design decision has a named implementation task and test/QA evidence.
- [ ] Existing prompt, reverse-image, compression, update, tray, shortcut, drag/drop, and window behavior still pass regression tests.
- [ ] v0.2.2 upgrades and crash recovery are data-safe and idempotent.
- [ ] API keys never enter frontend state, SQLite, JSON, backup, logs, or error reports.
- [ ] Project/report/reminder/Storyboard cross-module flows are transactionally consistent.
- [ ] Automated tests, Gstack QA/design review, Windows DPI/monitor checks, fresh install, upgrade install, updater, and updater-signature checks pass; Authenticode status is stated accurately.
- [ ] The user explicitly approves the final release before publication.
