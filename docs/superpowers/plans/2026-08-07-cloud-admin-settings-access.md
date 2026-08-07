# Cloud Administrator Settings Access Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ensure cloud-collaboration configuration is visible and writable only to an authenticated cloud administrator, while non-admin and logged-out users cannot see or invoke it.

**Architecture:** Supabase becomes the source of truth for the administrator claim: a JWT custom claim `app_metadata.cloud_admin = true` is issued only by a server-side/admin workflow. The frontend derives `isCloudAdmin` solely from the refreshed authenticated session, and every privileged Tauri command receives the access token, verifies its signed JWT against the configured Supabase JWKS and issuer/audience, then requires that claim before reading or writing administrator-only configuration. The local database is not a trust source.

**Tech Stack:** Supabase Auth/JWKS, Tauri 2/Rust, `jsonwebtoken` + `reqwest`, Vue 3/Pinia, Vitest, Rust tests.

## Global Constraints

- The sole visibility predicate is `auth.user !== null && auth.isCloudAdmin === true`.
- `isCloudAdmin` must no longer be inferred from the special email `000001@banana-box.local`; it must read only the verified session claim `app_metadata.cloud_admin === true`.
- The bootstrap user is promoted once using a Supabase service-role/admin procedure outside the client repository; do not ship a service-role key in the app.
- Privileged IPC requires a non-empty Supabase access token and rejects absent, expired, invalid-signature, wrong issuer/audience, or non-admin tokens with exactly `CLOUD_ADMIN_REQUIRED`.
- `load_cloud_config`, `save_cloud_config`, and a new `get_cloud_setup_sql` command are administrator-only. `load_cloud_runtime_config` remains available to ordinary users because normal login/collaboration needs its public client settings.
- The frontend must never request, render, copy, or cache cloud administrator configuration for logged-out/non-admin users.
- Existing default Supabase URL and anon key remain public configuration, but changing them is privileged.

---

## File Structure

- Create: `src-tauri/src/cloud_admin_auth.rs` — JWT claim DTO, JWKS fetch/cache, and `require_cloud_admin` verifier.
- Modify: `src-tauri/src/commands.rs` — authenticated command arguments and authorization gate.
- Modify: `src-tauri/src/lib.rs`, `src-tauri/Cargo.toml` — module/commands/dependency registration.
- Modify: `src/stores/auth.ts` — claim-backed `isCloudAdmin` and token accessor.
- Modify: `src/lib/ipc.ts`, `src/stores/cloudSession.ts` — attach token only to privileged invokes and handle stable denial.
- Modify: `src/components/SettingsModal.vue` — exact visibility predicate; move SQL through authorized IPC rather than a module-level string.
- Modify: `tests/components/SettingsModal.test.ts`; add Rust authorization tests.
- Create: `docs/cloud-admin.md` — one-time Supabase promotion/runbook without secrets.

### Task 1: Establish the server-issued administrator claim and frontend derivation

**Files:**
- Modify: `src/stores/auth.ts`
- Modify: `tests/components/SettingsModal.test.ts`
- Create: `docs/cloud-admin.md`

**Interfaces:**

```ts
type CloudAdminClaims = { app_metadata?: { cloud_admin?: boolean } }
getters: { isCloudAdmin(state): boolean { return state.user?.app_metadata?.cloud_admin === true } }
async function getAccessToken(): Promise<string | null> { return this.session?.access_token ?? null }
```

The runbook grants the first admin via Supabase Dashboard or service-role script: set the user’s `app_metadata` to `{ "cloud_admin": true }`, then force sign-out/sign-in so Supabase issues a fresh access token.

- [ ] **Step 1: Write failing user-state tests**

```ts
it('hides cloud settings when logged out', () => expect(mount(SettingsModal).find('.cloud-config-section').exists()).toBe(false))
it('hides cloud settings for signed-in users without cloud_admin claim', () => expect(sectionFor({ app_metadata: {} })).toBe(false))
it('shows cloud settings only for cloud_admin claim', () => expect(sectionFor({ app_metadata: { cloud_admin: true } })).toBe(true))
```

- [ ] **Step 2: Run tests to prove the current email rule fails**

Run: `pnpm vitest run tests/components/SettingsModal.test.ts`

Expected: FAIL for logged-out visibility and for claim-backed administrator detection.

- [ ] **Step 3: Implement claim-backed frontend state and exact visual predicate**

Replace the test-email getter. Change `canManageCloudSettings()` to `return auth.user !== null && auth.isCloudAdmin === true`. Ensure `onMounted` only calls `loadCloudSettings()` when this predicate is true and watches auth changes to clear in-memory URL/key/status values when it becomes false.

- [ ] **Step 4: Re-run frontend tests**

Run: `pnpm vitest run tests/components/SettingsModal.test.ts src/stores/auth.test.ts`

Expected: PASS; logged-out, normal signed-in, and `cloud_admin: true` test states match the three required outcomes.

- [ ] **Step 5: Commit the visible-access rule and runbook**

```bash
git add src/stores/auth.ts src/components/SettingsModal.vue tests/components/SettingsModal.test.ts docs/cloud-admin.md
git commit -m "feat: restrict cloud settings to cloud administrators"
```

### Task 2: Add cryptographically verified backend administrator authorization

**Files:**
- Create: `src-tauri/src/cloud_admin_auth.rs`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/cloud_admin_auth.rs` inline `#[cfg(test)]` module

**Interfaces:**

```rust
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CloudAdminAccess { pub access_token: String }
pub async fn require_cloud_admin(app: &tauri::AppHandle, access: &CloudAdminAccess) -> Result<(), String>;
// returns Err("CLOUD_ADMIN_REQUIRED") for every authentication/claim failure.
```

`require_cloud_admin` obtains the configured Supabase URL from the local runtime configuration, fetches `<supabase-url>/auth/v1/.well-known/jwks.json` over HTTPS, caches parsed public keys by `kid` with a bounded TTL, verifies JWT algorithm/issuer/audience/expiry/signature, then requires `claims.app_metadata.cloud_admin == true`.

- [ ] **Step 1: Write failing verifier tests with generated signing keys/JWKS fixture**

```rust
#[tokio::test] async fn missing_token_returns_stable_denial() { assert_eq!(require_cloud_admin(&app, &CloudAdminAccess { access_token: "".into() }).await, Err("CLOUD_ADMIN_REQUIRED".into())); }
#[tokio::test] async fn signed_non_admin_token_is_denied() { assert_eq!(verify_fixture(non_admin_jwt()).await, Err("CLOUD_ADMIN_REQUIRED".into())); }
#[tokio::test] async fn valid_admin_claim_is_accepted() { assert!(verify_fixture(admin_jwt()).await.is_ok()); }
```

- [ ] **Step 2: Run the test filter**

Run: `cargo test --manifest-path src-tauri/Cargo.toml cloud_admin_auth::tests -- --nocapture`

Expected: FAIL because verifier and dependencies are absent.

- [ ] **Step 3: Implement strict verifier and bounded JWKS cache**

Add `jsonwebtoken` with only required algorithm support; accept only the algorithms advertised by the configured Supabase JWKS, reject `alg: none`, and map all parsing/network/JWKS/token failures to `CLOUD_ADMIN_REQUIRED` externally while logging only non-secret diagnostic categories. Do not accept a frontend-provided email/boolean as authorization.

- [ ] **Step 4: Run authorization tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml cloud_admin_auth::tests -- --nocapture`

Expected: PASS for missing token, bad signature, expired token, normal user, and valid admin claim.

- [ ] **Step 5: Commit authorization primitive**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/cloud_admin_auth.rs src-tauri/src/lib.rs
git commit -m "feat: verify cloud administrator tokens in Rust"
```

### Task 3: Gate every privileged cloud configuration IPC command

**Files:**
- Modify: `src-tauri/src/commands.rs:63-102`
- Modify: `src/lib/ipc.ts`
- Modify: `src/stores/cloudSession.ts`
- Test: `src-tauri/src/commands.rs` inline tests; `tests/components/SettingsModal.test.ts`

**Interfaces:**

```rust
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CloudAdminCommandArgs { pub access_token: String }
#[tauri::command] pub async fn load_cloud_config(..., args: MainArgs<CloudAdminCommandArgs>) -> Result<CloudConfigDto, String>;
#[tauri::command] pub async fn save_cloud_config(..., args: MainArgs<SaveCloudConfigCommandArgs>) -> Result<CloudConfigDto, String>;
#[tauri::command] pub async fn get_cloud_setup_sql(..., args: MainArgs<CloudAdminCommandArgs>) -> Result<String, String>;
```

```ts
export async function loadCloudConfig(accessToken: string): Promise<CloudConfig>
export async function saveCloudConfig(input: SaveCloudConfigInput, accessToken: string): Promise<CloudConfig>
export async function getCloudSetupSql(accessToken: string): Promise<string>
```

- [ ] **Step 1: Write failing IPC authorization tests**

```rust
#[tokio::test] async fn save_cloud_config_rejects_non_admin() { assert_eq!(invoke_save(non_admin_token()).await.unwrap_err(), "CLOUD_ADMIN_REQUIRED"); }
#[tokio::test] async fn setup_sql_rejects_logged_out_caller() { assert_eq!(invoke_sql("").await.unwrap_err(), "CLOUD_ADMIN_REQUIRED"); }
```

- [ ] **Step 2: Run command tests before implementation**

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands::tests::cloud_admin -- --nocapture`

Expected: FAIL because current commands accept unauthenticated calls.

- [ ] **Step 3: Authorize before database work and move SQL generation behind the gate**

Call `require_cloud_admin(&app, &args.0.access)` before acquiring the database operation permit. Replace the SettingsModal-imported SQL concatenation with `get_cloud_setup_sql`; keep the SQL source in a backend module so IPC callers cannot retrieve it without authorization. Leave `load_cloud_runtime_config` unchanged and unauthenticated.

- [ ] **Step 4: Attach only a current token from Pinia**

Update `cloudSession.load/save` to request `auth.getAccessToken()`, reject locally with `CLOUD_ADMIN_REQUIRED` if absent, and pass the token in invoke args. On the same code return `CLOUD_ADMIN_REQUIRED`, clear cloud-admin form state and show the neutral message “需要云端管理员权限”.

- [ ] **Step 5: Run frontend/backend authorization suites**

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands::tests::cloud_admin cloud_admin_auth::tests -- --nocapture; pnpm vitest run tests/components/SettingsModal.test.ts`

Expected: PASS; no privileged command completes for a missing or ordinary user token, while verified admin calls succeed.

- [ ] **Step 6: Commit command authorization**

```bash
git add src-tauri/src/commands.rs src/lib/ipc.ts src/stores/cloudSession.ts src/components/SettingsModal.vue tests/components/SettingsModal.test.ts
git commit -m "fix: require verified admin for cloud configuration IPC"
```

### Task 4: Regression, upgrade, and documentation verification

**Files:**
- Modify: `docs/cloud-admin.md`
- Modify: `fabu.MD`

**Interfaces:**
- Consumes claim contract `app_metadata.cloud_admin === true` and `CLOUD_ADMIN_REQUIRED` error contract from Tasks 1–3.

- [ ] **Step 1: Add an upgrade test for the legacy account**

```ts
it('does not treat legacy 000001 email as admin without a claim', () => {
  expect(useAuthStore().isCloudAdmin).toBe(false)
})
```

- [ ] **Step 2: Run quality gates**

Run: `pnpm check && cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

- [ ] **Step 3: Manually validate all three personas**

Run: `pnpm tauri dev`

Expected: (1) logged out: no cloud section; (2) ordinary Supabase account: no cloud section and direct IPC yields `CLOUD_ADMIN_REQUIRED`; (3) promoted account after a fresh login: section appears, save and SQL copy work. Demote the test account and verify a refreshed session loses access.

- [ ] **Step 4: Document promotion and release checks**

Document the exact dashboard claim and re-login requirement, state explicitly that service-role keys stay outside the repository, and add the three-persona verification to `fabu.MD`.

- [ ] **Step 5: Commit release safeguards**

```bash
git add docs/cloud-admin.md fabu.MD tests/components/SettingsModal.test.ts
git commit -m "docs: add cloud administrator release checks"
```

## Self-Review

- Spec coverage: Task 1 fixes all frontend visibility cases; Task 2 defines a real trust boundary; Task 3 gates save/read/SQL commands behind that boundary; Task 4 covers legacy migration, three personas, and promotion documentation.
- Placeholder scan: JWT claim, error code, commands, arguments, endpoints, cache behaviour, and tests are explicitly specified.
- Type consistency: every privileged frontend call carries `accessToken`; every privileged Rust command receives `CloudAdminCommandArgs` and invokes `require_cloud_admin`.
