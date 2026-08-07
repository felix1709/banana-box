# PI / PI-Web Runtime On-Demand Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the 314 MiB PI-Web runtime from the Banana Box installer and let users deliberately download, verify, install, start, and update a signed runtime when they first need PI-Web.

**Architecture:** A separate public release repository, `felix1709/banana-box-pi-runtime`, publishes a versioned ZIP and signed manifest. Banana Box fetches and verifies the manifest, streams the ZIP with progress events, checks SHA-256 and signature, extracts to a new version directory, health-checks it, then atomically switches `current`; the previous healthy version remains usable. The Vue page only renders structured runtime state returned by Rust and never runs npm, npx, Node, or a terminal.

**Tech Stack:** Tauri 2, Rust (`reqwest`, `tokio`, `zip`, `sha2`, `minisign`), Vue 3, Pinia, Vitest, Rust unit/integration tests, GitHub Releases.

## Global Constraints

- Windows x64 is the initial supported runtime platform; no silent download occurs when a runtime is absent.
- Publish runtime assets only in `felix1709/banana-box-pi-runtime`; never add them to the latest release of `felix1709/banana-box`, which owns the app updater `latest.json`.
- The app embeds the runtime-manifest public key and verifies `pi-runtime.json.sig` before accepting URLs, checksums, or versions.
- Downloaded ZIP files must pass both SHA-256 from the verified manifest and the ZIP signature before extraction.
- Store managed copies at `app_data/pi-runtime/versions/<runtimeVersion>` and atomically record the healthy active version in `app_data/pi-runtime/current`.
- Preserve the previous healthy version on cancellation, download/hash/signature/extraction failure, or failed health check; do not delete it during an update.
- Do not fall back to `npx @agegr/pi-web@latest` for normal users; an independently detected local compatible service may be opened but is never silently adopted as the managed runtime.
- All progress is emitted as a structured Tauri event; no command window, API key, or sensitive value may be displayed or logged.
- The normal Banana Box build must not run `prepare:pi-web` or bundle `resources/pi-web-runtime.zip`.

---

## File Structure

- Create: `runtime-release/` in the new `felix1709/banana-box-pi-runtime` repository — reproducible pack, smoke test, manifest, signing, and release scripts.
- Create: `src-tauri/src/pi_runtime.rs` — manifest parsing, verification, download/install/update state machine, filesystem switching, and progress events.
- Modify: `src-tauri/src/pi_web.rs` — start PI-Web only from a verified managed runtime or a verified existing local service.
- Modify: `src-tauri/src/lib.rs` — register runtime module, service state, and IPC commands.
- Modify: `src-tauri/Cargo.toml` — add only the cryptographic verification dependency required by the implementation.
- Modify: `src-tauri/tauri.conf.json`, `package.json` — remove bundled-resource and build-script coupling.
- Modify: `src/lib/piWebIpc.ts`, `src/components/piweb/PiWebPage.vue` — expose/render the runtime lifecycle and progress states.
- Create: `tests/components/PiWebPage.runtime.test.ts` and Rust tests in `src-tauri/src/pi_runtime.rs` — cover state and failure paths.

### Task 1: Create the separately releasable, signed runtime artifact

**Files:**
- Create: external repository `felix1709/banana-box-pi-runtime/runtime-release/package-runtime.mjs`
- Create: external repository `felix1709/banana-box-pi-runtime/runtime-release/generate-manifest.mjs`
- Create: external repository `felix1709/banana-box-pi-runtime/runtime-release/smoke-test.mjs`
- Create: external repository `felix1709/banana-box-pi-runtime/README.md`

**Interfaces:**
- Produces `pi-runtime-win-x64-<runtimeVersion>.zip`, `<zip>.sig`, `pi-runtime.json`, and `pi-runtime.json.sig`.
- `pi-runtime.json` is exactly:

```json
{"schemaVersion":1,"runtimeVersion":"2026.08.07.1","platform":"win-x64","piVersion":"<pinned>","piWebVersion":"<pinned>","sizeBytes":0,"sha256":"<64 lowercase hex>","url":"https://github.com/felix1709/banana-box-pi-runtime/releases/download/v2026.08.07.1/pi-runtime-win-x64-2026.08.07.1.zip","signatureUrl":"<zip>.sig","publishedAt":"2026-08-07T00:00:00Z"}
```

- [ ] **Step 1: Write a failing artifact-layout smoke test**

```js
assert.ok(existsSync(join(stage, 'node.exe')))
assert.ok(existsSync(join(stage, 'node_modules', '@agegr', 'pi-web', 'dist', 'index.js')))
assert.equal(await fetch(`http://127.0.0.1:${port}`).then(r => r.ok), true)
```

- [ ] **Step 2: Run it before packaging**

Run: `node runtime-release/smoke-test.mjs --stage .runtime-stage`

Expected: FAIL because the stage directory does not yet exist.

- [ ] **Step 3: Implement reproducible packaging and signing**

```js
// package-runtime.mjs: install only pinned package-lock dependencies, copy Node + PI-Web,
// remove npm cache/docs/maps, create zip, then call minisign -Sm <zip> -s $env:PI_RUNTIME_SIGNING_KEY.
// generate-manifest.mjs: calculate bytes and SHA-256, write JSON, then sign it with the same release key.
```

Pin Node, `@mariozechner/pi`, and `@agegr/pi-web` versions in the external repository lockfile. `smoke-test.mjs` must start the staged `node.exe` and PI-Web script with port `30141`, wait for an HTTP response, then terminate the child.

- [ ] **Step 4: Re-run artifact smoke test and inspect output**

Run: `pnpm runtime:package && pnpm runtime:smoke && pnpm runtime:manifest`

Expected: PASS; all four release files exist, the manifest hash/size match the ZIP, and both signatures verify with the public key.

- [ ] **Step 5: Commit the runtime-release repository**

```bash
git add runtime-release README.md package.json pnpm-lock.yaml
git commit -m "feat: publish signed PI runtime artifacts"
```

### Task 2: Define and test the Rust runtime state machine

**Files:**
- Create: `src-tauri/src/pi_runtime.rs`
- Modify: `src-tauri/Cargo.toml`
- Test: `src-tauri/src/pi_runtime.rs` inline `#[cfg(test)]` module

**Interfaces:**
- Produces:

```rust
#[derive(Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PiRuntimeStatus { pub state: PiRuntimeState, pub installed_version: Option<String>, pub available_version: Option<String>, pub message: String, pub detail: String, pub can_download: bool, pub can_cancel: bool, pub can_update: bool }
pub enum PiRuntimeState { Checking, NotInstalled, Downloading, Verifying, Installing, Ready, UpdateAvailable, Error }
pub struct PiRuntimeProgress { pub phase: PiRuntimeState, pub downloaded_bytes: u64, pub total_bytes: Option<u64>, pub bytes_per_second: u64, pub message: String }
pub const PI_RUNTIME_PROGRESS_EVENT: &str = "pi-runtime-progress";
pub async fn get_runtime_status(app: &AppHandle, service: State<'_, PiRuntimeService>) -> Result<PiRuntimeStatus, String>;
pub async fn install_or_update_runtime(app: AppHandle, service: State<'_, PiRuntimeService>) -> Result<PiRuntimeStatus, String>;
pub fn cancel_runtime_download(service: State<PiRuntimeService>) -> Result<(), String>;
```

- [ ] **Step 1: Write failing tests for trust and switching rules**

```rust
#[test] fn rejects_manifest_with_bad_signature() { assert_eq!(verify_manifest("{}", "bad"), Err("PI_RUNTIME_MANIFEST_SIGNATURE_INVALID".into())); }
#[test] fn rejects_zip_with_wrong_sha256() { assert_eq!(verify_archive_hash(&archive, "00"), Err("PI_RUNTIME_ARCHIVE_HASH_MISMATCH".into())); }
#[test] fn failed_candidate_never_changes_current() { install_candidate_that_fails_healthcheck(&root); assert_eq!(read_current(&root), Some("old".into())); }
```

- [ ] **Step 2: Run the new tests to prove they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml pi_runtime::tests -- --nocapture`

Expected: FAIL because the module/functions do not exist.

- [ ] **Step 3: Implement manifest verification, safe extraction, and atomic activation**

```rust
// Fetch manifest and .sig with fixed HTTPS URLs; verify minisign using EMBEDDED_RUNTIME_PUBLIC_KEY.
// Stream ZIP into app_data/pi-runtime/downloads/<version>.part, emitting PiRuntimeProgress.
// Hash + verify ZIP signature; extract only enclosed paths to versions/<version>.tmp.
// Smoke-start node + PI-Web; rename tmp to versions/<version>; atomically write current.tmp then rename to current.
```

Reject non-HTTPS artifact URLs, directory traversal ZIP entries, duplicate install requests, invalid version strings, and archive sizes that differ from the verified manifest. Cancellation removes only `.part` and `.tmp` material.

- [ ] **Step 4: Run Rust runtime tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml pi_runtime::tests -- --nocapture`

Expected: PASS, including bad manifest, bad ZIP, cancellation cleanup, fresh install, and failed-update rollback cases.

- [ ] **Step 5: Commit this isolated backend foundation**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/pi_runtime.rs
git commit -m "feat: add verified PI runtime installer"
```

### Task 3: Make PI-Web launch consume only verified runtime paths

**Files:**
- Modify: `src-tauri/src/pi_web.rs:16-20,376-620`
- Modify: `src-tauri/src/lib.rs:1-190`
- Test: `src-tauri/src/pi_web.rs` inline tests

**Interfaces:**
- Consumes `PiRuntimeService::managed_launch_paths() -> Result<Option<(PathBuf, PathBuf)>, String>`.
- Changes `start_pi_web` so `missingRuntime` means `PiRuntimeState::NotInstalled` or a failed managed-runtime check, not a missing global Node/npm installation.

- [ ] **Step 1: Write failing launch-selection tests**

```rust
#[test] fn managed_current_runtime_is_preferred() { assert_eq!(launch_source(&fixture_with_current()), LaunchSource::Managed); }
#[test] fn npx_is_never_used_when_runtime_is_absent() { assert!(!build_launch_command(&fixture_without_runtime()).unwrap_err().contains("npx")); }
#[test] fn already_healthy_local_service_can_be_opened() { assert!(status_for_healthy_loopback().can_open); }
```

- [ ] **Step 2: Run launch-selection tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml pi_web::tests -- --nocapture`

Expected: FAIL because legacy bundled/npx behaviour remains.

- [ ] **Step 3: Replace bundled-resource launch logic**

Delete `PI_WEB_ARCHIVE`, `bundled_archive_path`, `extract_bundled_runtime`, and `ensure_bundled_pi_web_launch`. Resolve Node and script only from `PiRuntimeService` after its completeness marker and health check pass. Preserve the loopback service probe before declaring the runtime unavailable; remove all npm/npx install links from the normal status path.

- [ ] **Step 4: Register and exercise public commands**

Register `get_pi_runtime_status`, `install_or_update_pi_runtime`, and `cancel_pi_runtime_download` in `src-tauri/src/lib.rs`; initialise `PiRuntimeService` once with `PiWebService`. Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml pi_web::tests pi_runtime::tests -- --nocapture
```

Expected: PASS; neither test source nor compiled launch code references `npx @agegr/pi-web@latest`.

- [ ] **Step 5: Commit launcher migration**

```bash
git add src-tauri/src/pi_web.rs src-tauri/src/lib.rs
git commit -m "refactor: launch PI-Web from managed runtime"
```

### Task 4: Remove the runtime from normal application packaging

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `package.json`
- Delete: `src-tauri/resources/pi-web-runtime.zip`
- Move: `scripts/prepare-pi-web-runtime.mjs` to the external runtime-release repository, then delete it here
- Test: `scripts/check-installer-contents.mjs`

**Interfaces:**
- Produces `pnpm check:installer-contents`, which exits non-zero when an installer contains `pi-web-runtime.zip` or an unpacked PI runtime.

- [ ] **Step 1: Write the failing installer-content check**

```js
assert.equal(config.bundle.resources?.includes('resources/pi-web-runtime.zip'), false)
assert.equal(config.build.beforeBuildCommand.includes('prepare:pi-web'), false)
assert.equal(installerListing.includes('pi-web-runtime.zip'), false)
```

- [ ] **Step 2: Run it against current configuration**

Run: `node scripts/check-installer-contents.mjs --config-only`

Expected: FAIL because both resource and pre-build command are present.

- [ ] **Step 3: Remove the bundle coupling**

Set `beforeBuildCommand` to `pnpm build`, remove `bundle.resources`, remove `prepare:pi-web`, and delete only the explicitly named ZIP after confirming its path is `C:\Users\admin\banana-box-main\src-tauri\resources\pi-web-runtime.zip`.

- [ ] **Step 4: Verify a fresh installer**

Run: `pnpm check && pnpm tauri build && pnpm check:installer-contents -- --installer target/release/bundle/nsis/*.exe`

Expected: PASS; the installer listing has no runtime ZIP and its size is within the 6–10 MiB target range (record the exact size in release notes).

- [ ] **Step 5: Commit packaging reduction**

```bash
git add package.json src-tauri/tauri.conf.json scripts/check-installer-contents.mjs
git rm src-tauri/resources/pi-web-runtime.zip scripts/prepare-pi-web-runtime.mjs
git commit -m "build: remove PI runtime from installer"
```

### Task 5: Render download, update, and recovery UX in PI-Web

**Files:**
- Modify: `src/lib/piWebIpc.ts`
- Modify: `src/components/piweb/PiWebPage.vue`
- Create: `tests/components/PiWebPage.runtime.test.ts`

**Interfaces:**
- Consumes the Task 2 command/event DTOs.
- Adds `getPiRuntimeStatus()`, `installOrUpdatePiRuntime()`, `cancelPiRuntimeDownload()`, and `listenPiRuntimeProgress(callback)` to `src/lib/piWebIpc.ts`.

- [ ] **Step 1: Write failing component tests for each user-visible state**

```ts
it('does not start an absent runtime automatically', async () => expect(wrapper.get('[data-action="download-pi-runtime"]').text()).toContain('下载并启动 PI-Web'))
it('shows downloaded bytes, total bytes, speed and cancellation while downloading', async () => expect(wrapper.text()).toContain('12 MiB / 314 MiB'))
it('keeps the old version usable after update failure', async () => expect(wrapper.text()).toContain('继续使用 2026.08.01.1'))
```

- [ ] **Step 2: Run component tests to verify failure**

Run: `pnpm vitest run tests/components/PiWebPage.runtime.test.ts`

Expected: FAIL because runtime IPC/state cards do not exist.

- [ ] **Step 3: Implement the compact state card and event lifecycle**

On mount fetch `getPiRuntimeStatus()` and subscribe once to `pi-runtime-progress`; always call the returned unlisten function on unmount. Show exactly one primary action per state: explicit download, cancel, retry, start/open, or update. Disable duplicate actions while busy, announce status through `aria-live`, and retain the ready version/action after an update error.

- [ ] **Step 4: Run browser-visible and unit verification**

Run: `pnpm vitest run tests/components/PiWebPage.runtime.test.ts tests/components/PiWebPage.test.ts && pnpm check`

Expected: PASS. In a clean AppData test profile, manually confirm: first click shows download CTA (no automatic transfer), progress updates, cancel works, install starts PI-Web, and a deliberately invalid update retains the prior runtime.

- [ ] **Step 5: Commit the user flow**

```bash
git add src/lib/piWebIpc.ts src/components/piweb/PiWebPage.vue tests/components/PiWebPage.runtime.test.ts
git commit -m "feat: download and update PI runtime on demand"
```

### Task 6: Release and regression verification

**Files:**
- Modify: `README.md` or `docs/pi-web.md` — user-facing first-run/download/update explanation.
- Modify: `fabu.MD` — add runtime-repository and clean-profile checks to the release checklist.

**Interfaces:**
- Consumes the published `pi-runtime.json` from the separate repository and the standard Banana Box updater manifest from the application repository.

- [ ] **Step 1: Write release checklist assertions before release**

```text
- Banana Box release contains latest.json and no PI runtime asset.
- PI runtime release contains ZIP, ZIP signature, manifest, and manifest signature.
- Clean profile completes download → verify → start.
- Update failure preserves current runtime and startup succeeds.
```

- [ ] **Step 2: Run all automated verification**

Run: `pnpm check; cargo test --manifest-path src-tauri/Cargo.toml; pnpm tauri build; pnpm release:manifest`

Expected: all commands pass; installer inspection confirms the expected reduced size.

- [ ] **Step 3: Perform the two manual release cases**

Run in a Windows profile with no Node and no PI: install the new app, download/start runtime, close/reopen offline, then publish a deliberately unhealthy test runtime and confirm rollback. Record runtime/app versions and installer size in the release notes.

- [ ] **Step 4: Commit release documentation**

```bash
git add README.md docs/pi-web.md fabu.MD
git commit -m "docs: document PI runtime release checks"
```

## Self-Review

- Spec coverage: Tasks 1–2 cover isolated source, signatures, hashes, progress, cancellation, extraction, health check, atomic activation, and rollback; Task 3 covers first-use detection/launch; Task 4 removes installer payload; Task 5 covers all specified front-end states; Task 6 covers release verification.
- Completeness scan: no unnamed follow-up work or undefined interfaces; external repository files are deliberately named and their expected release contract is fixed.
- Type consistency: the `PiRuntimeStatus`, `PiRuntimeProgress`, event name, and command names defined in Task 2 are the only names consumed by Tasks 3 and 5.
