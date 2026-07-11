# Banana Box v1 Integration And Release Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将基础架构、香蕉桌面交互、项目/当日任务和 Storyboard Agent 合并成一个可从 v0.2.2 安全升级、可重复验证、带 Tauri updater 签名发布的 Banana Box v1.0.0 Windows 版本。

**Architecture:** 各功能子计划先在同一 `codex/v1-major-update` 分支完成并通过自己的测试，本计划再冻结共享接口、补跨域契约测试、执行真实升级夹具、收紧 Tauri 安全配置，并通过唯一的 release candidate 提交生成带 updater 签名的安装包。版本发布采用“先本地与 CI 证据、后人工批准发布”的门禁；任何 P0/P1、迁移数据差异、密钥泄漏或升级安装失败都会阻断 v1.0.0。

**Tech Stack:** Git, GitHub Actions, PowerShell, pnpm, Vue 3, Vitest, Rust, Cargo, Tauri 2, SQLite, Gstack browse/qa/design-review, Windows NSIS/MSI updater.

---

## Preconditions

- [ ] 以下四份子计划必须完成并各自保留绿灯提交：
  - [`2026-07-11-banana-box-v1-foundation.md`](./2026-07-11-banana-box-v1-foundation.md)
  - [`2026-07-11-banana-box-v1-desktop-interaction.md`](./2026-07-11-banana-box-v1-desktop-interaction.md)
  - [`2026-07-11-banana-box-v1-production-management.md`](./2026-07-11-banana-box-v1-production-management.md)
  - [`2026-07-11-banana-box-v1-storyboard-agent.md`](./2026-07-11-banana-box-v1-storyboard-agent.md)
- [ ] 工作目录必须是正式仓库克隆 `C:\Users\Felix\Downloads\banana-box-workspace`，当前分支必须是 `codex/v1-major-update`，远端必须是 `https://github.com/felix1709/banana-box.git`。
- [ ] 不允许用本地源码目录 `C:\Users\Felix\Downloads\banana-box-main` 覆盖正式克隆；它只可作为只读比对来源。
- [ ] 发布动作必须等待用户在 release candidate 验收后单独确认；本计划中的 tag/push/release 命令不是预授权。

### Task 0: Prove GitHub CLI, Authentication, And Release-gate Feasibility

**Files:**
- No source changes

- [ ] Make PowerShell 7.4+ a hard release-tooling prerequisite. The inspected machine currently has Windows PowerShell `5.1` and no `pwsh`; later blocks require .NET APIs such as `ProcessStartInfo.ArgumentList`, `RandomNumberGenerator.Fill`, and `Path.GetRelativePath` that 5.1 does not provide.

  ```powershell
  $pwshCommand = Get-Command pwsh -ErrorAction SilentlyContinue
  if (-not $pwshCommand) { throw 'PowerShell 7.4+ is missing; request approval before installing Microsoft.PowerShell' }
  $pwshVersion = & $pwshCommand.Source -NoLogo -NoProfile -Command '$PSVersionTable.PSVersion.ToString()'
  if ([version]$pwshVersion -lt [version]'7.4.0') { throw "PowerShell 7.4+ required, found $pwshVersion" }
  ```

  Stop for user approval before `winget install --id Microsoft.PowerShell --exact --source winget --accept-package-agreements --accept-source-agreements`. After installation, open `pwsh -NoLogo -NoProfile`, rerun the version check, and execute **every remaining Task 0/release/signing/hash block in that PowerShell 7 session**. Add `#requires -Version 7.4` to committed release `.ps1` scripts and set Windows GitHub Actions run steps to `shell: pwsh`; never fall back silently to `powershell.exe` 5.1.

- [ ] Require `actionlint` before authoring workflows; a missing parser is not a skipped-success path. Verify `actionlint -version` in PowerShell 7. If absent, stop for user approval before `winget install --id rhysd.actionlint --exact --version 1.7.12 --source winget --accept-package-agreements --accept-source-agreements`, then record the version. Task 7 runs it unconditionally on both workflow files.

- [ ] Separate the three layers instead of treating “GitHub 连接” as one switch. Prove Git/HTTPS access to the formal remote first:

  ```powershell
  Set-Location C:\Users\Felix\Downloads\banana-box-workspace
  Test-NetConnection github.com -Port 443
  git ls-remote origin HEAD
  git fetch --dry-run origin
  ```

  Expected: port 443 succeeds and both Git commands exit `0`. This proves clone/fetch networking and Git Credential Manager independently of the GitHub connector/CLI.

- [ ] Verify GitHub CLI exists before any later `gh` command:

  ```powershell
  $ghCommand = Get-Command gh -ErrorAction SilentlyContinue
  if (-not $ghCommand) { throw 'GitHub CLI is missing; request approval before installing GitHub.cli' }
  $ghCommand.Source
  gh --version
  ```

  On the currently inspected machine `gh` is absent even though GitHub HTTPS works; this is the concrete reason CLI-based repository/release operations are unavailable. Stop and request the user's approval before system installation. After approval only, install with `winget install --id GitHub.cli --exact --source winget --accept-package-agreements --accept-source-agreements`, open/refresh the PowerShell PATH, and rerun the check. Do not download an unverified executable ad hoc.

- [ ] Authenticate interactively and prove repository access without putting a token on the command line:

  ```powershell
  function Invoke-BananaGhApi {
    param([string[]]$Arguments, [AllowNull()][string]$InputObject)
    $headers = @('-H', 'Accept: application/vnd.github+json', '-H', 'X-GitHub-Api-Version: 2026-03-10')
    if ($PSBoundParameters.ContainsKey('InputObject')) {
      $result = $InputObject | gh api @headers @Arguments
    } else {
      $result = gh api @headers @Arguments
    }
    if ($LASTEXITCODE -ne 0) { throw "GitHub API failed: $($Arguments -join ' ')" }
    $result
  }
  gh auth status --hostname github.com
  if ($LASTEXITCODE -ne 0) {
    gh auth login --hostname github.com --git-protocol https --web
    if ($LASTEXITCODE -ne 0) { throw 'GitHub CLI browser authentication failed' }
  }
  gh auth status --hostname github.com
  $viewer = Invoke-BananaGhApi -Arguments @('user','--jq','.login')
  if (-not $viewer) { throw 'cannot read authenticated GitHub user' }
  $repository = Invoke-BananaGhApi -Arguments @('repos/felix1709/banana-box') | ConvertFrom-Json
  if (-not $repository.permissions.push) { throw 'authenticated user cannot push banana-box' }
  ```

  Never use `--with-token <value>`, `--body <secret>`, transcript logging, or an echoed environment variable. Browser authentication or secret stdin is the only allowed credential input.

- [ ] Prove the two-person environment gate before feature implementation, not at release time. Require non-empty `BANANA_RELEASE_REVIEWER`, require it differs from `$viewer`, resolve its user ID, and verify it is a collaborator. With explicit user approval to create/update repository environment configuration, PUT `production-release` with exactly that reviewer and `prevent_self_review=true`, then GET it and assert the rule, unique reviewer ID/login, and prevention flag exactly. Do not upload signing secrets yet.

  ```powershell
  $publisher = Invoke-BananaGhApi -Arguments @('user','--jq','.login')
  if (-not $publisher) { throw 'cannot read publisher' }
  if (-not $env:BANANA_RELEASE_REVIEWER -or $env:BANANA_RELEASE_REVIEWER -eq $publisher) { throw 'a different release reviewer is required' }
  $reviewerId = [int64](Invoke-BananaGhApi -Arguments @("users/$env:BANANA_RELEASE_REVIEWER",'--jq','.id'))
  if ($reviewerId -le 0) { throw 'cannot resolve release reviewer' }
  Invoke-BananaGhApi -Arguments @("repos/felix1709/banana-box/collaborators/$env:BANANA_RELEASE_REVIEWER",'--silent') | Out-Null
  $body = @{
    wait_timer = 0
    prevent_self_review = $true
    reviewers = @(@{ type = 'User'; id = $reviewerId })
    deployment_branch_policy = @{ protected_branches = $false; custom_branch_policies = $true }
  } | ConvertTo-Json -Depth 4
  Invoke-BananaGhApi -Arguments @('--method','PUT','repos/felix1709/banana-box/environments/production-release','--input','-') -InputObject $body | Out-Null
  $environment = Invoke-BananaGhApi -Arguments @('repos/felix1709/banana-box/environments/production-release') | ConvertFrom-Json
  $rule = @($environment.protection_rules | Where-Object type -eq 'required_reviewers')
  if ($rule.Count -ne 1 -or -not $rule[0].prevent_self_review) { throw 'required-reviewer rule is not enforced' }
  if (@($rule[0].reviewers).Count -ne 1) { throw 'exactly one reviewer must be persisted' }
  $persisted = $rule[0].reviewers[0].reviewer
  if ([int64]$persisted.id -ne $reviewerId -or $persisted.login -ne $env:BANANA_RELEASE_REVIEWER) { throw 'persisted reviewer does not match' }
  ```

  Delete any stale custom deployment policy only after separately confirming it is not used by another release flow, then create exactly one policy through the deployment-branch-policies API with `{ name:'v*', type:'tag' }`. Re-list and require exactly that tag policy and no branch policy. Include its ID/name/type plus `deployment_branch_policy={protected_branches:false,custom_branch_policies:true}` in `ENVIRONMENT_POLICY_FINGERPRINT`; this prevents any branch or non-version tag from even requesting the signing environment.

  GitHub's environment REST schema does not expose the UI setting “Allow administrators to bypass configured protection rules”. Do not send or GET a fictitious `can_admins_bypass` field. In an authenticated browser, open the exact `production-release` environment, turn that option off, reload and verify it remains off, and save a timestamped screenshot plus repository/environment/account identity outside the repo as `ADMIN_BYPASS_UI_EVIDENCE`. Repeat this browser check immediately before every later policy phase; the policy script requires a refreshed evidence record no older than ten minutes, while actual run `/approvals` evidence must still prove the different reviewer approved. If the UI option is unavailable or cannot be verified, stop and redesign the two-person gate.

  If the collaborator is absent, the authenticated user lacks repository administration, or the repository/account plan rejects required-reviewer protection, stop here. The valid choices are to add/authorize the collaborator or explicitly revise the confirmed design to a different release gate; never silently weaken the two-person rule after development. Record the successful CLI path, authenticated login, repository permission, reviewer login, and environment rule without recording any token.

- [ ] Prove the repository can preserve the tested `RC_SHA` through merge before feature work:

  ```powershell
  $repoPolicy = Invoke-BananaGhApi -Arguments @('repos/felix1709/banana-box') | ConvertFrom-Json
  if (-not $repoPolicy.allow_merge_commit) {
    throw 'merge commits are disabled; revise the RC qualification strategy before development'
  }
  [pscustomobject]@{
    allowMergeCommit = $repoPolicy.allow_merge_commit
    allowSquashMerge = $repoPolicy.allow_squash_merge
    allowRebaseMerge = $repoPolicy.allow_rebase_merge
  }
  ```

  Lock the v1 PR to GitHub **merge commit** only; its head may be `RC_SHA` plus the explicitly allowed documentation-only QA evidence commit, so `RC_SHA` remains an ancestor of main. Require no active merge-queue rule for this release because the later expected-head direct merge/evidence flow does not model a queue-generated commit. Do not use squash/rebase. If policy requires merge queue or cannot allow merge commits, stop now and explicitly revise/requalify the post-queue/post-rewrite commit as a new RC with the complete build/sign/install/updater matrix; never discover this after Approval A or tag a SHA absent from main history.

- [ ] Audit the effective `main` protection before feature work, not merely merge-method toggles. With API version `2026-03-10`, read `repos/felix1709/banana-box/branches/main`, `/branches/main/protection`, and `/rulesets?includes_parents=true`; canonicalize the relevant JSON and record its SHA-256 as `MAIN_POLICY_FINGERPRINT` outside the repository. Require `main.protected=true`, pull-request review protection with at least one approval, strict required checks whose contexts exactly include the future workflow job names `frontend`, `rust`, and `windows-smoke`, admin enforcement, `allow_force_pushes.enabled=false`, `allow_deletions.enabled=false`, and no required-linear-history rule that would forbid the locked merge-commit strategy. Include effective organization rulesets and bypass actors in the evidence; an unexpected admin/team/app bypass is a blocker. Canonicalize only the REST-verifiable `production-release` reviewer rule, publisher/reviewer identities, `prevent_self_review=true`, and deployment tag policy into `ENVIRONMENT_POLICY_FINGERPRINT`. Record `EXPECTED_SECRET_ALLOWLIST={TAURI_SIGNING_PRIVATE_KEY,TAURI_SIGNING_PRIVATE_KEY_PASSWORD}` separately because Task 0 intentionally uploads no secret; keep the UI-only administrator-bypass screenshot/timestamp as separate evidence rather than pretending it is a REST field.

  Also require an active repository ruleset targeting `refs/tags/v*` that permits first creation but blocks updates and deletion after creation, applies to administrators, and has no unexpected bypass actor. Fingerprint its condition, enforcement, update/deletion rules, and bypass actors as `TAG_POLICY_FINGERPRINT`. If the account cannot express this, stop for an explicit downgrade decision; do not call the tag immutable. If classic protection/ruleset APIs are unavailable on the account plan, rules conflict, or required checks cannot be configured, stop and get explicit approval to revise the release gate. With separate explicit repository-administration approval, configure the missing protection, GET it again, and fingerprint the persisted result; never assume the PR UI is protected. Task 7 workflow tests lock those job names, and the committed policy assertion script later re-reads/canonicalizes every endpoint and compares all fingerprints before push, merge, tag, Approval B, and publish.

- [ ] Audit immutable-release policy using `GET /repos/felix1709/banana-box/immutable-releases` with `X-GitHub-Api-Version: 2026-03-10`: HTTP 200 with `{ enabled:true }` means enabled and HTTP 404 means disabled; any other response blocks release planning. v1's confirmed canary rollback depends on being able to return the release to Draft/remove `latest.json`, so require disabled at Task 0, before Approval B, and immediately before publish. If `enforced_by_owner=true`, or the user prefers immutable releases, stop and explicitly redesign/approve the incident path before development: after publication assets cannot be changed/deleted, and deleting an immutable release makes its tag name permanently non-reusable. Do not silently disable a repository/organization security setting. Record the status and API version in the external policy evidence. Because this deliberately trades permanent asset immutability for canary rollback, evidence may claim exact hashes at Approval B/publication/canary only, never that assets are forever immutable; the separate tag ruleset still forbids tag movement/deletion.

- [ ] Freeze the official v0.2.2 installer baseline before feature work. Resolve release tag `v0.2.2` through the REST API, require published/non-prerelease, inspect the Git ref object kind, and resolve it to baseline `299dde2db3274a9c2ed844698795a6d4ed317126`: the known historical tag is lightweight (`type=commit`), so compare its ref SHA directly; if the server ever returns an annotated `type=tag`, recursively peel and compare the resulting commit instead. Record `tagKind` rather than incorrectly requiring an annotated historical tag. Download the exact NSIS, MSI, both `.sig` files, and `latest.json` to a fresh external directory. Record release ID plus every asset ID/name/size/API digest/calculated SHA-256/browser URL as `V022_ASSET_FINGERPRINT`. Validate manifest version/platform/production URLs and bind each signature/URL to the downloaded asset. Strictly outer-Base64-decode each `.sig` and verify its decoded Minisign box with the public key from the v0.2.2 source. Missing/duplicate assets, mutable metadata during the two reads, digest mismatch, bad signature, or tag mismatch blocks the release plan. Task 11 and the post-publication canary must freshly download v0.2.2 and exactly match this frozen fingerprint before calling it “official, untouched v0.2.2.” v1.0.0 itself remains required to use an annotated tag.

- [ ] Prove the existing updater key pair before feature work. Require `BANANA_TAURI_KEY_PATH` and `BANANA_TAURI_PASSWORD_PATH` to name non-empty external files. Run `pnpm install --frozen-lockfile`, verify `pnpm tauri signer sign --help` exists, and require a trusted Minisign verifier. If `minisign` is absent, stop for user approval before installing the official Winget package `jedisct1.minisign`; never fetch an ad hoc binary.

  With transcript/debug tracing disabled, sign cryptographically random dummy bytes using only temporary process environment variables, then verify the resulting `.sig` against the exact `plugins.updater.pubkey` already committed in `src-tauri/tauri.conf.json`:

  ```powershell
  if (-not (Test-Path -LiteralPath $env:BANANA_TAURI_KEY_PATH -PathType Leaf)) { throw 'updater private key file missing' }
  if (-not (Test-Path -LiteralPath $env:BANANA_TAURI_PASSWORD_PATH -PathType Leaf)) { throw 'updater key password file missing' }
  if ((Get-Item -LiteralPath $env:BANANA_TAURI_KEY_PATH).Length -eq 0) { throw 'updater private key file is empty' }
  if ((Get-Item -LiteralPath $env:BANANA_TAURI_PASSWORD_PATH).Length -eq 0) { throw 'updater password file is empty' }
  if (-not (Get-Command minisign -ErrorAction SilentlyContinue)) { throw 'minisign verifier missing; request approval before winget install --id jedisct1.minisign' }
  pnpm tauri signer sign --help | Out-Null
  if ($LASTEXITCODE -ne 0) { throw 'Tauri signer is unavailable' }

  $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
  $preflightDir = Join-Path $tempRoot ("banana-updater-key-preflight-" + [guid]::NewGuid())
  $dummy = Join-Path $preflightDir 'dummy.bin'
  $signature = "$dummy.sig"
  $decodedSignature = Join-Path $preflightDir 'dummy.minisig'
  New-Item -ItemType Directory -Path $preflightDir -ErrorAction Stop | Out-Null
  $bytes = [byte[]]::new(4096)
  [Security.Cryptography.RandomNumberGenerator]::Fill($bytes)
  [IO.File]::WriteAllBytes($dummy, $bytes)
  try {
    $env:TAURI_SIGNING_PRIVATE_KEY = Get-Content -Raw -LiteralPath $env:BANANA_TAURI_KEY_PATH
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = (Get-Content -Raw -LiteralPath $env:BANANA_TAURI_PASSWORD_PATH).TrimEnd([char[]]"`r`n")
    if ([string]::IsNullOrEmpty($env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD)) { throw 'updater password is empty after newline trimming' }
    pnpm tauri signer sign $dummy
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $signature -PathType Leaf)) { throw 'dummy updater signing failed' }
    $config = Get-Content -Raw -LiteralPath src-tauri/tauri.conf.json | ConvertFrom-Json
    $encodedPubkey = [string]$config.plugins.updater.pubkey
    if (-not $encodedPubkey) { throw 'committed updater public key is missing' }
    try {
      $decodedPubkeyText = [Text.UTF8Encoding]::new($false, $true).GetString(
        [Convert]::FromBase64String($encodedPubkey)
      )
    } catch {
      throw 'committed updater public key is not valid outer Base64 UTF-8'
    }
    $pubkeyLines = @($decodedPubkeyText -split "`r?`n" | Where-Object { $_ -match '^RW[A-Za-z0-9+/=]+$' })
    if ($pubkeyLines.Count -ne 1) { throw 'decoded updater public key must contain exactly one Minisign RW line' }
    $minisignPubkey = $pubkeyLines[0]
    $encodedSignatureText = (Get-Content -Raw -LiteralPath $signature).Trim()
    if (-not $encodedSignatureText -or $encodedSignatureText.Length % 4 -ne 0 -or $encodedSignatureText -notmatch '^[A-Za-z0-9+/]+={0,2}$') {
      throw 'Tauri updater signature is not strict outer Base64'
    }
    try {
      $decodedSignatureBytes = [Convert]::FromBase64String($encodedSignatureText)
      $decodedSignatureText = [Text.UTF8Encoding]::new($false, $true).GetString($decodedSignatureBytes)
    } catch {
      throw 'Tauri updater signature outer Base64 is not valid UTF-8'
    }
    if ($decodedSignatureText -notmatch '^untrusted comment: signature from tauri secret key\r?\n[A-Za-z0-9+/=]+\r?\ntrusted comment: .+\r?\n[A-Za-z0-9+/=]+\r?\n?$') {
      throw 'decoded Tauri updater signature is not a Minisign signature box'
    }
    [IO.File]::WriteAllBytes($decodedSignature, $decodedSignatureBytes)
    minisign -Vm $dummy -x $decodedSignature -P $minisignPubkey
    if ($LASTEXITCODE -ne 0) { throw 'private key/password do not match the committed updater public key' }
  } finally {
    Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue
    Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
    $bytes = $null
    $encodedPubkey = $null
    $decodedPubkeyText = $null
    $minisignPubkey = $null
    $pubkeyLines = $null
    $encodedSignatureText = $null
    $decodedSignatureBytes = $null
    $decodedSignatureText = $null
    $resolvedPreflight = [IO.Path]::GetFullPath($preflightDir)
    if (-not $resolvedPreflight.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'refusing unsafe preflight cleanup path' }
    Remove-Item -LiteralPath $resolvedPreflight -Recurse -Force -ErrorAction SilentlyContinue
  }
  ```

  Expected: Tauri signer unlocks the key, the generated `.sig` is first strictly outer-Base64-decoded to its Minisign signature box, Minisign verifies with the existing app public key, temp files are removed, and both signing environment variables are absent even on controlled failure. A test must prove correct signature PASS and tampered payload, outer signature, or decoded signature FAIL. If verification fails, v0.2.2 cannot trust a v1 updater signed by this key; stop and explicitly choose key recovery or a documented non-updater migration strategy before implementing v1. Never replace the committed public key and pretend existing installs can update.

### Task 1: Freeze Cross-plan Contracts And Resolve Shared-file Conflicts

**Files:**
- Create: `docs/architecture/v1-integration-contracts.md`
- Modify: `src/App.vue`
- Modify: `src/main.ts`
- Modify: `src/stores/ui.ts`
- Modify: `src/components/AppSidebar.vue`
- Modify: `src/lib/ipc.ts`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/app_state.rs`
- Modify: `src-tauri/src/db/schema.rs`
- Modify: `src-tauri/migrations/0001_v1.sql`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/capabilities/*.json`
- Test: `tests/integration/navigation-contract.test.ts`
- Create: `src-tauri/src/integration_tests/mod.rs`
- Test: `src-tauri/src/integration_tests/v1_contract.rs`

- [ ] Before resolving conflicts, write `v1-integration-contracts.md` with the single source of truth:

  ```text
  ActiveTool = prompts | reverse-image | compression | storyboard | projects | daily-tasks
  Tauri windows = main | floatbtn | reminder
  Main window = default 1080x720, minimum 760x560
  Startup service = AppServices { db, providers, provider_http, operations, images } + StartupGate + AgentRuntime
  SQLite migration = migrations/0001_v1.sql, schema version 1
  Daily-task navigation = daily_tasks::navigation::navigate_to_daily_tasks
  Navigation event = open-daily-tasks
  Storyboard stream events = storyboard-request-delta | storyboard-request-terminal
  Reminder events = reminder-attention | reminder-prepare | reminder-show | reminder-hide-request | reminder-hide | reminder-unread-changed
  Reminder exit IPC = ack_reminder_exit (reminder-only authorized envelope)
  Panel reveal synchronization = panel-target-changed -> ack_panel_reveal(generation, frame>=6)
  ```

- [ ] Add a failing frontend contract test that imports every `ActiveTool`, mounts `AppSidebar`, selects each tool, and verifies `App.vue` has exactly one matching page. Verify `main.ts` mounts `FloatButton`, `ReminderWindow`, or `MainRoot` based on `getCurrentWindow().label`, and assert `App` is mounted only inside `MainRoot` after a ready startup status. Add a config assertion that the `main` window is `1080 × 720` with `minWidth=760` and `minHeight=560`.

- [ ] Add a failing Rust contract test that opens the v1 schema and asserts every table/index/foreign key from the design exists once. Assert the command handler contains no duplicate command names. Build the real handler twice: Ready manages the exact five-field `AppServices`, while Recovery deliberately does not; both modes manage the same standalone operation/staging/validator states, empty `AgentRuntime`, empty `ReminderWindowRuntime`, empty `ReminderUnreadRuntime`, and `ReminderEligibilityFence`. Invoke malformed and valid payloads for representative Foundation, Production, Storyboard send/cancel/retry, and Reminder unread/activation/layout/action commands from wrong/correct labels in Recovery. Prove the authorized envelope returns `FORBIDDEN_WINDOW` before serde for wrong labels, `INVALID_INPUT` for malformed correct-label payload, then `STARTUP_NOT_READY` for valid business payload with zero repository/service/agent-runtime/reminder-window/unread-runtime/fence calls; a required `State<AppServices>` argument or any raw framework serde/missing-state error fails the contract. Only the three recovery-safe full-restore commands `inspect_full_backup`, `restore_full_backup`, and `discard_full_backup_preview` may continue past the Recovery gate using standalone dependencies.

- [ ] Run red tests:

  ```powershell
  pnpm vitest run tests/integration/navigation-contract.test.ts
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml integration_tests::v1_contract
  ```

  Expected: at least one failure until shared-file integrations are reconciled.

- [ ] Resolve shared files manually; do not accept one subplan's whole version over another. Keep one `Database`, one `AppServices`, one `invoke_handler`, one `ActiveTool`, one sidebar array, one main-window router, and one schema-v1 migration. Preserve all existing prompt/reverse/compression commands and tests. In `tauri.conf.json`, replace the old 720×520 main window values with `width: 1080`, `height: 720`, `minWidth: 760`, and `minHeight: 560`; keep the approved decoration, visibility, resizing, taskbar, and pin behavior unchanged.

- [ ] Ensure module registration in `lib.rs` follows this explicit shape (module internals may differ):

  ```rust
  mod app_state;
  mod agent;
  mod backup;
  mod backup_validation;
  mod command_auth;
  mod commands;
  mod daily_tasks;
  mod db;
  mod desktop_state;
  mod fs_atomic;
  mod image_store;
  mod legacy_import;
  mod library;
  mod migration;
  mod projects;
  mod provider_http;
  mod providers;
  mod production_backup_validator;
  mod reminder;
  mod safe_archive;
  mod secrets;
  mod skills;
  mod startup;
  mod storyboard;
  mod window_state;
  #[cfg(test)]
  mod integration_tests;
  ```

  Start `integration_tests/mod.rs` with `mod v1_contract;`. These are crate-internal white-box integration modules, so they can exercise private migration failpoints without making production APIs public. Each later task appends its module name under `#[cfg(test)]`. The contract test constructs final `AppServices` with exactly one shared `ProviderHttpClient`, `AppOperationGate`, `CredentialMutationCoordinator`, and `ImageStore`; proves the standalone managed gate is `Arc::ptr_eq` to `AppServices.operations`, StartupCoordinator/ProviderService share the credential coordinator, reverse-image and Storyboard share the client, and every image command shares the store. It also proves every business/internal/deferred write uses the gate, AgentRuntime is registered as the concrete restore blocker, and every shipped daily-task command uses `ReminderDailyTaskMutationHook` rather than `NoopDailyTaskMutationHook`.

  Construct the final `BackupDomainValidatorRegistry` before startup and assert its stable sorted name set is exactly `{foundation-v1, production-v1, reminder-v1, storyboard-v1}` with no duplicate or missing domain; `foundation-v1` owns Provider/core structured fields while bounded `validate_library` runs alongside the registry. For each name, remove only that registration in a test setup and prove ordinary startup selected-tuple validation plus full-backup inspect/pre-switch/ack all fail closed with the safe missing-domain code and zero live writes. Then run the complete valid v1 fixture through all four boundaries and expect PASS. Cross-domain fixtures combine Project/daily/reminder/Storyboard references so a validator cannot assume another domain silently repaired its rows.

- [ ] Run the contract tests again:

  ```powershell
  pnpm vitest run tests/integration/navigation-contract.test.ts
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml integration_tests::v1_contract
  ```

  Expected: PASS.

- [ ] Commit:

  ```powershell
  git add docs/architecture/v1-integration-contracts.md src src-tauri tests/integration/navigation-contract.test.ts src-tauri/src/integration_tests/v1_contract.rs
  git commit -m "chore: integrate Banana Box v1 modules"
  ```

### Task 2: Create Deterministic v0.2.2 Upgrade And Recovery Fixtures

**Files:**
- Create: `src-tauri/tests/fixtures/v022-complete/library.json`
- Create: `src-tauri/tests/fixtures/v022-missing-fields/library.json`
- Create: `src-tauri/tests/fixtures/v022-corrupt/library.json`
- Create: `src-tauri/tests/fixtures/v1-backup/manifest.json`
- Create: `src-tauri/tests/fixtures/README.md`
- Create: `src-tauri/src/integration_tests/upgrade_v022.rs`
- Create: `src-tauri/src/integration_tests/startup_recovery.rs`
- Modify: `src-tauri/src/integration_tests/mod.rs`

- [ ] Build sanitized fixtures from the actual v0.2.2 JSON shape, never from the user's personal data. Include at least two categories, three prompts, image paths, Unicode, favorite true/false, nontrivial order, theme/hotkey, reverse Provider endpoint/model, and a fake sentinel key `TEST_ONLY_DO_NOT_USE`.

- [ ] Add a second fixture where `favorite` and `order` are absent, a corrupt fixture, and one fixture for each sidecar state: `preparing`, `prepared`, `committing`, `complete`. Document expected post-upgrade hashes and row counts in `fixtures/README.md`.

- [ ] Write `upgrade_v022.rs` tests before changing migration code. The complete fixture must preserve all real fields and move the fake key only into `MemoryCredentialStore`; the missing-field fixture must assign array order and `favorite=false`; official JSON, DB, backup, sidecar, and logs must not contain the sentinel.

- [ ] Write `startup_recovery.rs` as a table test over six startup paths and a failpoint loop over every prepare/commit step, including `init-v1.json` fresh-initialization recovery:

  ```rust
  for failpoint in MigrationFailpoint::ALL {
      let dir = fixture_copy("v022-complete");
      let coordinator = test_coordinator();
      assert!(run_with_failpoint(&coordinator, &dir, *failpoint).is_err());
      let resumed = run_without_failpoint(&coordinator, &dir).unwrap();
      assert_ready_and_idempotent(resumed, &dir);
  }
  ```

- [ ] Run the focused tests:

  ```powershell
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml integration_tests::upgrade_v022
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml integration_tests::startup_recovery
  ```

  Expected: FAIL if any migration phase is incomplete or nondeterministic.

- [ ] Fix only migration/recovery defects exposed by the fixtures. After every simulated crash, either the valid original v0.2.2 data remains or the verified v1 pair is complete; no test may observe a mixed JSON/DB state. Network failure and invalid legacy Key must not block local migration commit.

- [ ] Run the focused tests twice to prove idempotence:

  ```powershell
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml integration_tests::upgrade_v022
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml integration_tests::startup_recovery
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml integration_tests::upgrade_v022
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml integration_tests::startup_recovery
  ```

  Expected: both runs PASS with identical counts/hashes.

- [ ] Commit:

  ```powershell
  git add src-tauri/tests/fixtures src-tauri/src/integration_tests/mod.rs src-tauri/src/integration_tests/upgrade_v022.rs src-tauri/src/integration_tests/startup_recovery.rs src-tauri/src/startup.rs src-tauri/src/migration.rs src-tauri/src/library.rs src-tauri/src/legacy_import.rs
  git commit -m "test: verify v022 to v1 upgrade recovery"
  ```

### Task 3: Test Reminder-to-daily-task Delivery End To End

**Files:**
- Create: `tests/integration/reminder-daily-tasks.test.ts`
- Create: `src-tauri/src/integration_tests/reminder_daily_tasks.rs`
- Modify: `src-tauri/src/integration_tests/mod.rs`
- Modify: `src/components/ReminderWindow.vue`
- Modify: `src/stores/dailyTasks.ts`
- Modify: `src/stores/ui.ts`
- Modify: `tests/stores/ui.test.ts`
- Modify: `src/types/desktop.ts`
- Modify: `src-tauri/src/reminder/mod.rs`
- Modify: `src-tauri/src/daily_tasks/mod.rs`
- Modify: `src-tauri/src/daily_tasks/carry.rs`
- Modify: `src-tauri/src/daily_tasks/navigation.rs`
- Modify: `src-tauri/src/window_state.rs`

- [ ] Write a Rust integration test with a fake clock at a weekday 17:59 -> 18:00. Create unfinished tasks, claim the initial reminder, ACK the rendered window, click “去结算”, and assert `complete_reminder_action` calls `navigate_to_daily_tasks` and emits exactly one `open-daily-tasks` event.

- [ ] Cover snooze once, sleep crossing 18:00, lease expiry, stale fencing token, restart, manual unread reopen, and settlement clearing unread. Assert no sequence produces a third automatic delivery.

- [ ] Write a frontend test that delivers `open-daily-tasks` while another page/settings dialog is open. `ui.openDailyTasks` must close `settingsOpen`, prompt `editorOpen`, `floatingActionDialogOpen`, and `previewImage`, while preserving unsent feature-store drafts; then select `daily-tasks` and load the correct local date. Native focus happens only after the explicit “去结算” click; reminder auto-show itself remains non-focus-stealing.

- [ ] Run red tests:

  ```powershell
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml integration_tests::reminder_daily_tasks
  pnpm vitest run tests/integration/reminder-daily-tasks.test.ts
  ```

- [ ] Reconcile event and navigation wiring. Refactor async `daily_tasks::navigation::navigate_to_daily_tasks` to call `WindowStateService::request_visibility(app, true, PanelTransitionReason::ReminderAction).await`, then emit `open-daily-tasks` only after the real main window is visible/focused. Do not keep its earlier direct `window.show/set_focus` calls and do not duplicate page-opening logic inside reminder code.

- [ ] Finish the production-owned `DailyTaskMutationHook` wiring from the desktop plan. `settle_day_in_transaction(transaction, input, mutation_hook)` owns all carry/report writes but does not commit. The shipped `settle_daily_task_day` command already holds its user-operation permit and `ReminderEligibilityFence`, opens the single `Database::with_transaction`, and supplies `ReminderDailyTaskMutationHook`; that helper sets `unread=0`, maps pending phases to cancelled/rearmed according to the locked eligibility rules, maps shown/hidden settled phases to actioned, and leaves already terminal rows unchanged. After the shared commit and while still holding both guards, the command invokes the desktop-owned nonblocking `begin_reminder_exit` for the exact rendered claim: it only atomically marks `exiting`, emits `reminder-hide-request`, spawns the exact-claim deadline, and returns immediately. The command then emits the revisioned repository-derived unread snapshot and releases both guards; later `ack_reminder_exit`/fallback independently acquire operation permit -> eligibility fence, perform native hide, and emit final `reminder-hide`. It never waits under the settlement guards, directly hides, or skips the 160/80ms exit. A stale timer/ACK then fails its fence. Reopening keeps `report_snapshot`, so the scheduler's `previously_settled` guard cannot create a new initial or snooze phase. Assert the final command/event registry contains `ack_reminder_exit`/both hide events and cannot construct the focused-test no-op hook.

  Invoke that hook only on the `settled=true` path after conflicts are fully resolved and carry/report writes are ready, never on the normal `settled=false` conflict return. A hook error must escape the transaction closure and roll back carry rows, report snapshot, `settled_at`, and reminder mutations together. Add explicit “conflict means zero hook calls” and “hook error means full rollback and retry succeeds” tests.

  The commit boundary is authoritative: once `with_transaction` returns success, a later window/event emission failure must be logged as a sanitized UI-sync warning and must not turn the IPC into a rejected settlement. Return the committed `SettlementResult`, schedule one best-effort hide/unread reconciliation on the app event loop, and let the frontend replace its day from that result. That deferred callback's first action is `services.operations.try_enter_background()`; maintenance means it performs zero repository/native/event access and durable startup reconciliation repairs the UI. Add injected-emitter and restore-versus-callback barriers proving `settled=true`, the snapshot, and carry writes survive an emit failure, while a true database failure still rejects and preserves the dirty settlement dialog.

- [ ] Run tests:

  ```powershell
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml integration_tests::reminder_daily_tasks
  pnpm vitest run tests/integration/reminder-daily-tasks.test.ts
  ```

  Expected: PASS.

- [ ] Commit:

  ```powershell
  git add src-tauri/src/reminder src-tauri/src/daily_tasks/mod.rs src-tauri/src/daily_tasks/carry.rs src-tauri/src/daily_tasks/navigation.rs src-tauri/src/window_state.rs src-tauri/src/integration_tests/mod.rs src-tauri/src/integration_tests/reminder_daily_tasks.rs src/components/ReminderWindow.vue src/stores/dailyTasks.ts src/stores/ui.ts src/types/desktop.ts tests/stores/ui.test.ts tests/integration/reminder-daily-tasks.test.ts
  git commit -m "test: integrate task reminders and navigation"
  ```

### Task 4: Test Projects, Daily Tasks, Reports, And Backups Together

**Files:**
- Create: `src-tauri/src/integration_tests/production_backup_roundtrip.rs`
- Modify: `src-tauri/src/integration_tests/mod.rs`
- Create: `tests/integration/production-workflow.test.ts`
- Modify: `src-tauri/src/backup.rs`
- Modify: `src-tauri/src/projects/repository.rs`
- Modify: `src-tauri/src/daily_tasks/repository.rs`

- [ ] Write a complete production fixture: one project with all eight overlapping stage ranges and fixed colors; grouped L36/L50 tasks with 100% and 50%; notes/duration; settlement snapshot; a carried task edited on the target day; and an archived project.

- [ ] Assert the exact report string includes incomplete tasks:

  ```text
  @日报
  #L36
  1.【L36】【三丽鸥跟进】【100%】
  2.【L36】【412漫画发型跟进】【100%】
  #L50
  1.【L50】【混厄录像带切片制作】【50%】
  ```

- [ ] Export via SQLite online backup, restore into a fresh directory, and compare normalized domain snapshots. Assert Provider credentials are absent/`needs_credentials=true`, task group positions persist, historical report text is unchanged, and project deletion cannot erase historical task codes.

- [ ] Test carry-forward branches after restore: unchanged duplicate is idempotent; a selected carry whose old target was deleted is recreated once; an edited target asks keep/overwrite; overwrite never mutates the source day or its report snapshot.

- [ ] Run:

  ```powershell
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml integration_tests::production_backup_roundtrip
  pnpm vitest run tests/integration/production-workflow.test.ts
  ```

  Expected: tests reveal any transaction/serialization mismatch before UI QA.

- [ ] Fix focused defects, rerun, and commit:

  ```powershell
  git add src-tauri/src/backup.rs src-tauri/src/projects src-tauri/src/daily_tasks src-tauri/src/integration_tests/mod.rs src-tauri/src/integration_tests/production_backup_roundtrip.rs tests/integration/production-workflow.test.ts
  git commit -m "test: verify production data backup roundtrip"
  ```

### Task 5: Test Storyboard Snapshots And Backup Restore Together

**Files:**
- Create: `src-tauri/src/integration_tests/storyboard_backup_roundtrip.rs`
- Modify: `src-tauri/src/integration_tests/mod.rs`
- Create: `tests/integration/storyboard-restart.test.ts`
- Modify: `src-tauri/src/backup.rs`
- Modify: `src-tauri/src/storyboard/repository.rs`
- Modify: `src-tauri/src/skills/repository.rs`

- [ ] Create a local conversation fixture that stops in each workflow state, includes preset/custom answers, safe/unsafe Markdown examples, immutable final-output blocks, a cancelled partial stream, and both original/current retry snapshots.

- [ ] Export/restore and assert: message order, workflow protocol/context, raw Markdown, block boundaries, Skill canonical `files_json` content/hashes/history, and request snapshots survive; credentials and temporary stream buffers do not; restored Providers require credentials. Delete the original local Skill import directory before export and again before post-restore context assembly to prove runtime/backup never depends on that path.

- [ ] Activate a newer compatible Skill after restore. Verify old history remains unchanged, the next request uses the active version, original retry uses its old snapshot, and current retry uses the new hashes.

- [ ] Simulate app restart with a stale `streaming` row. Verify it becomes `interrupted`, the draft and partial output remain visible, and retry is available.

- [ ] Run and commit after PASS:

  ```powershell
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml integration_tests::storyboard_backup_roundtrip
  pnpm vitest run tests/integration/storyboard-restart.test.ts
  git add src-tauri/src/backup.rs src-tauri/src/storyboard src-tauri/src/skills src-tauri/src/integration_tests/mod.rs src-tauri/src/integration_tests/storyboard_backup_roundtrip.rs tests/integration/storyboard-restart.test.ts
  git commit -m "test: verify storyboard restore semantics"
  ```

### Task 6: Lock Down CSP, Window Capabilities, Logs, And Archive Imports

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/capabilities/main.json`
- Modify: `src-tauri/capabilities/floatbtn.json`
- Create: `src-tauri/capabilities/reminder.json`
- Verify absent: `src-tauri/capabilities/default.json`
- Verify absent: `src-tauri/capabilities/desktop.json`
- Create: `src-tauri/src/integration_tests/security_contract.rs`
- Modify: `src-tauri/src/integration_tests/mod.rs`
- Create: `tests/integration/csp-contract.test.ts`
- Modify: `src-tauri/src/backup.rs`
- Modify: `src-tauri/src/skills/import.rs`

- [ ] Write security tests before config changes. Assert CSP is non-null; `floatbtn` and `reminder` do not have dialog/fs/updater/process/global-shortcut permissions; frontend source contains no unsafe raw model `v-html`; and release logs/serialized outputs reject key sentinel strings. Enumerate the final custom command handler by surface: main may call business commands; floatbtn only its panel commands; reminder only its fenced reminder commands. Invoke representative destructive commands (`delete_project`, `settle_daily_task_day`, `restore_full_backup`, `save_ai_provider`, `activate_storyboard_skill_version`) with valid, missing-field, wrong-type, unknown-field, and raw payloads from every wrong/unknown label and assert exact `FORBIDDEN_WINDOW` before deserialization or any state/repository/dialog mock. Correct-label malformed payload returns sanitized `INVALID_INPUT`. Repeat valid correct-label business invocations with a Recovery `StartupGate` and no `AppServices`; assert exact `STARTUP_NOT_READY`, never an invoke argument/state resolution error. This locks the universal `AuthorizedArgs CommandArg -> gate -> AppHandle::try_state` order across all modules.

- [ ] Add archive adversarial tests: input archive length exactly 2 GiB and 2 GiB + 1 byte via injected/sparse length, `..`, absolute paths, UNC paths, alternate separators, symlinks/reparse points, 10,001 files, single file over 512 MiB metadata, expanded total over 4 GiB, ratio over 100:1, hash mismatch, future schema, failed `integrity_check`, and failed `foreign_key_check`.

- [ ] Run red tests:

  ```powershell
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml integration_tests::security_contract
  pnpm vitest run tests/integration/csp-contract.test.ts
  ```

- [ ] Set the exact restrictive CSP below for local bundled assets and Rust-proxied Provider calls, then adjust only if a named production-loading test proves one directive insufficient:

  ```json
  "default-src 'self'; connect-src 'self' ipc: http://ipc.localhost; img-src 'self' asset: http://asset.localhost blob: data:; style-src 'self' 'unsafe-inline'; font-src 'self' data:; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'"
  ```

  Verify actual Tauri production loading before finalizing; do not add Provider hosts to WebView `connect-src` because network calls originate in Rust.

- [ ] Refine the foundation-owned capabilities by window. `main.json` gets only required user-initiated plugins; `floatbtn.json` gets the exact app-listener/event/start-dragging set required by drag/drop and panel events; `reminder.json` gets only exact event listen/unlisten access and no window command. Assert the deleted broad `default.json` and `desktop.json` were not recreated, no auxiliary capability contains `core:default`, and every custom command calls the shared Rust caller guard. Prefer Rust commands over granting broad file-system capability.

- [ ] Ensure logs include request IDs/error codes but exclude keys, full stories, full system prompts, project paths, and Skill contents. Ensure diagnostic UI gets sanitized error summaries.

- [ ] Run security tests plus full Rust archive tests:

  ```powershell
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml integration_tests::security_contract
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml backup::tests
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml skills::import::tests
  pnpm vitest run tests/integration/csp-contract.test.ts tests/lib/safe-markdown.test.ts
  ```

  Expected: PASS.

- [ ] Commit:

  ```powershell
  git add src-tauri/tauri.conf.json src-tauri/capabilities src-tauri/src/backup.rs src-tauri/src/skills/import.rs src-tauri/src/integration_tests/mod.rs src-tauri/src/integration_tests/security_contract.rs tests/integration/csp-contract.test.ts
  git commit -m "security: harden v1 desktop boundaries"
  ```

### Task 7: Add CI Gates For Windows And Cross-platform Static Checks

**Files:**
- Create: `.github/workflows/ci.yml`
- Create: `.github/workflows/release.yml`
- Create: `.node-version`
- Create: `rust-toolchain.toml`
- Create: `scripts/check-release-version.ps1`
- Create: `scripts/build-signed-release.ps1`
- Create: `scripts/assert-release-policy.ps1`
- Create: `scripts/test-updater-signature.ps1`
- Create: `scripts/test-updater-protocol.ps1`
- Create: `scripts/stage-release-assets.ps1`
- Create: `tests/config/release-workflows.test.ts`
- Create: `tests/config/release-scripts.test.ts`
- Create: `tests/config/updater-protocol-harness.test.ts`
- Modify: `docs/release-flow.md`
- Test: `tests/config/release-workflows.test.ts`

- [ ] Create `ci.yml` for pushes to `main` and pull requests targeting `main`, with `permissions: contents: read` and concurrency group `ci-${{ github.workflow }}-${{ github.ref }}` using `cancel-in-progress: true`. Pin every third-party action to the reviewed full commit SHA captured from its official repository on 2026-07-11; keep the human version in a comment and never execute a movable tag/branch:

  ```text
  actions/checkout@9c091bb21b7c1c1d1991bb908d89e4e9dddfe3e0          # v7
  actions/setup-node@48b55a011bda9f5d6aeb4c2d9c7362e8dae4041e        # v6
  pnpm/action-setup@0ebf47130e4866e96fce0953f49152a61190b271         # v6 peeled commit
  dtolnay/rust-toolchain@4be7066ada62dd38de10e7b70166bc74ed198c30    # stable
  actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a      # v7.0.1
  actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c    # v8.0.1
  ```

  During Task 0 resolve and record one supported exact Node 24 LTS patch and one exact Rust stable release, then commit those concrete values in `.node-version` and `rust-toolchain.toml` (including `rustfmt, clippy`); no `24`, `stable`, `latest`, or unresolved placeholder may remain. Use `windows-2025` rather than the moving `windows-latest` label for Windows jobs. Dependency/toolchain update PRs may propose newer reviewed values, but changing Node/Rust, GitHub runner `ImageVersion`, NSIS, WiX, or Tauri CLI after RC qualification invalidates the candidate and requires the full build/install/updater matrix again.

  Define these exact jobs and commands:

  ```text
  frontend / ubuntu-latest
    pnpm install --frozen-lockfile
    pnpm check
    pnpm build

  rust / windows-2025
    cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
    cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path src-tauri/Cargo.toml

  windows-smoke / windows-2025, needs [frontend, rust]
    pnpm install --frozen-lockfile
    pnpm tauri build --debug --no-bundle
  ```

  The Rust job is intentionally Windows because credential/window code is Windows-specific; frontend/static work remains on Ubuntu. Set every Windows PowerShell run step explicitly to `shell: pwsh` and begin release scripts with `#requires -Version 7.4`. Every `run` step must fail the job on a non-zero exit.

- [ ] Use `setup-node` with `cache: pnpm` and `cache-dependency-path: pnpm-lock.yaml`. Cache Cargo only through a reviewed cache action pinned to a full commit SHA, keyed by `runner.os`, `src-tauri/Cargo.lock`, and the toolchain; alternatively omit the Cargo cache in v1. Correctness must not depend on cache restore. Do not expose secrets to pull-request jobs, use `pull_request` rather than `pull_request_target`, and never echo environment variables.

- [ ] After implementing/testing `assert-release-policy.ps1` later in this task, return here and **read-only verify** the Task 0 GitHub environment before relying on it. Require `BANANA_RELEASE_REVIEWER` to name the same collaborator different from the authenticated publisher. Do not PUT/repair any policy here: drift stops execution and returns to a separately approved Task 0 administration step. Only after `PreSecretUpload` passes may this step upload the two signing values from external files without printing them:

  ```powershell
  function Invoke-BananaGhApi {
    param([string[]]$Arguments, [AllowNull()][string]$InputObject)
    $headers = @('-H', 'Accept: application/vnd.github+json', '-H', 'X-GitHub-Api-Version: 2026-03-10')
    if ($PSBoundParameters.ContainsKey('InputObject')) { $result = $InputObject | gh api @headers @Arguments }
    else { $result = gh api @headers @Arguments }
    if ($LASTEXITCODE -ne 0) { throw "GitHub API failed: $($Arguments -join ' ')" }
    $result
  }
  function Set-GhEnvironmentSecret {
    param([string]$Name, [string]$Value)
    $start = [System.Diagnostics.ProcessStartInfo]::new()
    $start.FileName = (Get-Command gh -ErrorAction Stop).Source
    $start.UseShellExecute = $false
    $start.RedirectStandardInput = $true
    foreach ($argument in @('secret', 'set', $Name, '--env', 'production-release')) {
      $start.ArgumentList.Add($argument)
    }
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $start
    try {
      if (-not $process.Start()) { throw "failed to start gh for $Name" }
      $process.StandardInput.Write($Value)
      $process.StandardInput.Close()
      $process.WaitForExit()
      if ($process.ExitCode -ne 0) { throw "gh secret set failed for $Name" }
    } finally {
      $process.Dispose()
      $Value = [string]::Empty
    }
  }

  $publisher = Invoke-BananaGhApi -Arguments @('user','--jq','.login')
  if (-not $publisher) { throw 'cannot read authenticated GitHub user' }
  if (-not $env:BANANA_RELEASE_REVIEWER -or $env:BANANA_RELEASE_REVIEWER -eq $publisher) { throw 'a different release reviewer is required' }
  $reviewerId = [int64](Invoke-BananaGhApi -Arguments @("users/$env:BANANA_RELEASE_REVIEWER",'--jq','.id'))
  if ($reviewerId -le 0) { throw 'cannot resolve release reviewer' }
  Invoke-BananaGhApi -Arguments @("repos/felix1709/banana-box/collaborators/$env:BANANA_RELEASE_REVIEWER",'--silent') | Out-Null
  $environment = Invoke-BananaGhApi -Arguments @('repos/felix1709/banana-box/environments/production-release') | ConvertFrom-Json
  $reviewerRule = @($environment.protection_rules | Where-Object type -eq 'required_reviewers')
  if ($environment.deployment_branch_policy.protected_branches -ne $false -or $environment.deployment_branch_policy.custom_branch_policies -ne $true) { throw 'custom tag deployment policy is not enabled' }
  if ($reviewerRule.Count -ne 1) { throw 'exactly one required-reviewer rule is required' }
  if (-not $reviewerRule[0].prevent_self_review) { throw 'prevent_self_review is not persisted' }
  if (@($reviewerRule[0].reviewers).Count -ne 1) { throw 'exactly one release reviewer must be persisted' }
  $persistedReviewerIds = @($reviewerRule[0].reviewers | ForEach-Object { [int64]$_.reviewer.id })
  $persistedReviewerLogins = @($reviewerRule[0].reviewers | ForEach-Object { $_.reviewer.login })
  if ($reviewerId -notin $persistedReviewerIds -or $env:BANANA_RELEASE_REVIEWER -notin $persistedReviewerLogins) { throw 'the configured reviewer does not match' }
  $policies = Invoke-BananaGhApi -Arguments @('repos/felix1709/banana-box/environments/production-release/deployment-branch-policies') | ConvertFrom-Json
  $policyRows = @($policies.branch_policies | ForEach-Object { "$($_.type)|$($_.name)" })
  if ($policyRows.Count -ne 1 -or $policyRows[0] -ne 'tag|v*') { throw 'deployment policy must be exactly tag v*' }
  $policyEvidence = Get-Content -Raw -LiteralPath $env:BANANA_RELEASE_POLICY_EVIDENCE_PATH | ConvertFrom-Json
  $uiEvidence = $policyEvidence.adminBypassUiEvidence
  $uiAge = (Get-Date).ToUniversalTime() - [datetime]::Parse($uiEvidence.verifiedAt).ToUniversalTime()
  if (-not $uiEvidence.verifiedOff -or $uiEvidence.repository -ne 'felix1709/banana-box' -or $uiEvidence.environment -ne 'production-release' -or $uiAge.TotalMinutes -lt -2 -or $uiAge.TotalMinutes -gt 10) { throw 'administrator-bypass UI evidence is missing, wrong-scope, future-dated, or stale' }
  if (-not (Test-Path -LiteralPath $uiEvidence.screenshotPath -PathType Leaf) -or (Get-FileHash -LiteralPath $uiEvidence.screenshotPath -Algorithm SHA256).Hash -ne $uiEvidence.screenshotSha256) { throw 'administrator-bypass UI screenshot evidence does not match' }
  pwsh -NoProfile -File scripts/assert-release-policy.ps1 -EvidencePath $env:BANANA_RELEASE_POLICY_EVIDENCE_PATH -Phase PreSecretUpload
  if ($LASTEXITCODE -ne 0) { throw 'release policy differs before secret upload' }
  $signingKey = $null
  $signingPassword = $null
  try {
    $signingKey = Get-Content -Raw -LiteralPath $env:BANANA_TAURI_KEY_PATH
    $signingPassword = (Get-Content -Raw -LiteralPath $env:BANANA_TAURI_PASSWORD_PATH).TrimEnd("`r", "`n")
    if (-not $signingKey -or -not $signingPassword) { throw 'signing material is empty' }
    Set-GhEnvironmentSecret -Name TAURI_SIGNING_PRIVATE_KEY -Value $signingKey
    Set-GhEnvironmentSecret -Name TAURI_SIGNING_PRIVATE_KEY_PASSWORD -Value $signingPassword
  } finally {
    $signingKey = $null
    $signingPassword = $null
  }
  $secretNames = @(gh secret list --env production-release --json name --jq '.[].name')
  if ($LASTEXITCODE -ne 0) { throw 'cannot verify release environment secrets' }
  $expectedSecretNames = @('TAURI_SIGNING_PRIVATE_KEY','TAURI_SIGNING_PRIVATE_KEY_PASSWORD') | Sort-Object
  if (Compare-Object $expectedSecretNames @($secretNames | Sort-Object)) { throw 'release environment secret allowlist differs' }
  pwsh -NoProfile -File scripts/assert-release-policy.ps1 -EvidencePath $env:BANANA_RELEASE_POLICY_EVIDENCE_PATH -Phase PostSecretUpload
  if ($LASTEXITCODE -ne 0) { throw 'release policy differs after secret upload' }
  ```

  Before reading/uploading either secret, repeat the authenticated-browser administrator-bypass check and refresh `ADMIN_BYPASS_UI_EVIDENCE`; then recompute the policy-only `ENVIRONMENT_POLICY_FINGERPRINT` and compare it with Task 0. After the exact allowlist is present, `PostSecretUpload` atomically records `ENVIRONMENT_READY_FINGERPRINT = SHA256(ENVIRONMENT_POLICY_FINGERPRINT + sorted actual secret names)` in the external evidence; every later phase requires that ready fingerprint. Extra secret names require a separate audit/removal decision; never silently ignore them.

  Run this block only with PowerShell transcript/debug tracing disabled. `ProcessStartInfo.ArgumentList` contains only command names/options; secret bytes travel through redirected stdin and never appear in the `gh.exe` process command line. Treat any transcript, `Write-*`, `echo`, or `--body <value>` secret path as a release blocker.

  Expected: the different collaborator is listed as required reviewer, self-review is prevented, and only secret names are displayed. If the repository plan/account cannot configure protection rules, release remains blocked; naming an environment alone is not a gate.

- [ ] Create `release.yml` triggered **only** by pushed tags matching `v*`; do not expose `workflow_dispatch`. Give the workflow default `permissions: contents: read` and split it into three jobs so no human approval or secret is requested before automated provenance/tests succeed, and no failed Draft upload ever rebuilds signed bytes:

  - `validate-release-tag` runs on `windows-2025`, has no `environment` and no secret expressions, checks out with `fetch-depth:0` and `persist-credentials:false`, and uses the exact committed Node/Rust versions plus pnpm 11.11.0. Require `GITHUB_REF_TYPE=tag`, strict SemVer tag syntax, `git cat-file -t refs/tags/$env:GITHUB_REF_NAME` exactly `tag` (annotated, not lightweight), its peeled commit exactly `GITHUB_SHA`/HEAD, and `git merge-base --is-ancestor HEAD origin/main` success after an explicit fetch of `origin/main`. Then run the version checker, frozen install, `pnpm check`, `pnpm build`, Rust fmt/clippy/test, and the updater/release source fixtures. Every native command is followed immediately by its own exit-code check.
  - `build-release-assets` runs on `windows-2025`, has `needs: validate-release-tag`, read-only contents permission, `environment: production-release`, and checkout `persist-credentials:false`. Install from the frozen lockfile, fail if either signing secret is empty, and expose both only to one native `pnpm tauri build` step. After secrets leave scope, generate/stage the exact six files and upload that directory once as a pinned `actions/upload-artifact` artifact whose name includes tag/SHA/run ID; record artifact ID/digest. This job has no contents-write token and cannot create a Release.
  - `create-draft` runs on `windows-2025`, has `needs: [validate-release-tag, build-release-assets]`, job-level `permissions: { contents: write, actions: read }`, no environment and no signing-secret expression. It performs a third pinned checkout of exact `GITHUB_SHA` with `persist-credentials:false` solely to obtain/verify `docs/releases/v1.0.0.md`; pinned `actions/download-artifact` fetches the exact artifact ID/name from this run. Verify checkout HEAD, artifact service digest, provenance, notes canonical hash, and six local hashes before Draft creation. Thus an invalid tag, SHA absent from `main`, or failed test cannot request approval/access signing secrets, while failed upload reruns only this job against frozen bytes.

  If build fails before its artifact exists, rerun the failed build and treat its first successful artifact as the only candidate. Once `build-release-assets` succeeds, never rerun it; a Draft failure uses “Re-run failed jobs” so only `create-draft` runs against the same artifact ID/digest. Re-running all jobs or observing a second candidate artifact invalidates the Draft attempt and requires explicit cleanup/requalification. No flow uses branch dispatch or a recreated tag.

  The signed step runs `pnpm tauri build` only. The next build-job step has neither signing variable, asserts both are absent, runs `pnpm release:manifest`, and creates `release-provenance.json` containing schema version, tag, `GITHUB_SHA`, origin run ID/attempt, exact Node/pnpm/rustc/cargo/Tauri/NSIS/WiX versions, runner `ImageOS`/`ImageVersion`, canonical release-notes hash, and five product asset names/sizes/SHA-256 values. Enforce the exact six-file allowlist: NSIS, MSI, their two `.sig` files, `latest.json`, and `release-provenance.json`; verify both signatures, then freeze these bytes in the one Actions artifact before any Release exists.

  `create-draft` runs an idempotent state machine over those frozen artifact bytes. On its first attempt, PreTag evidence proved Release-by-tag 404; create one Draft with `gh release create "${{ github.ref_name }}" --verify-tag --draft --title "Banana Box ${{ github.ref_name }}" --notes-file docs/releases/v1.0.0.md`. On a rerun, accept only the unique same-tag `draft=true`, `prerelease=false`, exact title/body, Actions-authored Release whose lineage points to the same frozen Actions artifact/run/head SHA. An empty Draft or partial exact allowed set is recoverable: retain each existing asset only when all metadata/downloaded bytes match the frozen artifact and upload only missing names. Extra, duplicate, or mismatched assets block; never delete/replace them silently. Provenance is never regenerated on upload rerun, so runAttempt fields and binary nondeterminism cannot cause drift.

  Re-read the Draft body, canonicalize source/body to UTF-8 LF with one terminal newline, and require byte equality, matching SHA-256, and locked headings. Before success, hard-compare exact six API rows with the frozen artifact's lengths/SHA-256 and require the same release ID/Actions uploader/validated run lineage. Record Release/asset IDs plus Actions artifact ID/digest in job summary and external evidence. Failure injection immediately after create and each upload proves rerunning only `create-draft` converges to one release ID/exact set; second-artifact, mismatch, or extra fixtures fail. The job never publishes, and `targetCommitish` is never identity evidence.

- [ ] Implement `scripts/check-release-version.ps1` with `#requires -Version 7.4` and `param([string]$Tag)`. Parse `package.json` and `tauri.conf.json` with `ConvertFrom-Json`; parse exactly the root `[package] version = "..."` in `src-tauri/Cargo.toml` and fail on zero/multiple matches. Require all three SemVer strings equal. When `-Tag` is supplied, require exact `v$version`; when CI tag environment is present, also require HEAD equals that tag's peeled commit. Print only the accepted version. Both `release.yml` and `build-signed-release.ps1` invoke the script directly under `$ErrorActionPreference='Stop'`; do not inspect stale `$LASTEXITCODE` after a PowerShell script call. Add process-level tests for matching versions, each individual file mismatch, malformed/multiple Cargo package versions, correct tag, wrong tag, and tag/HEAD mismatch.

- [ ] Add `tests/config/release-workflows.test.ts` and `release-scripts.test.ts`. They parse both YAML files/scripts and assert every `uses:` is a reviewed 40-hex SHA; release is tag-only; all three jobs use `windows-2025`; validation is read-only/no environment; build depends on validation and alone uses `production-release` plus signing expressions; Draft depends on frozen artifact and alone has contents-write/GITHUB_TOKEN. All three exact-SHA checkouts disable persisted credentials, and Draft checkout notes hash matches provenance before `gh release create --notes-file`. Parse YAML to require workflow/job `env` omit signing keys, exactly one native build step references both, every guard precedes it, upload/download artifact IDs/digests are bound, and Draft rerun never rebuilds. Lock exact-six, notes hash, policy calls, no `targetCommitish`, failure injection, and all self-tests/actionlint.

- [ ] Implement `scripts/assert-release-policy.ps1` with `#requires -Version 7.4` and parameters `-EvidencePath`, `-Phase PreSecretUpload|PostSecretUpload|PrePush|PreMerge|PostMerge|PreTag|ApprovalB|PrePublish`, and optional `-ExpectedTag`. Every REST call uses one internal wrapper that always sends `Accept: application/vnd.github+json` and `X-GitHub-Api-Version: 2026-03-10`; tests reject any raw/unversioned API call. The script fails closed if the external Task 0 JSON is missing, outside the expected user/temp evidence root, malformed, uses another API version, lacks a fingerprint, or its separate `ADMIN_BYPASS_UI_EVIDENCE` record is missing/wrong-scope/future-dated or outside the accepted `[-2,10]` minute age window. For every phase it re-reads and canonically sorts the REST-verifiable effective `main` protection/rulesets, `refs/tags/v*` update/deletion rules and bypass actors, and `production-release` reviewer/deployment-tag policy. `PreSecretUpload` compares the policy-only Task 0 fingerprint and permits only zero or a subset of `EXPECTED_SECRET_ALLOWLIST`, with no extra name; it never repairs policy. `PostSecretUpload` requires the exact two names and atomically records `ENVIRONMENT_READY_FINGERPRINT`; every later phase recomputes and compares that ready fingerprint. It never invents a REST administrator-bypass field and treats the authenticated browser refresh as a required human checkpoint; final workflow review history proves the actual run was approved by the different reviewer. `PreTag -ExpectedTag` additionally requires the local tag absent, both remote tag/ref-peeled queries empty, and the Release-by-tag REST response exactly 404; any existing object enters manual recovery and is never overwritten/recreated. `ApprovalB` and `PrePublish` additionally probe immutable releases and require the recorded disabled/404 state. Fixture tests cover every policy drift/status/error plus missing/stale/future/wrong-scope UI evidence and extra/partial secret sets.

- [ ] Implement `scripts/test-updater-signature.ps1` as the one reusable cryptographic verifier. Required parameters are `-ArtifactPath`, `-SignaturePath`, `-PublicKeyBase64`; optional `-EvidenceOutput` writes only artifact/signature SHA-256 and verification metadata. It strictly decodes the updater public key's outer Base64 to one Minisign `RW...` key line, strictly decodes the Tauri `.sig` outer Base64 to a four-line Minisign signature box without assuming the signature line starts with the public-key prefix, writes only inside a freshly created contained temp directory, calls trusted `minisign -Vm`, checks its exit immediately, and cleans in `finally`. Correct fixture PASS; changed artifact, outer signature, decoded signature, public key, whitespace/junk, duplicate lines, or path alias FAIL.

- [ ] Implement `scripts/test-updater-protocol.ps1` with required `-ArtifactDirectory`, `-ExpectedVersion`, `-ExpectedTag`, `-ExpectedCommit`, `-ExpectedBuildKind Local|GitHubActions`, `-V022FingerprintPath`, `-EvidenceOutput`, and mode `ValidateOnly|LocalEndpoint|EndpointOverride`; `-ControlledBaseUrl` is mandatory for the last two modes. Optional `-ReleaseAssetMetadataPath` is forbidden for local Task 11 staging and required for downloaded Draft/public-release validation; when present it supplies fixed release ID plus six API asset IDs/names/sizes/digests/uploader/times. The script enforces exact six assets and discriminated provenance: common commit/tag/toolchain fields are always required; `buildContext.kind=local` requires sanitized Windows/host-build evidence and forbids run/image fields; `kind=github-actions` requires run ID/attempt plus `ImageOS`/`ImageVersion` and forbids local-host fields. It also verifies read-only original `latest.json`, exact platforms/version/production URLs, matching signatures, and the exact five-asset v0.2.2 baseline.

  `LocalEndpoint` mechanically clones the parsed manifest and changes **only** platform URLs to the controlled loopback origin; version, notes, pub date, signatures, and installer bytes remain identical. Save canonical before/after hashes and a structured JSON-patch allowlist, serve exact files with fixed lengths/no redirects, and exercise discovery/download/verification/install/relaunch in a disposable Windows profile. If TLS is required, trust one ephemeral test certificate only inside that disposable VM; never disable certificate validation.

  `EndpointOverride` creates a random contained detached Git worktree at exact v0.2.2 source commit `299dde2db3274a9c2ed844698795a6d4ed317126`, applies a structured edit whose only tracked diff is the updater endpoint in `src-tauri/tauri.conf.json`, uses the frozen historical lockfiles, records source/diff/binary hashes, builds there, and points the test-only binary at the same local manifest. It never edits/builds from the RC worktree and always removes the verified contained worktree in `finally`. The output JSON records inputs, exact asset hashes, v0.2.2 baseline fingerprint, source diff, discovery/download/signature/install/relaunch result, before/after version and data fixture, and tool versions. `updater-protocol-harness.test.ts` runs `-SelfTest` fixtures for success plus extra/missing asset, URL-only-diff violation, redirect, manifest/signature mismatch, tampered installer, wrong baseline, wrong commit, dirty/out-of-bound worktree, server failure, install failure, and relaunch failure; every negative case exits nonzero and writes no PASS evidence.

- [ ] Implement `scripts/stage-release-assets.ps1` with required `-BundleRoot`, `-StagingDirectory`, `-ExpectedVersion`, `-ExpectedTag`, `-ExpectedCommit`, `-ReleaseNotesPath`, and `-BuildContextKind Local|GitHubActions`. Local requires sanitized OS/build-host evidence and rejects CI fields; GitHubActions requires run ID/attempt and `ImageOS`/`ImageVersion` and rejects local fields. Refuse an existing/non-empty or out-of-bound staging directory; resolve exactly one NSIS, one MSI, adjacent signatures, and `latest.json`; copy only those five bytes, verify them, then write the discriminated `release-provenance.json` with common hashes/notes/commit/tag/tool versions and the selected context. Assert exact six names. Tests cover both valid variants, every missing/foreign branch field, duplicates, unsafe paths, changed notes, and deterministic canonical provenance.

- [ ] Add `scripts/build-signed-release.ps1` as the single local signing wrapper. It reads both external paths only inside `try`, applies `TrimEnd("`r", "`n")` only to the password, runs only the Tauri build while the signing environment exists, and removes both process variables in `finally`. Only after asserting both are absent does it generate the updater manifest. A `-SelfTestCleanup` mode loads dummy values, throws a controlled failure immediately after load, catches only that known sentinel, and proves cleanup plus zero manifest invocation. It never reads real key files in self-test mode.

  Implement this exact control flow; no command prints the loaded values:

  ```powershell
  #requires -Version 7.4
  [CmdletBinding()]
  param([switch]$SelfTestCleanup)
  $ErrorActionPreference = 'Stop'
  $PSNativeCommandUseErrorActionPreference = $true

  function Clear-SigningEnvironment {
    Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue
    Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
  }

  function Invoke-WithSigningEnvironment {
    param(
      [scriptblock]$LoadKey,
      [scriptblock]$LoadPassword,
      [scriptblock]$Build
    )
    $password = $null
    try {
      $env:TAURI_SIGNING_PRIVATE_KEY = & $LoadKey
      $password = & $LoadPassword
      $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = $password.TrimEnd([char[]]"`r`n")
      if ([string]::IsNullOrEmpty($env:TAURI_SIGNING_PRIVATE_KEY)) { throw 'signing key is empty' }
      if ([string]::IsNullOrEmpty($env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD)) { throw 'signing password is empty after newline trimming' }
      & $Build
    } finally {
      Clear-SigningEnvironment
      $password = $null
    }
  }

  if ($SelfTestCleanup) {
    try {
      Invoke-WithSigningEnvironment `
        { 'TEST_ONLY_KEY' } `
        { "TEST_ONLY_PASSWORD`r`n" } `
        { throw 'CONTROLLED_SIGNING_WRAPPER_FAILURE' }
      throw 'controlled failure did not occur'
    } catch {
      if ($_.Exception.Message -ne 'CONTROLLED_SIGNING_WRAPPER_FAILURE') { throw }
    }
    if (Test-Path Env:TAURI_SIGNING_PRIVATE_KEY) { throw 'key cleanup failed' }
    if (Test-Path Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD) { throw 'password cleanup failed' }
    exit 0
  }

  if (-not (Test-Path -LiteralPath $env:BANANA_TAURI_KEY_PATH -PathType Leaf)) { throw 'signing key file missing' }
  if (-not (Test-Path -LiteralPath $env:BANANA_TAURI_PASSWORD_PATH -PathType Leaf)) { throw 'signing password file missing' }

  & "$PSScriptRoot\check-release-version.ps1"

  Invoke-WithSigningEnvironment `
    { Get-Content -Raw -LiteralPath $env:BANANA_TAURI_KEY_PATH } `
    { Get-Content -Raw -LiteralPath $env:BANANA_TAURI_PASSWORD_PATH } `
    { pnpm tauri build; if ($LASTEXITCODE -ne 0) { throw "tauri build failed: $LASTEXITCODE" } }

  if (Test-Path Env:TAURI_SIGNING_PRIVATE_KEY) { throw 'key survived signed build' }
  if (Test-Path Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD) { throw 'password survived signed build' }
  pnpm release:manifest
  if ($LASTEXITCODE -ne 0) { throw "manifest failed: $LASTEXITCODE" }
  ```

  `release-scripts.test.ts` runs the wrapper in a temporary fixture with command shims and proves: correct version/build/manifest succeeds; version failure never invokes build; key-load/password-load/build failures clear both variables and never invoke manifest; manifest runs only after a successful build with both variables absent; manifest failure returns nonzero.

- [ ] Keep signing material only in GitHub Actions secrets and the user's external local secret files. Document required secret names and never add their values:

  ```text
  TAURI_SIGNING_PRIVATE_KEY
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD
  ```

- [ ] Update `docs/release-flow.md` to remove the stale “no .git” diagnosis, use the actual formal clone path, explain the local proxy as machine-local Git config only, and describe recovery if GitHub Actions or the updater draft fails.

- [ ] Validate both workflows locally with the required `actionlint`; absence or a diagnostic blocks the task:

  ```powershell
  $ErrorActionPreference = 'Stop'
  $PSNativeCommandUseErrorActionPreference = $true
  $actionlint = Get-Command actionlint -ErrorAction SilentlyContinue
  if (-not $actionlint) { throw 'actionlint preflight missing' }
  actionlint .github/workflows/ci.yml .github/workflows/release.yml
  if ($LASTEXITCODE -ne 0) { throw 'GitHub Actions workflow validation failed' }
  pnpm exec vitest run tests/config/release-workflows.test.ts tests/config/release-scripts.test.ts tests/config/updater-protocol-harness.test.ts
  pwsh -NoProfile -File scripts/build-signed-release.ps1 -SelfTestCleanup
  pnpm check
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml
  ```

  Expected: local product checks PASS; no secret appears in `git diff`.

- [ ] Commit:

  ```powershell
  git add .github/workflows .node-version rust-toolchain.toml scripts/check-release-version.ps1 scripts/build-signed-release.ps1 scripts/assert-release-policy.ps1 scripts/test-updater-signature.ps1 scripts/test-updater-protocol.ps1 scripts/stage-release-assets.ps1 tests/config/release-workflows.test.ts tests/config/release-scripts.test.ts tests/config/updater-protocol-harness.test.ts docs/release-flow.md
  $expected = @('.github/workflows/ci.yml','.github/workflows/release.yml','.node-version','rust-toolchain.toml','scripts/check-release-version.ps1','scripts/build-signed-release.ps1','scripts/assert-release-policy.ps1','scripts/test-updater-signature.ps1','scripts/test-updater-protocol.ps1','scripts/stage-release-assets.ps1','tests/config/release-workflows.test.ts','tests/config/release-scripts.test.ts','tests/config/updater-protocol-harness.test.ts','docs/release-flow.md') | Sort-Object
  $actual = @(git diff --cached --name-only) | Sort-Object
  if (Compare-Object $expected $actual) { throw 'Task 7 staged allowlist mismatch' }
  if (git status --short | Where-Object { $_ -match '^\?\?' }) { throw 'Task 7 left untracked files' }
  git commit -m "ci: add Banana Box v1 release gates"
  ```

### Task 8: Build The Unified Acceptance Matrix

**Files:**
- Create: `docs/qa/v1-acceptance-matrix.md`
- Create: `docs/qa/v1-known-limitations.md`
- Modify: `tests/components/App.test.ts`
- Modify: `tests/integration/*.test.ts`

- [ ] Translate every normative requirement and explicit “首版不做” boundary in design §§2–15 into a matrix row with ID, source section, feature, setup, action, expected result, automation link, manual evidence, severity, and status. No requirement may be marked “covered” without a test path or named manual procedure; §§12 and 15 are release gates, not the only mapped sections.

- [ ] Include five release-blocking user journeys:
  1. Upgrade v0.2.2 and use the existing prompt library/reverse image without data loss or key exposure.
  2. Click/reverse-click the animated banana, open/close main panel, and receive/use the reminder.
  3. Create a project with overlapping eight-stage dates and read the timeline/current marker.
  4. Enter grouped daily tasks, settle at 50/100%, copy exact report, carry unfinished work, and receive weekday 18:00 reminder.
  5. Configure Storyboard, use the bundled Skill with preset/custom answers, stop/retry, and copy exact Markdown blocks.

- [ ] Document confirmed v1 limitations only: application must be running for reminders; weekdays exclude weekends but not statutory holiday swaps; no Storyboard attachments/tools; no custom stages; no cloud/team sync; no automatic Skill update.

- [ ] Run all automated gates and fill only automated status:

  ```powershell
  pnpm check
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" fmt --manifest-path src-tauri/Cargo.toml -- --check
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml
  pnpm build
  ```

  Expected: every command exits `0` before manual QA starts.

- [ ] Commit:

  ```powershell
  git add docs/qa tests
  git commit -m "test: define Banana Box v1 acceptance matrix"
  ```

### Task 9: Run Desktop Visual, Interaction, DPI, And Accessibility QA

**Files:**
- Modify: only files required by findings
- Update: `docs/qa/v1-acceptance-matrix.md`

- [ ] Start a fresh debug app, not a stale Vite-only tab:

  ```powershell
  pnpm tauri dev
  ```

  Confirm the process uses the current working tree and record the commit SHA in the QA matrix.

- [ ] Run Gstack `browse` for all primary views and states: prompts, reverse image, compression, Storyboard empty/choice/final/error, projects empty/board/editor/timeline, daily tasks/settlement/report, settings, reminder, banana closed/open/intermediate.

- [ ] Run Gstack `qa` across the five release-blocking journeys. Verify there are no blank windows, stale navigation, clipped controls, scroll traps, focus loss, unexpected main-window hiding, or duplicate requests/reminders.

- [ ] Run Gstack `design-review`. Test native minimum `760×560` and default `1080×720`, long Chinese strings, 100/125/150/175/200% Windows scaling, single/multiple monitor positions, display unplug, keyboard-only navigation, reduced motion, and high-contrast/contrast readability. A browser-only 720×520 viewport may be recorded as an overflow stress test, never as a native Tauri size.

- [ ] Verify animation with evidence: click feedback starts within 50 ms; 12 frames complete in 360 ms; fast reverse continues from current frame; visible banana stays within 52 px inside a 64×64 window; reminder emerges from the banana location and remains within the active work area.

- [ ] Classify every finding. Fix P0/P1 and rerun the exact case; P2 may ship only if explicitly accepted and listed in known limitations. Never mark a failed row complete based on visual impression alone.

- [ ] Stop the debug process cleanly after all checks and update the matrix with screenshot/log artifact paths.

- [ ] Commit fixes and evidence metadata (do not commit user data or secrets):

  ```powershell
  git add src src-tauri docs/qa
  git commit -m "fix: resolve v1 acceptance findings"
  ```

  Skip the commit if no tracked files changed.

### Task 10: Prepare The v1.0.0 Release Candidate

**Files:**
- Modify: `package.json`
- Modify: `pnpm-lock.yaml`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`
- Modify: `src-tauri/tauri.conf.json`
- Create: `CHANGELOG.md` if absent, otherwise modify it
- Create: `docs/releases/v1.0.0.md`

- [ ] Verify the worktree is clean and all prior tasks are committed:

  ```powershell
  git status --short
  git log --oneline --decorate -12
  ```

  Expected: no uncommitted files before version editing.

- [ ] Set version `1.0.0` in exactly `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`; regenerate both lockfiles through package managers rather than hand-editing resolved metadata:

  ```powershell
  pnpm install --lockfile-only
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" check --manifest-path src-tauri/Cargo.toml
  ```

- [ ] Add release notes covering the five features, upgrade behavior, credential re-entry rules after full restore, privacy disclosure, known limitations, and rollback/support instructions. Do not claim reminder delivery while the app is closed.

- [ ] Run the Task 7 version-consistency checker (do not modify it in this task), then run all gates from a clean dependency install:

  ```powershell
  pwsh -NoProfile -File scripts/check-release-version.ps1 -Tag v1.0.0
  pnpm install --frozen-lockfile
  pnpm check
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" fmt --manifest-path src-tauri/Cargo.toml -- --check
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
  & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml
  pnpm build
  ```

  Expected: PASS; all three product versions report `1.0.0`.

- [ ] Commit the release candidate:

  ```powershell
  git add package.json pnpm-lock.yaml src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json CHANGELOG.md docs/releases/v1.0.0.md
  git commit -m "chore: prepare Banana Box v1.0.0"
  ```

- [ ] Record the resulting commit SHA as `RC_SHA` in the current shell and an external test-evidence note outside the repository; do not edit the tracked QA matrix yet, because Task 11 must build from a clean `HEAD == RC_SHA`:

  ```powershell
  $RC_SHA = (git rev-parse HEAD).Trim()
  if ($RC_SHA -notmatch '^[0-9a-f]{40}$') { throw 'invalid RC SHA' }
  Set-Content -LiteralPath "$env:TEMP\banana-box-v1-rc-sha.txt" -Value $RC_SHA -NoNewline
  ```

  From this point, any production-code change invalidates the candidate and requires rerunning Tasks 8–10. A resumed shell reloads the exact value with `$RC_SHA = (Get-Content -Raw -LiteralPath "$env:TEMP\banana-box-v1-rc-sha.txt").Trim()` before any comparison.

### Task 11: Build And Test Updater-signed Windows Artifacts

**Files:**
- Generated, not committed: `src-tauri/target/release/bundle/**`
- Update: `docs/qa/v1-acceptance-matrix.md`
- Update: `docs/qa/v1-known-limitations.md`

- [ ] Confirm updater signing key and password files exist without printing contents. Use user-specific external paths or GitHub Actions secrets; do not retain the stale hardcoded `C:\Users\admin` path from the old release doc.

- [ ] Verify and build the exact clean candidate through the committed wrapper. First run its controlled-failure self-test, then the real build; the parent shell never receives either signing value:

  ```powershell
  if ((git rev-parse HEAD).Trim() -ne $RC_SHA) { throw 'HEAD no longer matches RC_SHA' }
  if (git status --porcelain) { throw 'release worktree is not clean' }
  pwsh -NoProfile -File scripts/build-signed-release.ps1 -SelfTestCleanup
  if ($LASTEXITCODE -ne 0) { throw 'signing wrapper cleanup self-test failed' }
  pwsh -NoProfile -File scripts/build-signed-release.ps1
  if ($LASTEXITCODE -ne 0) { throw "signed build failed: $LASTEXITCODE" }
  $localStage = Join-Path $env:TEMP ("banana-box-v1-local-stage-" + [guid]::NewGuid())
  pwsh -NoProfile -File scripts/stage-release-assets.ps1 `
    -BundleRoot src-tauri/target/release/bundle -StagingDirectory $localStage `
    -ExpectedVersion 1.0.0 -ExpectedTag v1.0.0 -ExpectedCommit $RC_SHA `
    -ReleaseNotesPath docs/releases/v1.0.0.md -BuildContextKind Local
  if ($LASTEXITCODE -ne 0) { throw 'failed to stage the exact local release assets' }
  ```

  Expected: the fresh external staging directory contains exactly NSIS, MSI, their two `.sig` files, `latest.json`, and `release-provenance.json` for version `1.0.0`/`RC_SHA`.

  These `.sig` files are Tauri updater signatures, not Windows Authenticode publisher signatures. v1 does not claim a verified Windows publisher unless a separate code-signing certificate, timestamp server, `signtool sign`, and `signtool verify /pa` evidence are supplied before RC freeze. Without that certificate, record “Windows 可能显示未知发布者” in release notes/known limitations; do not describe the installer itself as Authenticode-signed.

- [ ] Run the committed protocol harness against exactly that staging directory and the frozen official v0.2.2 baseline. Store structured evidence outside the repository:

  ```powershell
  if (-not (Test-Path -LiteralPath $env:BANANA_V022_FINGERPRINT_PATH -PathType Leaf)) { throw 'v0.2.2 baseline fingerprint is missing' }
  $protocolEvidence = Join-Path $env:TEMP "banana-box-v1-local-protocol-$RC_SHA.json"
  pwsh -NoProfile -File scripts/test-updater-protocol.ps1 `
    -Mode ValidateOnly -ArtifactDirectory $localStage -ExpectedVersion 1.0.0 -ExpectedBuildKind Local `
    -ExpectedTag v1.0.0 -ExpectedCommit $RC_SHA `
    -V022FingerprintPath $env:BANANA_V022_FINGERPRINT_PATH -EvidenceOutput $protocolEvidence
  if ($LASTEXITCODE -ne 0) { throw 'local updater protocol validation failed' }
  ```

- [ ] Run a four-route Windows installer matrix from snapshots, never reusing an already-upgraded profile: official fingerprint-matched v0.2.2 NSIS → v1 NSIS; official v0.2.2 MSI → v1 MSI; v1 NSIS fresh → uninstall → reinstall; v1 MSI fresh → uninstall → reinstall. Journey 1's actual “preserve existing v0.2.2 data” case runs on both upgrade routes; both fresh routes run its named fresh-install variant that creates/uses a new prompt and reverse-image Provider without claiming migration. Journeys 2–5 run on all four routes. Every route also checks version, applicable data/credential semantics, tray/autostart/shortcut/three windows, multi-DPI, disclosure, uninstall policy, and relaunch. Record route/journey applicability, installer hashes, before/after data hashes, screenshots/log references, and PASS/FAIL; an inapplicable migration cell is `N/A` with the fresh variant evidence, never a false PASS.

- [ ] Test the updater protocol before publication without claiming impossible production discovery. The shipped v0.2.2 endpoint is fixed to `https://github.com/felix1709/banana-box/releases/latest/download/latest.json`; GitHub Draft assets are neither selected by `/releases/latest` nor anonymously downloadable, and the official installed v0.2.2 cannot be redirected to localhost. Therefore run both of these pre-publication checks:

  ```powershell
  $controlledBaseUrl = 'https://127.0.0.1:18443'
  pwsh -NoProfile -File scripts/test-updater-protocol.ps1 `
    -Mode LocalEndpoint -ControlledBaseUrl $controlledBaseUrl `
    -ArtifactDirectory $localStage -ExpectedVersion 1.0.0 -ExpectedTag v1.0.0 -ExpectedBuildKind Local `
    -ExpectedCommit $RC_SHA -V022FingerprintPath $env:BANANA_V022_FINGERPRINT_PATH `
    -EvidenceOutput "$env:TEMP\banana-box-v1-local-endpoint-$RC_SHA.json"
  if ($LASTEXITCODE -ne 0) { throw 'independent local updater journey failed' }
  pwsh -NoProfile -File scripts/test-updater-protocol.ps1 `
    -Mode EndpointOverride -ControlledBaseUrl $controlledBaseUrl `
    -ArtifactDirectory $localStage -ExpectedVersion 1.0.0 -ExpectedTag v1.0.0 -ExpectedBuildKind Local `
    -ExpectedCommit $RC_SHA -V022FingerprintPath $env:BANANA_V022_FINGERPRINT_PATH `
    -EvidenceOutput "$env:TEMP\banana-box-v1-v022-override-$RC_SHA.json"
  if ($LASTEXITCODE -ne 0) { throw 'endpoint-only v0.2.2 updater journey failed' }
  ```

  These prove protocol and signature/install behavior, but explicitly do **not** prove that the untouched production v0.2.2 can discover the future public GitHub `/latest` release. Record that residual gate in the Approval B evidence; only the immediate post-publication canary below can close it. A browser-only download still does not count.

- [ ] Prove the signing environment variables were cleared immediately after build, including the failure-path test:

  ```powershell
  if (Test-Path Env:TAURI_SIGNING_PRIVATE_KEY) { throw 'signing key still present' }
  if (Test-Path Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD) { throw 'signing password still present' }
  ```

- [ ] Mark artifact rows PASS only after all four installer routes, both updater-harness modes, signature verification, and the five journeys succeed. Do not commit generated installers, staging directories, evidence containing machine paths, or keys.

- [ ] Now update `docs/qa/v1-acceptance-matrix.md` and `docs/qa/v1-known-limitations.md` with `RC_SHA`, exact toolchain/runner versions, test totals, six staged artifact hashes, v0.2.2 baseline fingerprint, both updater-harness results, four-route install results, and the Authenticode status. Verify no machine path or secret is present, then commit evidence separately:

  ```powershell
  git add docs/qa/v1-acceptance-matrix.md docs/qa/v1-known-limitations.md
  git diff --cached --check
  git commit -m "docs: record Banana Box v1 release evidence"
  ```

  This documentation-only commit may follow `RC_SHA`; it does not change the tested binary. Any non-`docs/qa/**` change after `RC_SHA` invalidates the candidate.

### Task 12: Generate The CI Draft After Approval, Then Publish After A Second Approval

**Files:**
- Modify after canary only: `docs/qa/v1-acceptance-matrix.md`
- Modify after canary only: `docs/qa/v1-known-limitations.md`
- External, not committed: Approval manifests, VM evidence, and downloaded assets

- [ ] Present `RC_SHA`, exact toolchain/runner versions, local test totals, remaining P2 items, six local artifact hashes, four-route installer results, both updater-harness results, release-notes hash, and Authenticode status. Stop for **Approval A: allow the reviewed PR merge, creation of one protected annotated `v1.0.0` tag, and generation of one non-public GitHub CI Draft**. This does not authorize publication and does not call the mutable-asset release immutable.

- [ ] Only after Approval A, revalidate policy, bind the pushed branch to one exact PR head, and fail on every native command:

  Immediately before each `PrePush`, `PreMerge`, `PreTag`, `ApprovalB`, and `PrePublish` invocation, use the authenticated GitHub settings page to re-check that administrator bypass remains off and refresh the scoped screenshot/timestamp/hash in the external evidence file. The script's ten-minute freshness rule forces a new check when a long test/approval interval elapses.

  ```powershell
  $ErrorActionPreference = 'Stop'
  $PSNativeCommandUseErrorActionPreference = $true
  pwsh -NoProfile -File scripts/assert-release-policy.ps1 -EvidencePath $env:BANANA_RELEASE_POLICY_EVIDENCE_PATH -Phase PrePush
  if ($LASTEXITCODE -ne 0) { throw 'release policy changed before push' }
  if (git status --porcelain) { throw 'release worktree is not clean' }
  git cat-file -e "$RC_SHA^{commit}"
  if ($LASTEXITCODE -ne 0) { throw 'invalid RC_SHA' }
  $LOCAL_HEAD = (git rev-parse HEAD).Trim()
  if ($LASTEXITCODE -ne 0 -or $LOCAL_HEAD -notmatch '^[0-9a-f]{40}$') { throw 'invalid local head' }
  git merge-base --is-ancestor $RC_SHA $LOCAL_HEAD
  if ($LASTEXITCODE -ne 0) { throw 'RC_SHA is not an ancestor of the reviewed head' }
  $postRcPaths = @(git diff --name-only "${RC_SHA}..HEAD" --)
  if ($LASTEXITCODE -ne 0) { throw 'cannot inspect post-RC changes' }
  if ($postRcPaths | Where-Object { $_ -notlike 'docs/qa/*' }) { throw 'non-QA files changed after RC_SHA' }
  git push -u origin codex/v1-major-update
  if ($LASTEXITCODE -ne 0) { throw 'branch push failed' }
  gh auth status --hostname github.com
  if ($LASTEXITCODE -ne 0) { throw 'GitHub authentication failed' }
  $prs = @(gh pr list --head codex/v1-major-update --base main --state all --json number,headRefOid,baseRefName,state | ConvertFrom-Json)
  if ($LASTEXITCODE -ne 0 -or $prs.Count -gt 1) { throw 'cannot identify a unique release PR' }
  if ($prs.Count -eq 0) {
    gh pr create --base main --head codex/v1-major-update --title "Banana Box v1.0.0" --body-file docs/releases/v1.0.0.md
    if ($LASTEXITCODE -ne 0) { throw 'PR creation failed' }
    $prs = @(gh pr list --head codex/v1-major-update --base main --state open --json number,headRefOid,baseRefName,state | ConvertFrom-Json)
  }
  if ($prs.Count -ne 1 -or $prs[0].headRefOid -ne $LOCAL_HEAD -or $prs[0].baseRefName -ne 'main' -or $prs[0].state -ne 'OPEN') { throw 'PR identity differs from pushed head/main' }
  $PR_NUMBER = [int]$prs[0].number
  gh pr checks $PR_NUMBER --watch --fail-fast
  if ($LASTEXITCODE -ne 0) { throw 'required PR checks failed' }
  ```

  Wait until `gh pr view $PR_NUMBER --json reviewDecision,statusCheckRollup,headRefOid,baseRefName` reports `APPROVED`, the same `LOCAL_HEAD`/`main`, and all required check rollups successful. Refresh the UI-only administrator-bypass evidence again; run `pwsh -NoProfile -File scripts/assert-release-policy.ps1 -EvidencePath $env:BANANA_RELEASE_POLICY_EVIDENCE_PATH -Phase PreMerge`; immediately execute `gh pr merge $PR_NUMBER --merge --match-head-commit $LOCAL_HEAD` without `--admin`; then immediately run the same policy script with `-Phase PostMerge`. Check every exit. Re-read that same PR and require `state=MERGED`, the same head/base, `reviewDecision=APPROVED`, successful required checks, non-null `mergedAt`/`mergedBy`, and a non-null `mergeCommit.oid`. Fetch `origin/main`; require the merge commit has exactly two parents, its second parent is `LOCAL_HEAD`, `RC_SHA` is its ancestor, and the merge commit is an ancestor of `origin/main`. Any failure invalidates Approval A; never infer success from the current main tip alone or leave a time gap for an unbound UI merge.

- [ ] Immediately before tag creation, run `assert-release-policy.ps1 -Phase PreTag -ExpectedTag v1.0.0`. It must prove policy unchanged and local tag, both remote tag refs, and Release-by-tag are all absent (REST exactly 404). Then create/push once, checking each command; require local object type `tag`, local/remote peeled commit `RC_SHA`, and never delete, move, or recreate it:

  ```powershell
  pwsh -NoProfile -File scripts/assert-release-policy.ps1 -EvidencePath $env:BANANA_RELEASE_POLICY_EVIDENCE_PATH -Phase PreTag -ExpectedTag v1.0.0
  if ($LASTEXITCODE -ne 0) { throw 'tag preflight failed' }
  $preexistingRunIds = @(gh run list --workflow release.yml --event push --branch v1.0.0 --limit 100 --json databaseId --jq '.[].databaseId')
  if ($LASTEXITCODE -ne 0) { throw 'cannot snapshot pre-tag workflow run IDs' }
  git tag -a v1.0.0 $RC_SHA -m "Banana Box v1.0.0"
  if ($LASTEXITCODE -ne 0 -or (git cat-file -t refs/tags/v1.0.0).Trim() -ne 'tag' -or (git rev-parse 'refs/tags/v1.0.0^{}').Trim() -ne $RC_SHA) { throw 'annotated tag creation failed' }
  git push origin refs/tags/v1.0.0
  if ($LASTEXITCODE -ne 0) { throw 'release tag push failed' }
  ```

  Define `Assert-RemoteReleaseTag` to require exactly one tag object ref plus one peeled ref and peeled SHA `RC_SHA`. Poll workflow runs using tag branch `v1.0.0`, event `push`, `headSha=RC_SHA`, `headBranch=v1.0.0`, and workflow `release.yml`; accept one unique run ID absent from `$preexistingRunIds`. Query the versioned workflow-run API and require original `actor.login` plus every rerun `triggering_actor.login` match the Task 0 publisher/explicitly authorized actor. Record the same `RUN_ID` but separate `artifactOriginAttempt` (successful signed build/provenance/Actions artifact) and `draftFinalAttempt` (eventual successful create-draft); they may differ after “Re-run failed jobs”. Watch the final attempt to success and bind review history to the environment job in the origin attempt. Require approval by exactly the recorded different reviewer and none by publisher. Bind Draft release ID/tag/draft/author/notes, six asset times across the validated attempt lineage, frozen artifact ID/digest, and provenance `RUN_ID`/`artifactOriginAttempt`/`RC_SHA`; never require provenance attempt to equal `draftFinalAttempt`. Record Release `created_at` but do not compare it with run start because GitHub derives it from tagged commit date. `targetCommitish` is never identity evidence.

- [ ] Download the actual CI Draft into a fresh directory and build one canonical external Approval manifest that binds release ID, fixed title `Banana Box v1.0.0`, `draft=true`, `prerelease=false`, workflow `RUN_ID`/head SHA/tag, distinct `artifactOriginAttempt` and `draftFinalAttempt`, Actions artifact ID/digest, reviewer, notes SHA, and exact six assets. For each asset combine GitHub asset ID/name/size/digest/uploader/created time with downloaded length/SHA-256; reject any missing, duplicate, or extra row and any API/local digest mismatch. Compute/display `APPROVAL_MANIFEST_SHA256`; Approval B text cites the full hash. Pass metadata to `test-updater-protocol.ps1 -ReleaseAssetMetadataPath` and run all three modes with fresh evidence.

  Every CI harness call supplies `-ExpectedBuildKind GitHubActions`. Repeat the same route/applicability matrix against these **downloaded CI bytes**: journey 1 migration on the two upgrade routes, its fresh-install variant on the two fresh routes, and journeys 2–5 on all four. Local artifacts are not substitutes. Do not claim untouched production v0.2.2 discovered a private Draft.

- [ ] Run `pwsh -NoProfile -File scripts/assert-release-policy.ps1 -EvidencePath $env:BANANA_RELEASE_POLICY_EVIDENCE_PATH -Phase ApprovalB` and require exit `0`; this also proves immutable releases remain disabled for the confirmed rollback strategy. Present the exact run/reviewer/Draft/release/body/assets, CI four-route results, signature/harness evidence, full `APPROVAL_MANIFEST_SHA256`, and residual production `/latest` risk. Stop for **Approval B: publish exactly the manifest identified by that hash, then immediately run the official-v0.2.2 production canary**.

- [ ] Only after Approval B, first recompute the canonical Approval manifest hash and require exact equality with the user-cited `APPROVAL_MANIFEST_SHA256`; then run `pwsh -NoProfile -File scripts/assert-release-policy.ps1 -EvidencePath $env:BANANA_RELEASE_POLICY_EVIDENCE_PATH -Phase PrePublish` and require exit `0`. Re-read the same release ID, title, `draft=true`, `prerelease=false`, tag, body, six GitHub asset IDs/metadata, remote peeled tag, and freshly downloaded hashes. Re-run `ValidateOnly` with `-ReleaseAssetMetadataPath` and compare every row with the still-hash-locked manifest before `gh release edit v1.0.0 --draft=false --latest`; any difference stops publication.

  Immediately after publish, re-read the same public release ID/title/tag with `draft=false` and `prerelease=false`; GET `/releases/latest` and require that same release ID. Assert the remote peeled tag again, download all assets into a third fresh directory, and compare the exact API metadata plus local SHA-256 matrix with Approval B. Run `ValidateOnly -ReleaseAssetMetadataPath` once more and verify the public release-notes hash. Only this post-publish evidence supports “published bytes equal approved bytes”; because immutable releases are deliberately disabled, do not claim they can never change later.

- [ ] Run Gstack `canary` immediately. Freshly download the official v0.2.2 NSIS and MSI, require both match `V022_ASSET_FINGERPRINT`, install each in its own clean profile, and invoke the untouched production updater through the real `/releases/latest/download/latest.json`. Require discovery, HTTP success, signature verification, install, relaunch, v1.0.0 display, data preservation, and all five journeys for both paths. Validate the public release page and recheck public six-asset hashes at the end of the canary window.

- [ ] If canary updater/install/migration fails while immutable releases remain disabled, return the same release to Draft/remove `latest.json` to stop discovery, preserve the inspected artifacts for diagnosis, and never replace a binary or retarget the protected `v1.0.0` tag. A fix uses `v1.0.1`. If immutable policy changed, stop instead of assuming this rollback is available.

- [ ] When canary is clean, update the two tracked QA documents with release ID, public body/asset hashes, official v0.2.2 baseline verification, both real production updater routes, five-journey results, timestamps, and final policy status. Create a fresh `codex/v1-release-evidence` branch from current `origin/main`, commit only `docs/qa/**`, push, and open a separate reviewed docs-only PR; do not change the existing tag, Release, or assets. The final evidence is not complete while it exists only in a local file.

## Unified Release Definition Of Done

- [ ] The formal Git clone, branch, origin, CI, and release workflow are healthy.
- [ ] All four subplans and all cross-plan contract tests pass from one commit.
- [ ] v0.2.2 upgrade preserves actual prompt/favorite/order/settings semantics and leaves no plaintext Key in files/backups/logs.
- [ ] Fresh install, crash recovery, full backup/restore, and signed updater paths pass.
- [ ] The five release-blocking user journeys pass at required Windows DPI/monitor conditions.
- [ ] CSP/capabilities/Markdown/archive/credential security gates pass.
- [ ] Gstack browse, qa, design-review, and post-release canary have no P0/P1 findings.
- [ ] All version files and updater artifacts say `1.0.0` and match the recorded release candidate SHA.
- [ ] Approval A authorized only tag/CI Draft generation; separate Approval B authorized publication of the inspected CI assets.
