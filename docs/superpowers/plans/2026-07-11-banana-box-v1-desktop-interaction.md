# Banana Box v1 Desktop Interaction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the 64x64 persistent floating banana, its reversible 12-frame/360ms panel animation, and the production-ready “蕉签场记” reminder window with weekday scheduling, snooze, lease/ACK fencing, unread recovery, and sleep catch-up.

**Architecture:** Keep three Tauri windows with narrow responsibilities: main owns the workspace, floatbtn owns banana animation and unread affordance, and reminder owns one compact actionable bubble. Rust is the authority for panel visibility, saved float position, reminder eligibility, durable reminder state, leases, and window placement; Vue renders only event payloads and reports fenced acknowledgements/actions. Reminder scheduling reads the shared daily-task database and reuses daily_tasks::navigation::navigate_to_daily_tasks rather than introducing a second navigation path.

**Tech Stack:** Vue 3, TypeScript, Vitest, CSS sprite animation, Tauri 2, Rust, rusqlite/SQLite, chrono, serde, Windows multi-window APIs.

---

## Scope And Execution Order

This is the desktop-interaction sub-plan for the approved design in docs/superpowers/specs/2026-07-11-banana-box-v1-design.md.

Tasks 1-9 can execute after the v1 security/database foundation is present. Tasks 10-11 additionally require the daily-task schema and navigation module. The following indented declarations are signature-only integration contracts, not implementation bodies:

    // Provided by the shared database foundation.
    pub struct Database {
        path: std::path::PathBuf,
        connection: std::sync::Mutex<rusqlite::Connection>,
    }
    impl Database {
        pub fn with_connection<T>(
            &self,
            operation: impl FnOnce(&rusqlite::Connection) -> Result<T, String>,
        ) -> Result<T, String>;
        pub fn with_immediate_transaction<T>(
            &self,
            operation: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T, String>,
        ) -> Result<T, String>;
    }

    // Provided by the daily-task implementation plan.
    pub async fn navigate_to_daily_tasks(
        app: &tauri::AppHandle,
        local_date: String,
    ) -> Result<(), String>;

The navigation function above shows and focuses main, selects the requested local date, and emits open-daily-tasks. This plan calls it; it must not emit a competing event or duplicate that behavior.

The scheduler depends only on these two stable daily-task queries:

    SELECT settled_at, report_snapshot
      FROM daily_task_days
     WHERE local_date = ?1;

    SELECT EXISTS(
      SELECT 1
        FROM daily_tasks AS task
        JOIN daily_task_groups AS task_group ON task_group.id = task.group_id
        JOIN daily_task_days AS task_day ON task_day.id = task_group.day_id
       WHERE task_day.local_date = ?1
    );

The shared foundation creates these tables; the production-management plan owns their data and navigation behavior. `SqliteDailyTaskSource` treats a missing day row as `has_tasks=false, settled=false, previously_settled=false` and never guesses task ownership from a nonexistent `daily_tasks.local_date` column. A non-null `report_snapshot` means the date was settled before; reopening clears `settled_at` but keeps that snapshot, so the scheduler suppresses new automatic phases for a reopened date.

## Locked IPC And Event Contract

Create src/types/desktop.ts as the single frontend contract:

    export type PanelTransitionReason =
      | 'banana'
      | 'tray'
      | 'shortcut'
      | 'fileDrop'
      | 'focusLoss'
      | 'titlebarClose'
      | 'reminderAction'
      | 'secondInstance'
      | 'startup'

    export interface PanelTargetChanged {
      generation: number
      targetVisible: boolean
      reason: PanelTransitionReason
      revealAtFrame: 6
    }

    export interface PanelVisibilityChanged {
      generation: number
      visible: boolean
    }

    export interface PanelRevealAck {
      generation: number
      frame: number
    }

    export interface PanelStateSnapshot {
      generation: number
      desiredVisible: boolean
      actualVisible: boolean
    }

    export type ReminderKind = 'dailyTasks'
    export type ReminderPhase = 'initial' | 'snooze'
    export type ReminderAction = 'settle' | 'snooze' | 'dismiss'
    export type ReminderSide = 'left' | 'right'

    export interface ReminderClaimRef {
      kind: ReminderKind
      localDate: string
      phase: ReminderPhase
      deliveryId: string
      attemptToken: string
      ownerId: string
      fence: number
    }

    export interface ReminderPlacement {
      side: ReminderSide
      tailOffsetPx: number
    }

    export interface ReminderPreparePayload {
      claim: ReminderClaimRef
      title: string
      body: string
      timestamp: string
      actions: ReminderAction[]
      severity: 'info' | 'warning'
    }

    export interface ReminderShownPayload {
      claim: ReminderClaimRef
    }

    export interface ReminderAttentionPayload {
      claim: ReminderClaimRef
      durationMs: 220
    }

    export interface ReminderUnreadChanged {
      unread: boolean
      revision: number
    }

    export interface ReminderUnreadState {
      unread: boolean
      revision: number
    }

    export interface ActivateFloatButtonResult {
      action: 'panelToggleRequested' | 'unreadReminderReopened' | 'reminderPriorityInFlight'
    }

    export interface ReminderMutationResult {
      accepted: true
      replayed: boolean
      uiSyncWarning: boolean
    }

Create the matching Rust contract in `src-tauri/src/reminder/mod.rs`; unit-test JSON round trips against the TypeScript literals above:

    #[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub enum ReminderKind { DailyTasks }

    #[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub enum ReminderPhase { Initial, Snooze }

    #[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub enum ReminderAction { Settle, Snooze, Dismiss }

    #[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub enum ReminderSide { Left, Right }

    #[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    pub struct ReminderClaimRef {
        pub kind: ReminderKind,
        pub local_date: String,
        pub phase: ReminderPhase,
        pub delivery_id: String,
        pub attempt_token: String,
        pub owner_id: String,
        pub fence: i64,
    }

    #[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub struct ReminderPlacement { pub side: ReminderSide, pub tail_offset_px: i32 }

    #[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub struct ReminderMutationResult {
        pub accepted: bool,
        pub replayed: bool,
        pub ui_sync_warning: bool,
    }

    #[derive(Clone, Copy, Debug, serde::Serialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub enum ReminderSeverity { Info, Warning }

    #[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub struct ReminderPreparePayload {
        pub claim: ReminderClaimRef,
        pub title: String,
        pub body: String,
        pub timestamp: String,
        pub actions: Vec<ReminderAction>,
        pub severity: ReminderSeverity,
    }

    #[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub struct ReminderShownPayload { pub claim: ReminderClaimRef }

    #[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub struct ReminderAttentionPayload {
        pub claim: ReminderClaimRef,
        pub duration_ms: u16,
    }

    #[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub struct ReminderUnreadChanged { pub unread: bool, pub revision: u64 }

    pub type ReminderUnreadState = ReminderUnreadChanged;

    #[derive(Clone, Copy, Debug, serde::Serialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub enum ActivateFloatButtonAction {
        PanelToggleRequested,
        UnreadReminderReopened,
        ReminderPriorityInFlight,
    }

    #[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub struct ActivateFloatButtonResult { pub action: ActivateFloatButtonAction }

`ReminderMutationResult::accepted` is always `true` on a returned value; `replayed=true` is allowed only for an exact idempotent shown-ACK replay, and pre-commit rejection remains `Err(String)`. No second ad hoc payload shape is allowed. JSON round-trip tests assert all three activation literals, including `reminderPriorityInFlight`.

Custom IPC commands:

- get_panel_state
- get_reminder_unread_state
- activate_float_button
- ack_reminder_attention
- ack_panel_reveal
- show_panel
- prepare_reminder_layout
- show_prepared_reminder
- ack_reminder_rendered
- ack_reminder_exit
- mark_reminder_auto_hidden
- complete_reminder_action
- debug_set_reminder_now, debug builds only

Every command uses exactly one foundation authorized whole-payload envelope before touching state: `get_panel_state`, `get_reminder_unread_state`, `activate_float_button`, `ack_reminder_attention`, and `ack_panel_reveal` use `FloatArgs`; `show_panel` uses `MainOrFloatArgs`; all fenced reminder commands, including `ack_reminder_exit`, use `ReminderArgs`; `debug_set_reminder_now` uses `MainArgs`. Inner DTOs are camelCase/deny-unknown and preserve flat invoke payloads. A mismatched/unknown caller receives `FORBIDDEN_WINDOW` before malformed payload deserialization; an authorized malformed payload receives `INVALID_INPUT`. Core/plugin capability JSON does not replace this Rust check.

Pure panel/window-state commands (`get_panel_state`, `ack_panel_reveal`, `show_panel`) do not touch SQLite and are not placed behind restore maintenance. Every command that reads/mutates reminder rows (`get_reminder_unread_state`, `activate_float_button`, prepare/layout/show/ACK/hide/action, debug tick) performs caller authorization → Ready → `services.operations.enter_user()` and retains the user permit through its final DB/native reconciliation. The scheduler tick takes `services.operations.try_enter_background()` before any query/claim and skips cleanly if maintenance is pending. Every detached ACK deadline, timer, post-commit unread/hide reconciliation, and app-loop retry does the same at callback entry; if no background permit is available it performs zero repository/native/event work and relies on durable startup/next-tick reconciliation. Add command/callback maintenance tests and drain barriers; no reminder DB write or stale native callback may race a restore snapshot.

Tauri events:

- panel-target-changed, Rust to floatbtn
- panel-visibility-changed, Rust to floatbtn and main
- reminder-attention, Rust to floatbtn before reminder preparation
- reminder-prepare, Rust to hidden reminder WebView for content measurement
- reminder-show, Rust to reminder only after native show succeeds
- reminder-hide-request, Rust to reminder to begin the fenced 160/80ms exit
- reminder-hide, Rust to reminder only after matching native hide/fallback completes, for final Vue cleanup
- reminder-unread-changed, Rust to floatbtn
- open-daily-tasks, owned by daily_tasks::navigation
- floating-file-dropped, existing floatbtn to main event retained unchanged

Every reminder mutation includes ReminderClaimRef. Rust updates with kind + local_date + phase + delivery_id + attempt_token + owner_id + attempt_count/fence; zero affected rows returns the stable error code STALE_REMINDER_CLAIM and never changes current state.

`ReminderClaimRef.fence` is the camelCase payload name for the row's `attempt_count`; there is no second fence column. Each automatic reclaim increments it, while a user-initiated unread reopen rotates `delivery_id` and resets `attempt_count/fence` to `1`.

## File Map

Create:

- src/types/desktop.ts: shared panel/reminder payloads and stable IPC result types.
- src/lib/bananaAnimation.ts: pure 12-frame animation state machine.
- tests/lib/bananaAnimation.test.ts: duration, midpoint, reversal, stale tick, and reduced-motion tests.
- src/assets/banana/banana-peel-sprite.webp: approved 3072x256 horizontal sprite, frames 0-11.
- src/assets/banana/banana-open-approved.png: user-approved transparent endpoint source with recorded SHA-256.
- src/components/AnimatedBananaButton.vue: sprite renderer and requestAnimationFrame driver.
- tests/components/AnimatedBananaButton.test.ts: visual-frame and reversal component tests.
- src-tauri/tests/banana_assets.rs: sprite dimensions, alpha edge, and distinct endpoint validation.
- src-tauri/src/desktop_state.rs: atomic desktop-state.json persistence and 64x64 restore/clamp logic.
- src-tauri/src/window_state.rs: desired/actual panel state, generation fencing, events, and all show/hide entry points.
- src/lib/reminderTimer.ts: pausable 12-second active-time countdown.
- tests/lib/reminderTimer.test.ts: pause/resume/expire tests with fake time.
- src/components/ReminderWindow.vue: “蕉签场记” markup, focus-safe behavior, actions, and B-style CSS.
- tests/components/ReminderWindow.test.ts: render ACK, timer, focus, auto-hide, and action tests.
- src-tauri/capabilities/reminder.json: reminder-window minimum capability.
- src-tauri/src/reminder/mod.rs: public commands, Tauri window orchestration, unread reopen, and navigation action.
- src-tauri/src/reminder/geometry.rs: monitor-safe mirror/clamp/tail placement.
- src-tauri/src/reminder/repository.rs: shared reminder_log repository and fenced state transitions; it never owns schema migration.
- src-tauri/src/reminder/backup_validator.rs: read-only `reminder-v1` persisted-state semantic validator.
- src-tauri/src/reminder/eligibility.rs: shared DayStatus/fence and task-mutation eligibility rules.
- src-tauri/src/reminder/scheduler.rs: weekday 18:00 eligibility, initial/snooze claims, and wake catch-up loop.

Modify:

- src/components/FloatButton.vue: compose AnimatedBananaButton, listen for Rust state/unread events, preserve dragging/drop behavior.
- tests/components/FloatButton.test.ts: replace emoji assertions and cover state sync/unread priority.
- src/main.ts: mount ReminderWindow for label reminder.
- src-tauri/tauri.conf.json: 64x64 hidden-until-restored floatbtn and hidden reminder window.
- src-tauri/capabilities/main.json and floatbtn.json: keep the foundation-owned per-window permissions; reminder is added only to its own new capability.
- src-tauri/Cargo.toml: direct tokio time dependency if foundation has not already added it, plus tauri-plugin-single-instance; rusqlite comes from the foundation.
- src-tauri/src/lib.rs: register modules/state/commands, route tray/shortcut/drop/focus through WindowStateService, restore float position, and start/stop scheduler.

Do not modify daily_tasks::navigation or create another daily-task routing event in this plan.

### Task 0: Verify The Formal Workspace Baseline

**Files:** None.

- [ ] **Step 1: Confirm the branch and clean worktree**

Run:

    git status --short --branch

Expected:

    ## codex/v1-major-update

There must be no untracked or modified files except this approved plan when execution begins.

- [ ] **Step 2: Install the locked frontend dependencies**

Run:

    pnpm install --frozen-lockfile

Expected: exit code 0 and no lockfile changes.

- [ ] **Step 3: Run the current frontend checks**

Run:

    pnpm check

Expected: typecheck, ESLint, and all existing Vitest tests pass.

- [ ] **Step 4: Run the current Rust tests**

Run:

    & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml

Expected: exit code 0 with every existing Rust test passing.

### Task 1: Lock Types And The Reversible Banana State Machine

**Files:**

- Create: src/types/desktop.ts
- Create: src/lib/bananaAnimation.ts
- Create: tests/lib/bananaAnimation.test.ts

- [ ] **Step 1: Write failing animation tests**

Create tests/lib/bananaAnimation.test.ts:

    import { describe, expect, it } from 'vitest'
    import {
      BANANA_CLOSED_FRAME,
      BANANA_FRAME_COUNT,
      BANANA_OPEN_FRAME,
      BANANA_REVEAL_FRAME,
      BANANA_TOTAL_MS,
      frameAt,
      retarget,
    } from '@/lib/bananaAnimation'

    describe('banana animation state', () => {
      it('uses 12 frames over 360ms and reaches frame 6 at the reveal point', () => {
        expect(BANANA_FRAME_COUNT).toBe(12)
        expect(BANANA_TOTAL_MS).toBe(360)
        const state = retarget(null, BANANA_OPEN_FRAME, 0)
        expect(frameAt(state, 0)).toBe(BANANA_CLOSED_FRAME)
        expect(frameAt(state, state.revealAtMs)).toBe(BANANA_REVEAL_FRAME)
        expect(frameAt(state, 360)).toBe(BANANA_OPEN_FRAME)
      })

      it('reverses from the currently displayed frame without jumping', () => {
        const opening = retarget(null, BANANA_OPEN_FRAME, 0)
        const current = frameAt(opening, 180)
        const closing = retarget(opening, BANANA_CLOSED_FRAME, 180)
        expect(closing.startFrame).toBe(current)
        expect(frameAt(closing, 180)).toBe(current)
        expect(frameAt(closing, 540)).toBe(BANANA_CLOSED_FRAME)
      })

      it('collapses immediately to the target when reduced motion is active', () => {
        const state = retarget(null, BANANA_OPEN_FRAME, 10, true)
        expect(frameAt(state, 10)).toBe(BANANA_OPEN_FRAME)
      })
    })

- [ ] **Step 2: Run the focused test and confirm RED**

Run:

    pnpm test -- tests/lib/bananaAnimation.test.ts

Expected: FAIL because @/lib/bananaAnimation does not exist.

- [ ] **Step 3: Add the desktop event types**

Create src/types/desktop.ts with the exact contract in “Locked IPC And Event Contract”. Keep camelCase fields because Rust payloads use serde(rename_all = "camelCase").

- [ ] **Step 4: Implement the minimum animation state machine**

Create src/lib/bananaAnimation.ts:

    export const BANANA_FRAME_COUNT = 12
    export const BANANA_CLOSED_FRAME = 0
    export const BANANA_OPEN_FRAME = 11
    export const BANANA_REVEAL_FRAME = 6
    export const BANANA_TOTAL_MS = 360

    export interface BananaAnimationState {
      startFrame: number
      targetFrame: number
      startedAtMs: number
      durationMs: number
      revealAtMs: number
    }

    function clampFrame(frame: number) {
      return Math.min(BANANA_OPEN_FRAME, Math.max(BANANA_CLOSED_FRAME, Math.round(frame)))
    }

    export function frameAt(state: BananaAnimationState, nowMs: number) {
      if (state.durationMs === 0) return state.targetFrame
      const progress = Math.min(1, Math.max(0, (nowMs - state.startedAtMs) / state.durationMs))
      return clampFrame(
        state.startFrame + (state.targetFrame - state.startFrame) * progress,
      )
    }

    export function retarget(
      previous: BananaAnimationState | null,
      targetFrame: number,
      nowMs: number,
      reducedMotion = false,
    ): BananaAnimationState {
      const startFrame = previous ? frameAt(previous, nowMs) : BANANA_CLOSED_FRAME
      const target = clampFrame(targetFrame)
      const distance = Math.abs(target - startFrame)
      const durationMs = reducedMotion
        ? 0
        : Math.round((BANANA_TOTAL_MS * distance) / BANANA_OPEN_FRAME)
      return {
        startFrame,
        targetFrame: target,
        startedAtMs: nowMs,
        durationMs,
        revealAtMs: Math.round(
          (BANANA_TOTAL_MS * BANANA_REVEAL_FRAME) / BANANA_OPEN_FRAME,
        ),
      }
    }

- [ ] **Step 5: Run the focused test and confirm GREEN**

Run:

    pnpm test -- tests/lib/bananaAnimation.test.ts

Expected: 3 tests pass.

- [ ] **Step 6: Run typecheck and commit**

Run:

    pnpm typecheck
    git add src/types/desktop.ts src/lib/bananaAnimation.ts tests/lib/bananaAnimation.test.ts
    git commit -m "feat: define desktop interaction protocol"

Expected: typecheck exits 0 and the commit succeeds.

### Task 2: Produce And Render The Approved 12-Frame Banana

**Files:**

- Create: src/assets/banana/banana-open-approved.png (only after explicit visual approval)
- Create: docs/design/banana-open-approved.sha256 (approval record)
- Create: src/assets/banana/banana-peel-sprite.webp
- Create: src-tauri/tests/banana_assets.rs
- Create: src/components/AnimatedBananaButton.vue
- Create: tests/components/AnimatedBananaButton.test.ts

- [ ] **Step 1: Establish and approve the real open endpoint before generating frames**

The repository contains only the system `🍌` character in `FloatButton.vue`; it has no approved peeled-banana bitmap. Do not call that emoji a peeled endpoint and do not invent frame 11 silently. On the actual Windows 11 test machine, capture the current runtime glyph at 100% and 200% DPI while recording OS build, `Segoe UI Emoji` font version, browser/WebView version, and CSS/font rendering settings. Show that capture to the user as a **reference only** and ask whether it is the intended opened state.

If the user rejects it, use the image-generation skill to create three 256×256 transparent open peeled-banana endpoint candidates in the already approved Banana Box visual direction, then stop for explicit selection. If the user accepts the glyph reference, first verify redistribution rights; otherwise use image generation to create a visually matching original asset rather than embedding the system glyph. Only after explicit approval save the selected original bitmap as `banana-open-approved.png` and record its SHA-256 plus approval date/reference in `docs/design/banana-open-approved.sha256`. This approval is a hard gate: no sprite generation or asset GREEN claim proceeds without the file/hash.

- [ ] **Step 2: Write the failing binary-asset test**

Create src-tauri/tests/banana_assets.rs:

    use image::{GenericImageView, Pixel, RgbaImage};
    use std::path::PathBuf;

    #[test]
    fn banana_sprite_has_twelve_square_frames_and_transparent_edges() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../src/assets/banana/banana-peel-sprite.webp");
        let image = image::open(path).expect("banana sprite must exist");
        let approved_open = image::open(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../src/assets/banana/banana-open-approved.png"),
        ).expect("approved open endpoint must exist").to_rgba8();
        assert_eq!(image.dimensions(), (3072, 256));
        let frames: Vec<RgbaImage> = (0..12)
            .map(|frame| image.crop_imm(frame * 256, 0, 256, 256).to_rgba8())
            .collect();
        for (index, frame) in frames.iter().enumerate() {
            let bbox = alpha_bbox(frame).expect("frame must contain visible pixels");
            assert!(bbox.min_x >= 46 && bbox.max_x <= 209 && bbox.min_y >= 46 && bbox.max_y <= 209,
                "frame {index} must keep at least 18% transparent padding on all sides");
            assert_all_four_border_rows_and_columns_transparent(frame);
            assert_centroid_within(frame, &approved_open, 8.0);
        }
        for index in 0..11 {
            let changed_ratio = changed_pixel_ratio(&frames[index], &frames[index + 1]);
            assert!((0.005..=0.28).contains(&changed_ratio),
                "adjacent frame {index} change must be visible but bounded");
            assert_bbox_scale_delta_at_most(&frames[index], &frames[index + 1], 0.12);
        }
        assert_eq!(image.crop_imm(11 * 256, 0, 256, 256).to_rgba8(), approved_open);
    }

Implement the helpers over every decoded RGBA pixel, not a center-pixel sample. `changed_pixel_ratio` counts any lossless RGBA difference; centroid uses alpha weight. Also assert the WebP decoder reports lossless alpha and the approved endpoint SHA file matches before comparing pixels.

- [ ] **Step 3: Run the asset test and confirm RED**

Run:

    & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml --test banana_assets

Expected: FAIL with “banana sprite must exist”.

- [ ] **Step 4: Generate the approved sprite with the image generation skill**

Invoke the image-generation skill with this exact art direction:

    Create one horizontal 12-frame animation sprite sheet on a fully transparent
    background. Each cell is square and shows the same small polished banana mascot
    from the same camera, scale, center, lighting, outline weight, and shadow direction.
    Frame 0 is a whole unpeeled banana. Frames 1-10 progressively peel the same banana
    in physically coherent small increments. Frame 11 matches the approved open peeled
    Banana Box icon. No text, face, limbs, stickers, extra fruit, clouds, colored glow,
    panel dividers, or background. Preserve at least 18 percent transparent padding
    around every pose. The motion arc must read smoothly left to right.

Pass the SHA-verified `banana-open-approved.png` as the required endpoint reference. Normalize that approved endpoint itself into the locked 18%-padding/center convention before recording its final hash; use image editing so frame 11 is copied pixel-identically from that normalized asset, while frames 0-10 form the new closed-to-open motion. Encode the 3072×256 sprite as **lossless WebP with alpha preserved**; the decoded frame-11 pixels must equal the approved PNG exactly. Inspect it with view_image at original detail and verify the codec metadata; reject lossy output, identity drift, moving visual center, clipped peel, opaque borders, repeated adjacent frames, or a change ratio/bounding-box jump outside the test thresholds.

- [ ] **Step 5: Run the asset test and confirm GREEN**

Run:

    & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml --test banana_assets

Expected: 1 test passes.

- [ ] **Step 6: Write failing component tests**

Create tests/components/AnimatedBananaButton.test.ts:

    import { mount } from '@vue/test-utils'
    import { beforeEach, describe, expect, it, vi } from 'vitest'
    import AnimatedBananaButton from '@/components/AnimatedBananaButton.vue'

    describe('AnimatedBananaButton', () => {
      beforeEach(() => vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => {
        cb(performance.now() + 360)
        return 1
      }))

      it('renders the closed frame first and the open frame after retargeting', async () => {
        const wrapper = mount(AnimatedBananaButton, { props: { open: false } })
        expect(wrapper.attributes('data-frame')).toBe('0')
        await wrapper.setProps({ open: true })
        expect(wrapper.attributes('data-frame')).toBe('11')
      })

      it('exposes a stable 64px hit surface without changing the sprite bounds', () => {
        const wrapper = mount(AnimatedBananaButton, { props: { open: false } })
        expect(wrapper.classes()).toContain('animated-banana')
        expect(wrapper.find('.banana-sprite').exists()).toBe(true)
      })
    })

- [ ] **Step 7: Run component tests and confirm RED**

Run:

    pnpm test -- tests/components/AnimatedBananaButton.test.ts

Expected: FAIL because AnimatedBananaButton.vue does not exist.

- [ ] **Step 8: Implement the sprite renderer**

Create src/components/AnimatedBananaButton.vue. It must:

- accept open and unread Boolean props;
- call retarget whenever open changes;
- run requestAnimationFrame until the target frame is reached;
- set data-frame and CSS variable --banana-frame;
- declare `const emit = defineEmits<{ frame: [value: number] }>()` and emit `frame` after each rendered-frame change so Rust can synchronize the native window with the real animation position;
- cancel its RAF on unmount;
- render a 6px unread dot without changing layout;
- use background-size: 1200% 100% and background-position-x based on frame / 11;
- reserve a 64x64 root and a centered 52x52 sprite;
- use a 150ms opacity crossfade under prefers-reduced-motion.

Core template and frame style:

    <button
      class="animated-banana"
      type="button"
      :data-frame="frame"
      :style="{ '--banana-frame': frame }"
      aria-label="打开或收起 Banana Box"
    >
      <span class="banana-sprite" aria-hidden="true" />
      <span v-if="unread" class="banana-unread" aria-label="有未读提醒" />
    </button>

    .animated-banana {
      position: relative;
      width: 64px;
      height: 64px;
      padding: 0;
      border: 0;
      background: transparent;
    }
    .banana-sprite {
      width: 52px;
      height: 52px;
      display: block;
      margin: 6px;
      background: url("@/assets/banana/banana-peel-sprite.webp") no-repeat;
      background-size: 1200% 100%;
      background-position:
        calc(var(--banana-frame) * 100% / 11) 0;
    }
    .banana-unread {
      position: absolute;
      top: 7px;
      right: 7px;
      width: 6px;
      height: 6px;
      border-radius: 50%;
      background: #ffd85a;
      box-shadow: 0 0 0 2px #101c24;
    }

- [ ] **Step 9: Run focused tests and commit**

Run:

    pnpm test -- tests/lib/bananaAnimation.test.ts tests/components/AnimatedBananaButton.test.ts
    git add src/assets/banana/banana-open-approved.png src/assets/banana/banana-peel-sprite.webp docs/design/banana-open-approved.sha256 src/components/AnimatedBananaButton.vue tests/components/AnimatedBananaButton.test.ts src-tauri/tests/banana_assets.rs
    git commit -m "feat: add reversible banana sprite animation"

Expected: all focused frontend tests and the previously run asset test pass; commit succeeds.

### Task 3: Restore And Persist The 64x64 Float Position

**Files:**

- Create: src-tauri/src/desktop_state.rs
- Reuse: src-tauri/src/fs_atomic.rs
- Modify: src-tauri/src/lib.rs
- Modify: src-tauri/tauri.conf.json

- [ ] **Step 1: Write failing pure geometry and persistence tests**

Add tests at the bottom of desktop_state.rs before implementation:

    #[cfg(test)]
    mod tests {
        use super::*;
        use tempfile::tempdir;

        fn monitor(id: &str, x: i32, y: i32) -> MonitorWorkArea {
            MonitorWorkArea {
                id: id.into(),
                bounds: PhysicalRect { x, y, width: 1920, height: 1080 },
                scale_factor: 1.0,
                primary: id == "primary",
            }
        }

        fn saved_on_removed_monitor(bounds: PhysicalRect) -> SavedFloatPosition {
            SavedFloatPosition {
                logical_x: 160.0,
                logical_y: 240.0,
                monitor_id: "removed".into(),
                scale_factor: 1.0,
                saved_work_area: bounds,
            }
        }

        #[test]
        fn restores_logical_offset_at_the_monitors_current_scale() {
            let monitor = PhysicalRect { x: 0, y: 0, width: 1920, height: 1080 };
            assert_eq!(
                logical_to_physical(LogicalPoint { x: 320.0, y: 240.0 }, monitor, 2.0),
                PhysicalPoint { x: 640, y: 480 },
            );
        }

        #[test]
        fn missing_saved_monitor_chooses_the_nearest_work_area() {
            let saved = saved_on_removed_monitor(PhysicalRect { x: 1920, y: 0, width: 1920, height: 1080 });
            let monitors = vec![monitor("left", -1920, 0), monitor("primary", 0, 0)];
            assert_eq!(select_restore_monitor(&saved, &monitors).id, "primary");
        }

        #[test]
        fn atomically_round_trips_the_saved_position() {
            let dir = tempdir().unwrap();
            let store = DesktopStateStore::new(dir.path().join("desktop-state.json"));
            let expected = SavedFloatPosition {
                logical_x: 320.0,
                logical_y: 240.0,
                monitor_id: "DISPLAY1".into(),
                scale_factor: 1.25,
                saved_work_area: PhysicalRect { x: 0, y: 0, width: 1920, height: 1080 },
            };
            store.save(&expected).unwrap();
            store.save(&expected).unwrap();
            assert_eq!(store.load().unwrap(), Some(expected));
            assert!(!dir.path().join("desktop-state.json.tmp").exists());
        }
    }

- [ ] **Step 2: Run the module test and confirm RED**

Temporarily add mod desktop_state; to src-tauri/src/lib.rs, then run:

    & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml desktop_state::tests

Expected: compile FAIL because the types and functions are undefined.

- [ ] **Step 3: Implement the atomic store and clamp function**

In src-tauri/src/desktop_state.rs define:

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct LogicalPoint { pub x: f64, pub y: f64 }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PhysicalPoint { pub x: i32, pub y: i32 }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub struct PhysicalRect {
        pub x: i32,
        pub y: i32,
        pub width: u32,
        pub height: u32,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct MonitorWorkArea {
        pub id: String,
        pub bounds: PhysicalRect,
        pub scale_factor: f64,
        pub primary: bool,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct SavedFloatPosition {
        pub logical_x: f64,
        pub logical_y: f64,
        pub monitor_id: String,
        pub scale_factor: f64,
        pub saved_work_area: PhysicalRect,
    }

    pub fn clamp_float_position(
        point: PhysicalPoint,
        bounds: PhysicalRect,
        window_size: i32,
        margin: i32,
    ) -> PhysicalPoint {
        let min_x = bounds.x + margin;
        let min_y = bounds.y + margin;
        let max_x = bounds.x + bounds.width as i32 - window_size - margin;
        let max_y = bounds.y + bounds.height as i32 - window_size - margin;
        PhysicalPoint {
            x: point.x.clamp(min_x, max_x.max(min_x)),
            y: point.y.clamp(min_y, max_y.max(min_y)),
        }
    }

`logical_x/logical_y` are offsets from the saved monitor work-area origin, in logical pixels. Add `logical_to_physical`, `physical_to_saved`, `select_restore_monitor`, and `clamp_float_position`: first match `monitor_id`; if missing, reconstruct the old physical window center from `saved_work_area + logical offset × saved scale`, then choose the monitor whose work area has the smallest squared point-to-rectangle distance. Convert with the selected monitor's current scale and clamp the full 64-logical-pixel window plus margin inside its current work area.

`DesktopStateStore::save` writes pretty JSON to `desktop-state.json.tmp`, flushes it, and calls the foundation-owned Windows-safe `fs_atomic::replace_file` so repeated saves can atomically replace an existing destination. Add a two-save test; do not use `std::fs::rename(temp, existing)` on Windows. `load` returns `Ok(None)` only for NotFound; malformed JSON returns an error and uses the safe default position without overwriting the malformed file.

- [ ] **Step 4: Make floatbtn hidden until position restoration**

In src-tauri/tauri.conf.json change floatbtn to:

    {
      "label": "floatbtn",
      "title": "",
      "width": 64,
      "height": 64,
      "decorations": false,
      "alwaysOnTop": true,
      "skipTaskbar": true,
      "visible": false,
      "resizable": false,
      "transparent": true,
      "shadow": false
    }

Remove fixed x and y. In setup, call `StartupGate::require_ready()` before loading desktop state or showing auxiliary windows. In ready mode, restore by monitor ID/current scale; if that monitor is absent choose the nearest work area using the saved origin/scale, clamp with a 12-logical-pixel margin, immediately persist the corrected logical state, then show floatbtn. With no saved state, place it 16 logical pixels from the primary monitor’s right edge and vertically centered. In recovery mode, keep `floatbtn` and `reminder` hidden and leave only foundation's `MainRoot -> RecoveryPage` main window visible.

- [ ] **Step 5: Persist Moved events with a 250ms generation debounce**

Add `FloatPositionDebouncer` with `AtomicU64` generation. On `WindowEvent::Moved` or `ScaleFactorChanged` for floatbtn, convert the current physical position to a logical work-area offset and record monitor ID, current scale, and work area. Spawn one 250ms delayed save; a closure whose captured generation is no longer current exits without writing.

Start a ready-mode `MonitorTopologyWatcher` that compares the sorted `(monitor_id, work_area, scale_factor)` signature every 5 seconds and on app resume. When it changes, rerun monitor selection/clamp, set the corrected physical position, and immediately persist the new logical state. Tests cover 100% -> 200% scale changes, resolution change, saved-monitor removal choosing the nearest display, and repeated atomic saves.

Do not save position changes for main or reminder.

- [ ] **Step 6: Run Rust tests and commit**

Run:

    & "$env:USERPROFILE\.cargo\bin\cargo.exe" fmt --manifest-path src-tauri/Cargo.toml
    & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml desktop_state::tests
    git add src-tauri/src/desktop_state.rs src-tauri/src/lib.rs src-tauri/tauri.conf.json
    git commit -m "feat: restore floating banana position"

Expected: both desktop_state tests pass and the commit succeeds.

### Task 4: Centralize Panel Visibility With Generation Fencing

**Files:**

- Create: src-tauri/src/window_state.rs
- Modify: src-tauri/src/lib.rs

- [ ] **Step 1: Write failing state-machine tests**

In window_state.rs add:

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn a_second_toggle_fences_the_pending_open() {
            let state = PanelStateMachine::default();
            let open = state.request(true);
            let close = state.request(false);
            assert!(state.complete(open.generation, true).is_none());
            assert_eq!(state.complete(close.generation, false).unwrap().visible, false);
        }

        #[test]
        fn toggle_uses_desired_not_delayed_actual_visibility() {
            let state = PanelStateMachine::default();
            state.request(true);
            assert_eq!(state.toggle().target_visible, false);
        }
    }

- [ ] **Step 2: Run focused Rust tests and confirm RED**

Add mod window_state; to lib.rs, then run:

    & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml window_state::tests

Expected: compile FAIL because PanelStateMachine is undefined.

- [ ] **Step 3: Implement one linearizable desired/actual/generation state**

Use a single mutex-protected value; do not split related state across atomics:

    #[derive(Default)]
    struct PanelState {
        desired_visible: bool,
        actual_visible: bool,
        generation: u64,
        reveal_ack_generation: Option<u64>,
    }

    #[derive(Default)]
    pub struct PanelStateMachine {
        inner: std::sync::Mutex<PanelState>,
    }

    pub struct PanelTransition {
        pub generation: u64,
        pub target_visible: bool,
    }

    impl PanelStateMachine {
        pub fn request(&self, target_visible: bool) -> PanelTransition {
            let mut state = self.inner.lock().expect("panel state poisoned");
            state.generation += 1;
            state.desired_visible = target_visible;
            state.reveal_ack_generation = None;
            PanelTransition { generation: state.generation, target_visible }
        }

        pub fn toggle(&self) -> PanelTransition {
            let mut state = self.inner.lock().expect("panel state poisoned");
            state.generation += 1;
            state.desired_visible = !state.desired_visible;
            state.reveal_ack_generation = None;
            PanelTransition {
                generation: state.generation,
                target_visible: state.desired_visible,
            }
        }

        pub fn complete(
            &self,
            generation: u64,
            visible: bool,
        ) -> Option<PanelVisibilityChanged> {
            let mut state = self.inner.lock().expect("panel state poisoned");
            if state.generation != generation || state.desired_visible != visible {
                return None;
            }
            state.actual_visible = visible;
            Some(PanelVisibilityChanged { generation, visible })
        }

        pub fn acknowledge_reveal(&self, generation: u64, frame: u8) -> bool {
            let mut state = self.inner.lock().expect("panel state poisoned");
            if frame < 6 || !state.desired_visible || state.generation != generation {
                return false;
            }
            state.reveal_ack_generation = Some(generation);
            true
        }
    }

`snapshot`, `request`, `toggle`, `complete`, ACK, native-observation, and compensation all lock this same state. Add a barrier test where an old completion has reached its commit barrier, a new request is inserted, then the old completion continues; it must change no field and emit no visibility event.

- [ ] **Step 4: Implement the one Rust visibility entry point**

Define the Rust reason enum in `window_state.rs` so every entry point and the integration plan share one serializable contract:

    #[derive(Clone, Copy, Debug, serde::Serialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub enum PanelTransitionReason {
        Banana,
        Tray,
        Shortcut,
        FileDrop,
        FocusLoss,
        TitlebarClose,
        ReminderAction,
        SecondInstance,
        Startup,
    }

    #[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub struct PanelTargetChanged {
        pub generation: u64,
        pub target_visible: bool,
        pub reason: PanelTransitionReason,
        pub reveal_at_frame: u8,
    }

    #[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub struct PanelVisibilityChanged { pub generation: u64, pub visible: bool }

    #[derive(Clone, Debug, serde::Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    pub struct PanelRevealAck { pub generation: u64, pub frame: u8 }

    #[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub struct PanelStateSnapshot {
        pub generation: u64,
        pub desired_visible: bool,
        pub actual_visible: bool,
    }

Set `reveal_at_frame` to the locked constant `6`; add a Rust serialization test that matches every TypeScript field/literal in the top contract, including `TitlebarClose -> "titlebarClose"`.

WindowStateService::request_visibility(app, target, reason) must:

1. request a new generation;
2. emit panel-target-changed to floatbtn immediately with revealAtFrame=6;
3. for close, hide main immediately and complete/broadcast false;
4. for open, wait until `ack_panel_reveal` reports frame 6 or later for the same generation; use a 400ms timeout only as event-loss recovery, then re-check generation and desired state before showing/focusing main and completing/broadcasting true;
5. emit panel-visibility-changed to floatbtn and main only after the real Tauri operation succeeds;
6. return errors instead of pretending the window changed.

`WindowStateService` owns a `tokio::sync::Notify`; `ack_panel_reveal` stores the acknowledged generation before notifying, so an ACK arriving just before the waiter starts cannot be lost. A reversed opening already at frame 6 or later ACKs immediately instead of waiting a fixed 196ms. Add `tokio = { version = "1", features = ["sync", "time"] }` to Cargo.toml only if it is not already a direct dependency.

`WindowStateService` also owns one async `native_transition: tokio::sync::Mutex<()>`. Never hold the state mutex while awaiting animation or calling native APIs. Immediately before a real show/hide, acquire the native mutex and re-read generation/desired; a stale worker exits without side effects. Keep the native mutex through the native call and state observation/commit. If a newer request arrives while the call is in progress, record the successful physical visibility, then, before releasing the native mutex, loop to the newest desired generation and perform the compensating native operation. Only the generation that matches the final physical state emits the committed visibility event.

Add barrier adapters for the exact `old show -> new close` and `old hide -> new open` interleavings: pause the old native call, enqueue/complete the new transition, then release the old call. Final native visibility, desired/actual state, generation, banana frame, and emitted visibility must all agree with the newest request; no stale native success may escape after a newer opposite operation.

Lock side-effect commit and compensation semantics. `actual_visible` changes only after native `show`/`hide` succeeds. If the initial `panel-target-changed` emit fails, or native show/hide fails, atomically restore `desired_visible=actual_visible` under a **new generation**, emit a compensating target plus visibility snapshot for that real state, schedule one event-loop reconciliation if either compensation emit fails, and return the original error. A later click therefore always toggles from reality, not the abandoned target.

Native show/hide success is the commit boundary. After show succeeds set `actual_visible=true` even when focus or post-commit event emission fails; after hide succeeds set it false. Focus/visibility-event failures return an accepted UI-sync warning, schedule bounded focus/event reconciliation, and never pretend the native operation failed or reverse it. Add injected failures for initial target emit, show, hide, focus, and post-commit visibility emit. Each test asserts the final desired/actual/generation, emitted compensation/warning, banana target frame, and that the immediately following click performs the expected opposite transition without opening/closing in the wrong direction.

- [ ] **Step 5: Route every existing entry point through WindowStateService**

Replace direct show/hide calls in lib.rs:

- banana command: reason banana;
- tray click/menu: reason tray;
- global shortcut: reason shortcut;
- show_panel after file drop: reason fileDrop;
- daily-task navigation after a reminder action: reason reminderAction (wired after production navigation exists);
- main-window `CloseRequested` from the titlebar: call `api.prevent_close()` and request false with reason titlebarClose so the banana target frame also closes;
- main focus loss: reason focusLoss.

Keep the existing drag grace period and pinned-window rule. A focus-loss close cancels any pending delayed open through the next generation.

Add a route test that fires `CloseRequested`, asserts native default close was prevented, main visibility becomes false through `WindowStateService`, and floatbtn receives the matching closed target/visibility events. No titlebar path may call `window.hide()` directly.

- [ ] **Step 6: Enforce one application instance before any other plugin**

Add tauri-plugin-single-instance = "2" to Cargo.toml. It must be the first plugin registered after tauri::Builder::default():

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            window_state::wake_existing_instance(app);
        }))
        .plugin(/* global shortcut plugin */)

`wake_existing_instance` reads `StartupGate`: when ready it requests target visible with reason secondInstance through `WindowStateService`; in recovery it only shows/focuses the existing main RecoveryPage and never touches normal panel state. It does not construct a database, tray, float window, reminder repository, or scheduler. Keep scheduler startup in setup so the rejected second process never reaches it.

Add a WindowStateService test that calls wake_existing_instance against the state-machine adapter and asserts one open request with reason secondInstance. Add a scheduler start-guard test in Task 10 that proves two start calls create one loop.

- [ ] **Step 7: Register get_panel_state, ack_panel_reveal, and show_panel**

get_panel_state returns desired/actual/generation. `ack_panel_reveal(generation, frame)` accepts only the current opening generation at frame 6 or later and wakes the matching service waiter; stale/early ACKs return false and never show a window. show_panel requests target true through the service and returns Result<(), String>; it must not call WebviewWindow::show directly.

- [ ] **Step 8: Run tests and commit**

Run:

    & "$env:USERPROFILE\.cargo\bin\cargo.exe" fmt --manifest-path src-tauri/Cargo.toml
    & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml window_state::tests
    & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml
    git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/window_state.rs src-tauri/src/lib.rs
    git commit -m "feat: synchronize panel window state"

Expected: panel fencing and second-instance wake tests plus the full Rust suite pass; commit succeeds.

### Task 5: Wire FloatButton To Rust State And Unread Priority

**Files:**

- Modify: src/components/FloatButton.vue
- Modify: tests/components/FloatButton.test.ts
- Modify: src/main.ts

- [ ] **Step 1: Replace the first two FloatButton tests with failing state tests**

Add listen, `get_panel_state`, and `get_reminder_unread_state` mocks. Capture event callbacks with this setup, then assert:

    const listeners = new Map<string, (event: { payload: unknown }) => void>()
    mocks.listen.mockImplementation(async (name, callback) => {
      listeners.set(name, callback)
      return () => listeners.delete(name)
    })

    it('renders the closed sprite from the Rust snapshot', async () => {
      mocks.invoke.mockImplementation((command) => {
        if (command === 'get_panel_state') {
          return Promise.resolve({ generation: 0, desiredVisible: false, actualVisible: false })
        }
        return Promise.resolve(undefined)
      })
      const wrapper = mount(FloatButton)
      await flushPromises()
      expect(wrapper.find('[data-frame="0"]').exists()).toBe(true)
    })

    it('uses activate_float_button and lets Rust prioritize unread reminders', async () => {
      const wrapper = mount(FloatButton)
      await wrapper.find('button').trigger('click')
      expect(mocks.invoke).toHaveBeenCalledWith('activate_float_button')
      expect(mocks.invoke).not.toHaveBeenCalledWith('toggle_panel')
    })

    it('hydrates persisted unread and rejects an older snapshot after a newer event', async () => {
      let resolveSnapshot!: (value: { unread: boolean; revision: number }) => void
      mocks.invoke.mockImplementation((command) => command === 'get_reminder_unread_state'
        ? new Promise((resolve) => { resolveSnapshot = resolve })
        : Promise.resolve({ generation: 0, desiredVisible: false, actualVisible: false }))
      const wrapper = mount(FloatButton)
      await flushPromises()
      listeners.get('reminder-unread-changed')!({ payload: { unread: true, revision: 4 } })
      resolveSnapshot({ unread: false, revision: 3 })
      await flushPromises()
      expect(wrapper.find('[aria-label="有未读提醒"]').exists()).toBe(true)
    })

    it('ACKs the current reveal generation when a forward or reversed animation reaches frame 6', async () => {
      const wrapper = mount(FloatButton)
      const onTarget = listeners.get('panel-target-changed')
      expect(onTarget).toBeTypeOf('function')
      onTarget!({
        payload: { generation: 7, targetVisible: true, reason: 'tray', revealAtFrame: 6 },
      })
      await flushPromises()
      wrapper.getComponent(AnimatedBananaButton).vm.$emit('frame', 6)
      await flushPromises()
      expect(mocks.invoke).toHaveBeenCalledWith('ack_panel_reveal', {
        generation: 7,
        frame: 6,
      })
    })

Keep the existing drag and file-drop tests.

- [ ] **Step 2: Run the FloatButton test and confirm RED**

Run:

    pnpm test -- tests/components/FloatButton.test.ts

Expected: FAIL because FloatButton still renders the emoji and invokes toggle_panel.

- [ ] **Step 3: Compose AnimatedBananaButton and subscribe to state events**

FloatButton.vue must:

- register all listeners first, then load `get_panel_state` and `get_reminder_unread_state` on mount;
- listen to panel-target-changed and set targetOpen from targetVisible;
- listen to panel-visibility-changed as reconciliation after native operations;
- keep `latestUnreadRevision`; apply `reminder-unread-changed` or unread snapshot only when its process-local revision is at least the last applied revision, so a delayed snapshot cannot overwrite a newer event;
- track the latest opening generation and invoke ack_panel_reveal exactly once when AnimatedBananaButton emits a frame greater than or equal to revealAtFrame; if a reversal starts beyond frame 6, ACK immediately;
- invoke activate_float_button for a non-drag click;
- guard activation with one `activationInFlight` promise: while it is pending, subsequent non-drag clicks are no-ops and the fixed-size button exposes `aria-busy=true`; clear the guard in `finally` without optimistically changing panel/reminder state;
- retain the current 3px drag threshold and dropped-file payload;

- on a fenced `reminder-attention`, add one 220 ms `banana-attention-sway` class to the fixed 64×64 visual wrapper (keyframes `0deg → -4deg → 3deg → 0deg`, transform origin `50% 85%`) without changing the 12-frame banana state, panel target, hitbox, or window bounds; ACK that exact claim on `animationend`, or immediately when `prefers-reduced-motion: reduce` is active. A newer claim replaces/clears the old class, and `reminder-hide`/timeout cleanup leaves no residual transform.

Task 9 seeds the Rust unread runtime from durable `reminder_log` before floatbtn is shown and registers `get_reminder_unread_state`. Add a mount fixture with `hidden/unread=1` already in SQLite and no new event; the first snapshot alone must show the yellow dot.
- unsubscribe from all listeners on unmount.

Do not optimistically toggle targetOpen on click; the Rust event is authoritative.

- [ ] **Step 4: Keep floatbtn as the first main.ts branch**

The branch remains:

    if (label === 'floatbtn') {
      createApp(FloatButton).mount('#app')
    }

The reminder branch is added in Task 7, before the main-app fallback.

- [ ] **Step 5: Run focused tests and commit**

Run:

    pnpm test -- tests/components/FloatButton.test.ts tests/components/AnimatedBananaButton.test.ts
    pnpm typecheck
    git add src/components/FloatButton.vue tests/components/FloatButton.test.ts src/main.ts
    git commit -m "feat: drive floating banana from native state"

Expected: all focused tests and typecheck pass; the file-drop assertions remain unchanged.

### Task 6: Build The Pausable 12-Second Reminder UI State

**Files:**

- Create: src/lib/reminderTimer.ts
- Create: tests/lib/reminderTimer.test.ts

- [ ] **Step 1: Write failing fake-time tests**

Create tests/lib/reminderTimer.test.ts:

    import { describe, expect, it, vi } from 'vitest'
    import { createReminderTimer } from '@/lib/reminderTimer'

    describe('reminder timer', () => {
      it('expires after 12 seconds of active time', () => {
        vi.useFakeTimers()
        const elapsed = vi.fn()
        const timer = createReminderTimer(12_000, elapsed)
        timer.start()
        vi.advanceTimersByTime(11_999)
        expect(elapsed).not.toHaveBeenCalled()
        vi.advanceTimersByTime(1)
        expect(elapsed).toHaveBeenCalledTimes(1)
      })

      it('does not spend time while pointer or focus interaction is active', () => {
        vi.useFakeTimers()
        const elapsed = vi.fn()
        const timer = createReminderTimer(12_000, elapsed)
        timer.start()
        vi.advanceTimersByTime(4_000)
        timer.pause()
        vi.advanceTimersByTime(30_000)
        expect(elapsed).not.toHaveBeenCalled()
        timer.resume()
        vi.advanceTimersByTime(8_000)
        expect(elapsed).toHaveBeenCalledTimes(1)
      })
    })

- [ ] **Step 2: Run timer tests and confirm RED**

Run:

    pnpm test -- tests/lib/reminderTimer.test.ts

Expected: FAIL because reminderTimer.ts does not exist.

- [ ] **Step 3: Implement active-time accounting**

createReminderTimer(durationMs, onElapsed) returns start, pause, resume, cancel, and remainingMs. Record Date.now() when active; pause subtracts only active elapsed time and clears the timeout; resume schedules exactly remainingMs. Guard onElapsed so it runs once.

Do not restart a fresh 12 seconds on every hover exit.

- [ ] **Step 4: Run timer tests and commit**

Run:

    pnpm test -- tests/lib/reminderTimer.test.ts
    git add src/lib/reminderTimer.ts tests/lib/reminderTimer.test.ts
    git commit -m "feat: add pausable reminder timeout"

Expected: 2 tests pass and the commit succeeds.

### Task 7: Implement The B-Style Reminder Window And Focus Rules

**Files:**

- Create: src/components/ReminderWindow.vue
- Create: tests/components/ReminderWindow.test.ts
- Create: src-tauri/capabilities/reminder.json
- Modify: src/main.ts
- Modify: src-tauri/tauri.conf.json

- [ ] **Step 1: Write failing ReminderWindow tests**

Mock listen and invoke; assert `ReminderWindow.vue` never imports a native window show/hide API. In `beforeEach`, make `prepare_reminder_layout` resolve `{ side: 'left', tailOffsetPx: 66 }`, fenced mutation commands resolve `{ accepted: true, replayed: false, uiSyncWarning: false }`, and other commands resolve `undefined`. Cover:

    it('measures while hidden and ACKs only after native show succeeds', async () => {
      Object.defineProperty(HTMLElement.prototype, 'scrollHeight', {
        configurable: true,
        value: 132,
      })
      mocks.invoke.mockImplementation(async (command) => {
        if (command === 'prepare_reminder_layout') {
          return { side: 'left', tailOffsetPx: 66 }
        }
        if (['ack_reminder_rendered', 'mark_reminder_auto_hidden', 'complete_reminder_action'].includes(command)) {
          return { accepted: true, replayed: false, uiSyncWarning: false }
        }
        return undefined
      })
      mount(ReminderWindow)
      await emitReminderPrepare(payload)
      await nextTick()
      expect(mocks.invoke).toHaveBeenCalledWith('prepare_reminder_layout', {
        claim: payload.claim,
        heightLogicalPx: 132,
      })
      expect(mocks.invoke).toHaveBeenCalledWith('show_prepared_reminder', {
        claim: payload.claim,
      })
      expect(mocks.invoke).not.toHaveBeenCalledWith('ack_reminder_rendered', expect.anything())
      await emitReminderShow({ claim: payload.claim })
      await nextTick()
      expect(mocks.invoke).toHaveBeenCalledWith('ack_reminder_rendered', {
        claim: payload.claim,
      })
    })

Add a deferred-ACK test: after `reminder-show`, advance the normal 12-second timer while `ack_reminder_rendered` is unresolved and assert no auto-hide call and all action buttons remain disabled; resolve ACK successfully before the Rust deadline, then assert the 12 active-second timer starts and actions enable. Reject ACK with `STALE_REMINDER_CLAIM` and assert the matching bubble clears without starting a timer. Reject ACK with a transient database error and assert the visible bubble shows “提醒状态保存失败，正在重试”, keeps actions disabled, and retries only within the five-second ACK window. At the deadline Rust emits matching `reminder-hide-request`; assert the bubble remains visible/disabled through 159ms, calls `ack_reminder_exit` on the 160ms animation end, and clears only after matching final `reminder-hide`/ACK cleanup (80ms in reduced-motion).

Add an ACK-response-loss fixture: the first invocation commits in Rust but its promise rejects as a transport loss; the retry returns `{ accepted: true, replayed: true, uiSyncWarning: false }`. The same bubble must remain, actions enable, and exactly one 12-second timer starts. A replay for a different claim or after hidden/actioned remains `STALE_REMINDER_CLAIM` and cannot revive content.

Also resolve ACK as `{ accepted: true, replayed: false, uiSyncWarning: true }`: this means the DB is already `shown`, so assert the bubble stays visible, actions enable, and the 12-second timer starts while Rust retries only unread-event synchronization. In separate auto-hide/dismiss/snooze/settle tests, the same warning on terminal/hidden mutations must disable and retain matching content until the fenced hide-request/exit/final cleanup, and must never resubmit the committed command.

Add explicit pre-show component failures: rejected `prepare_reminder_layout` must never invoke show/ACK or start a timer; rejected `show_prepared_reminder` must never invoke ACK or expose the hidden content as actionable; a later `reminder-prepare` for a reclaimed claim must render cleanly without stale inline errors.

    it('pauses auto-hide while hovered and marks hidden after 12 active seconds', async () => {
      vi.useFakeTimers()
      const wrapper = mount(ReminderWindow)
      await emitReminderPrepare(payload)
      await nextTick()
      await emitReminderShow({ claim: payload.claim })
      await wrapper.trigger('mouseenter')
      vi.advanceTimersByTime(20_000)
      expect(mocks.invoke).not.toHaveBeenCalledWith('mark_reminder_auto_hidden', expect.anything())
      await wrapper.trigger('mouseleave')
      vi.advanceTimersByTime(12_000)
      expect(mocks.invoke).toHaveBeenCalledWith('mark_reminder_auto_hidden', {
        claim: payload.claim,
      })
      expect(wrapper.find('[data-reminder]').exists()).toBe(true)
      expect(wrapper.get('[data-reminder]').attributes('aria-busy')).toBe('true')
      await emitReminderHideRequest({
        claim: payload.claim,
        durationMs: 160,
        reducedMotionDurationMs: 80,
      })
      vi.advanceTimersByTime(159)
      expect(wrapper.find('[data-reminder]').exists()).toBe(true)
      vi.advanceTimersByTime(1)
      expect(mocks.invoke).toHaveBeenCalledWith('ack_reminder_exit', {
        claim: payload.claim,
      })
      await emitReminderHide({ claim: payload.claim })
      expect(wrapper.find('[data-reminder]').exists()).toBe(false)
    })

    it.each([
      ['settle', 'settle'],
      ['snooze', 'snooze'],
      ['dismiss', 'dismiss'],
    ])('sends fenced %s action', async (_, action) => {
      const wrapper = mount(ReminderWindow)
      await emitReminderPrepare(payload)
      await nextTick()
      await emitReminderShow({ claim: payload.claim })
      await wrapper.get('[data-action="' + action + '"]').trigger('click')
      expect(mocks.invoke).toHaveBeenCalledWith('complete_reminder_action', {
        claim: payload.claim,
        action,
      })
    })

- [ ] **Step 2: Run component tests and confirm RED**

Run:

    pnpm test -- tests/components/ReminderWindow.test.ts

Expected: FAIL because ReminderWindow.vue does not exist.

- [ ] **Step 3: Implement event rendering and fenced actions**

ReminderWindow.vue listens for `reminder-prepare`, `reminder-show`, and fenced `reminder-hide-request`:

1. on prepare, replace any old payload while the native window remains hidden;
2. render title/body/time and only the ordered actions supplied by `payload.actions`; the close icon maps to `dismiss`, the daily-task initial payload supplies `['settle', 'snooze']`, and the snooze-phase payload supplies `['settle']`;
3. await nextTick, measure the single bubble's `scrollHeight`, clamp to 112..148 logical px, and invoke `prepare_reminder_layout` with the exact claim and height; store the returned side/tail placement while the window is still hidden;
4. after another nextTick, invoke `show_prepared_reminder` with the same claim; on matching `reminder-show`, invoke `ack_reminder_rendered` with the exact claim within 5 seconds while actions remain disabled;
5. start one 12-second active-time timer and enable actions only after matching ACK succeeds, never merely because the native show event arrived.

Add extreme-content component fixtures before implementation: a no-space 200-character Chinese title, a 500-character model body, long timestamp, both action buttons, left/right tails, and 200% logical scaling. For each, assert the transparent canvas stays within `276×148`, the surface/content viewport stays within its reserved lane, no text/button overlap occurs, and the close/action hit targets remain reachable. Pixel-alpha assertions prove the tail and full shadow remain inside all four native bounds without a clipped hard edge.

mouseenter, focusin, and pointerdown pause the timer. mouseleave and focusout resume only when neither pointer nor focus remains inside. Every action cancels the timer, immediately disables all buttons, and invokes `complete_reminder_action`; the database transition remains authoritative and is never rolled back for animation failure. After a matching accepted hidden/actioned result, Rust starts the two-phase exit protocol below rather than hiding the native window immediately. `STALE_REMINDER_CLAIM` clears only a still-matching stale payload, and an old exit response cannot hide a newer reminder.

Before native show, prepare/layout/show errors leave the native window hidden: clear the failed payload and wait for lease recovery rather than trying to render an invisible inline error. After native show but before ACK, use the ACK policy above. After ACK, genuine database/navigation rejection keeps the visible bubble and dirty action state for retry. A matching accepted auto-hide/dismiss/snooze/settle response means the DB state is committed, but Vue retains the disabled payload until `reminder-hide-request { claim, durationMs: 160, reducedMotionDurationMs: 80 }` arrives. It applies the exact claim's exit class, waits for `animationend`/reduced-motion transition end, invokes `ack_reminder_exit`, and clears only after that ACK or a matching backend cleanup event. A returned `uiSyncWarning=true` is committed success and is never retried; Rust's bounded exact-claim fallback owns native cleanup when the event/ACK path fails.

- [ ] **Step 4: Apply the exact “蕉签场记” B visual tokens**

Use one surface inside one transparent native canvas, no nested cards:

- total transparent canvas width 276px and maximum height 148px; the visible surface is 252px wide and 112..136px high;
- #101C24 main surface, #162730 secondary control surface, #2D4650 border;
- #F4FBF8 primary text, #9EB4B8 secondary text;
- #FFD85A 4px left status rail, #66F7D3 primary action, #FF7C73 error;
- 6px radius and one neutral shadow only;
- right-side tail, mirrored left by data-side;
- 44x44px interaction safety area for close and actions;
- no cloud outline, cartoon face, gradient orb, or colored layered shadow;
- title 14/20 semibold, body 12/18 regular, metadata 11/16 semibold, letter-spacing 0.

Lock overflow behavior rather than relying on native clipping. `.reminder-canvas` is the transparent `276×(surfaceHeight+12)` root with `overflow:visible` and every painted pixel explicitly kept inside its native bounds. On the tail side reserve an 18px lane and on the opposite side reserve a 6px neutral-shadow inset; mirror those lanes with `data-side`. Place the 252px surface inside them, with 6px vertical shadow insets. The tail/shadow are sibling paint layers behind the surface and never children of its clipped content viewport. Only `.reminder-content` uses `overflow:hidden`; every text/grid child uses `min-width:0; overflow-wrap:anywhere`. Reserve the close button's 44×44 area in the header. Metadata is one line; title is a two-line `line-clamp`; body is a one-line clamp when actions are present (two only in a no-action error state); the action row is fixed/non-shrinking and each control keeps a 44px hit area. Put the full untruncated title/body in accessible names and native `title` tooltips without adding hidden overflow layout. Tests inspect real element rectangles and alpha pixels, not only snapshots: left/right tail tips, blur extent, surface, and all hit targets must be inside the canvas at 100%/200% DPI, while content scroll width never exceeds the surface viewport.

Enter uses transform-origin at the tail, opacity 0 to 1, translateX(16px) to 0, scaleX(.82) to 1 over 235ms. Exit remains visibly rendered for 160ms toward the banana before frontend ACK; `prefers-reduced-motion` uses an 80ms opacity transition only. A native hide may not occur in the same turn as the normal hide-request emission.

- [ ] **Step 5: Add the reminder window configuration**

Append to app.windows in tauri.conf.json:

    {
      "label": "reminder",
      "title": "",
      "width": 276,
      "height": 148,
      "decorations": false,
      "alwaysOnTop": true,
      "skipTaskbar": true,
      "visible": false,
      "resizable": false,
      "transparent": true,
      "shadow": false,
      "focus": false
    }

Create src-tauri/capabilities/reminder.json:

    {
      "$schema": "../gen/schemas/desktop-schema.json",
      "identifier": "reminder",
      "description": "Minimum capability for the reminder bubble",
      "windows": ["reminder"],
      "permissions": [
        "core:event:allow-listen",
        "core:event:allow-unlisten"
      ]
    }

Do not recreate the deleted `default.json`/`desktop.json`, and do not add reminder to `main.json` or `floatbtn.json`. The reminder WebView gets no window, dialog, fs, updater, process, global-shortcut, clipboard, or broad `core:default` permission; Rust owns native show/hide/position/resize and Vue can only listen/unlisten plus invoke its allowlisted fenced commands. Add tests proving reminder cannot invoke `delete_project`, `restore_full_backup`, or `save_ai_provider`, and floatbtn cannot invoke those or reminder actions; each returns `FORBIDDEN_WINDOW` before dependencies are called.

- [ ] **Step 6: Mount ReminderWindow by native label**

Update main.ts without bypassing the foundation startup gate:

    if (label === 'floatbtn') {
      createApp(FloatButton).mount('#app')
    } else if (label === 'reminder') {
      createApp(ReminderWindow).mount('#app')
    } else {
      createApp(MainRoot).use(createPinia()).mount('#app')
    }

`MainRoot` remains the only main-window root because it chooses `App` versus `RecoveryPage` after `get_startup_status`; never mount `App` directly from `main.ts`.

- [ ] **Step 7: Run focused checks and commit**

Run:

    pnpm test -- tests/lib/reminderTimer.test.ts tests/components/ReminderWindow.test.ts
    pnpm typecheck
    git add src/components/ReminderWindow.vue tests/components/ReminderWindow.test.ts src/main.ts src-tauri/tauri.conf.json src-tauri/capabilities/reminder.json
    git commit -m "feat: add focus-safe reminder bubble"

Expected: focused tests and typecheck pass; commit succeeds.

### Task 8: Calculate Mirror, Clamp, And Tail Placement In Rust

**Files:**

- Create: src-tauri/src/reminder/geometry.rs
- Create: src-tauri/src/reminder/mod.rs
- Modify: src-tauri/src/lib.rs

- [ ] **Step 1: Write failing geometry tests**

In geometry.rs:

    #[cfg(test)]
    mod tests {
        use super::*;

        const SCREEN: Rect = Rect { x: 0, y: 0, width: 1920, height: 1080 };
        const BUBBLE: Size = Size { width: 276, height: 132 };

        #[test]
        fn prefers_left_of_a_right_edge_banana() {
            let placement = place_reminder(
                Rect { x: 1840, y: 500, width: 64, height: 64 },
                BUBBLE,
                SCREEN,
                14,
                12,
            );
            assert_eq!(placement.side, Side::Left);
            assert_eq!(placement.x, 1550);
            assert_eq!(placement.tail_offset_px, 66);
        }

        #[test]
        fn mirrors_right_when_left_does_not_fit() {
            let placement = place_reminder(
                Rect { x: 8, y: 400, width: 64, height: 64 },
                BUBBLE,
                SCREEN,
                14,
                12,
            );
            assert_eq!(placement.side, Side::Right);
            assert_eq!(placement.x, 86);
        }

        #[test]
        fn clamps_vertically_but_keeps_the_tail_pointing_at_the_banana() {
            let placement = place_reminder(
                Rect { x: 1800, y: 2, width: 64, height: 64 },
                BUBBLE,
                SCREEN,
                14,
                12,
            );
            assert_eq!(placement.y, 12);
            assert_eq!(placement.tail_offset_px, 22);
        }
    }

- [ ] **Step 2: Run geometry tests and confirm RED**

Register mod reminder; and pub mod geometry; then run:

    & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml reminder::geometry::tests

Expected: compile FAIL because Rect, Size, Side, and place_reminder are undefined.

- [ ] **Step 3: Implement deterministic physical-pixel placement**

Define Rect, Size, Side, and Placement. place_reminder:

1. computes the banana center;
2. tries the total transparent canvas on the left while placing the **tail tip**, not the canvas edge or surface edge, 14px from the banana's nearest edge;
3. mirrors right if left violates the 12px monitor margin;
4. if neither fits, selects the side with more usable width and clamps x;
5. vertically centers the total canvas, then clamps its outer bounds to the monitor;
6. calculates the tail offset relative to the visible surface and clamps it 22px from the surface corners; CSS receives both the surface-relative offset and mirrored tail-lane side.

All pure-geometry inputs and `Placement` outputs are physical pixels. `prepare_reminder_layout` selects the target monitor from the banana center first, then uses that monitor's scale factor, never the reminder window's stale pre-move scale. Convert the logical total canvas width `276`, surface width `252`, surface height clamped to `112..136`, total canvas height `surfaceHeight+12`, tail lane `18`, opposite shadow inset `6`, vertical inset `6`, tail-tip inset `6`, gap `14`, monitor margin `12`, and surface tail-edge margin `22` to physical pixels with that target scale before calling the pure function. Before returning `ReminderPlacement` to Vue, divide the physical surface-relative tail offset by the same target scale factor and round once, because CSS consumes logical pixels. Geometry/command tests cover 112, 124, and 136px surface heights, left/right mirroring, and a mixed-DPI 200% target where every canvas/lane/inset doubles. DOM-rectangle plus canvas-alpha screenshots assert no tail/shadow pixel touches a clipped native edge and the 14px measurement is from the actual tail tip to the banana; no code positions using a hard-coded 276px surface, 132px native height, or stale monitor scale.

- [ ] **Step 4: Implement two-phase prepare/show without focus stealing**

In reminder/mod.rs:

- manage one process-wide `ReminderWindowRuntime(Mutex<{ active: Option<ActiveReminderDelivery>, prepared: Option<PreparedReminder>, rendered: Option<ReminderClaimRef>, ack_deadline: Option<AckDeadline>, exit_deadline: Option<ExitDeadline> }>)` **before startup classification in all modes**. Recovery manages the same empty runtime so command argument extraction can never fail before authorized-envelope/Ready checks; no Recovery command may touch/lock it. `ActiveReminderDelivery` contains the exact claim and one stage from `attention | awaitingLayout | prepared | rendered | exiting`; `prepared` contains that same claim, measured logical height, target monitor ID/scale, and physical placement; `rendered` records only the claim currently occupying the native window; and both deadlines own that full claim plus cancellation tokens. This is the single-flight authority shared by the scheduler, manual unread reopen, float-button activation, and window commands. No database schema field is added;
- `prepare_reminder_window` verifies `state='pending'`, the exact fence, and `lease_expires_at > now`, then advances only the matching active claim from `attention` to `awaitingLayout` and emits `reminder-prepare` to the hidden reminder WebView; it does not ACK or change DB state. A different active claim returns `REMINDER_PRIORITY_IN_FLIGHT`; it must never clear or replace the older delivery;
- `prepare_reminder_layout` repeats that live-lease verification, reads floatbtn.outer_position/outer_size, selects the monitor containing the banana center (fallback current then primary), calculates the total-canvas physical placement with the selected monitor scale, sets native reminder canvas size/position, stores `PreparedReminder`, converts the surface-relative tail offset back to logical CSS pixels using the same target scale, and returns logical side/tail placement without showing;
- Vue applies the returned placement while hidden and invokes `show_prepared_reminder`; Rust atomically `take()`s a prepared entry only when its exact claim still has a live pending lease. A show-before-prepare, second show, stale claim, expired lease, or prepared entry from a prior claim returns `STALE_REMINDER_CLAIM` without touching the native window;
- after consuming the prepared state, Rust calls a Windows-tested `show_without_activation` helper, never plain repeated `window.show()` and never `set_focus`. For each hidden -> shown transition the helper executes `set_focusable(false) -> show() -> set_focusable(true)` with best-effort focusable restoration on every error; the locked Tao/Tauri version must be verified to use no-activate style changes. Merely setting config `focus:false` is insufficient because Tao consumes its first-show no-focus marker. After native no-activate show succeeds it advances that same active claim to `rendered`, sets `rendered` to the claim, emits `reminder-show`, starts one Rust-side five-second ACK deadline, then Vue ACKs and transitions pending -> shown;
- successful matching ACK, including an exact `replayed=true` ACK after response loss, cancels the matching ACK deadline before returning. When that deadline fires, its first action is `try_enter_background()`; maintenance means zero repo/runtime/event/native access. Under a permit it rechecks both the exact runtime rendered claim and the repository row: only the same still-pending claim enters the fenced exit protocol below; it leaves the DB pending for normal 30-second lease recovery. A replaced claim or already-shown row makes the old deadline a no-op;
- if measurement, positioning, or native show fails, clear runtime state only when it still belongs to the exact failed claim and leave the DB row pending for 30-second lease recovery. That still-live pending row remains a global delivery blocker until lease expiry, so the next scheduler tick cannot claim an unrelated reminder. If native show succeeds but `reminder-show` emission fails, immediately hide the native reminder, clear only the matching active/rendered state, do not ACK, and leave the row pending for recovery;
- implement **nonblocking** `begin_reminder_exit(claim)` for ACK-deadline failure, eligibility cancellation of a rendered claim, and every committed auto-hide/action. It atomically advances only the matching rendered claim to `exiting`, disables replacement through the existing global slot, emits `reminder-hide-request { claim, durationMs: 160, reducedMotionDurationMs: 80 }`, spawns one hard 260ms exact-claim fallback, and returns immediately without waiting under any caller's operation/eligibility guard. `ack_reminder_exit(claim)` or the fallback later obtains its own operation permit then eligibility fence, rechecks `active.stage=exiting` and `rendered` against the full claim, hides the native window, emits final `reminder-hide`, and clears only matching runtime/deadline state. Duplicate/stale ACKs return `STALE_REMINDER_CLAIM`; an old timer can never hide a newer claim. If event emission fails, the exact fallback still hides; if frontend ACK is lost, the same fallback hides. If maintenance wins, a callback performs zero runtime/native work and durable startup/exit reconciliation hides auxiliary windows; it never reaches across restore to mutate a new runtime. Database hidden/actioned/cancelled state is never rolled back for exit animation failure.

Use fake native-window, emitter, and clock adapters for backend tests: show-before-prepare, double show, expired lease, and prior-claim prepared state all return `STALE_REMINDER_CLAIM` without a native call; a second claim presented while another claim is in any active stage returns `REMINDER_PRIORITY_IN_FLIGHT` and leaves the first runtime payload byte-for-byte unchanged; resize/position/show failure clears only matching runtime state and leaves DB pending; show-success/event-failure performs exactly one compensating native hide, clears only matching rendered state, and leaves DB pending for reclaim. Assert the exact `focusable(false) -> show -> focusable(true)` call order and focusable restoration on each injected failure. In a real Windows integration test, capture `GetForegroundWindow`, programmatically show/hide/show the reminder for two complete cycles, and require the foreground HWND never changes; then click the reminder and prove normal user focus still works. Add the needed reviewed `windows-sys` `Win32_UI_WindowsAndMessaging` feature only once. Also prove the five-second ACK deadline begins exit for one still-pending matching claim, successful render ACK cancels it, normal/reduced-motion exit remains native-visible for 160/80ms, event or exit-ACK loss reaches the 260ms fallback, and an old ACK/deadline cannot hide a newer rendered claim. Race exit against restore maintenance and prove callbacks either finish under their permit before restore or do zero post-maintenance work. These are executable tests, not manual-only QA.

The explicit user click on a reminder action may focus reminder naturally; programmatic display must not activate it.

- [ ] **Step 5: Run tests and commit**

Run:

    & "$env:USERPROFILE\.cargo\bin\cargo.exe" fmt --manifest-path src-tauri/Cargo.toml
    & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml reminder::geometry::tests
    git add src-tauri/src/reminder/geometry.rs src-tauri/src/reminder/mod.rs src-tauri/src/lib.rs
    git commit -m "feat: position reminder beside floating banana"

Expected: 3 geometry tests pass and the commit succeeds.

### Task 9: Persist Reminder Claims With Lease, ACK, And Fencing

**Files:**

- Create: src-tauri/src/reminder/repository.rs
- Create: src-tauri/src/reminder/backup_validator.rs
- Modify: src-tauri/src/reminder/mod.rs
- Modify: src-tauri/src/lib.rs

- [ ] **Step 1: Write failing repository tests against the shared v1 database**

Open a temporary banana.db through crate::db::Database::open so the foundation-owned src-tauri/migrations/0001_v1.sql creates reminder_log. Do not call ReminderRepository::migrate and do not execute CREATE TABLE from this module. First assert PRAGMA table_info(reminder_log) contains exactly the foundation columns used here: id, kind, local_date, phase, state, delivery_id, attempt_token, owner_id, lease_expires_at, attempt_count, claimed_at, shown_at, acknowledged_at, snoozed_until, and unread.

Then cover:

    #[test]
    fn an_expired_unacked_lease_is_reclaimed_with_a_higher_fence() {
        let repo = test_repository();
        let first = repo.claim_initial("2026-07-13", at("2026-07-13T10:00:00Z"), "app-a").unwrap();
        let second = repo.claim_initial("2026-07-13", at("2026-07-13T10:00:31Z"), "app-b").unwrap();
        assert_eq!(second.delivery_id, first.delivery_id);
        assert_ne!(second.attempt_token, first.attempt_token);
        assert!(second.fence > first.fence);
        assert_eq!(
            repo.ack_rendered(&first, at("2026-07-13T10:00:32Z")).unwrap_err(),
            "STALE_REMINDER_CLAIM",
        );
        repo.ack_rendered(&second, at("2026-07-13T10:00:32Z")).unwrap();
    }

    #[test]
    fn auto_hide_sets_unread_and_reopen_rotates_the_delivery_fence() {
        let repo = test_repository();
        let claim = shown_initial(&repo);
        repo.mark_auto_hidden(&claim, at("2026-07-13T10:00:12Z")).unwrap();
        let reopened = repo.reopen_latest_unread("app-b", at("2026-07-13T10:00:13Z")).unwrap().unwrap();
        assert_ne!(reopened.delivery_id, claim.delivery_id);
        assert_ne!(reopened.attempt_token, claim.attempt_token);
        assert_eq!(reopened.fence, 1);
        assert_eq!(reopened.local_date, "2026-07-13");
    }

    #[test]
    fn snooze_is_created_once_and_is_not_claimable_before_due_time() {
        let repo = test_repository();
        let initial = shown_initial(&repo);
        repo.complete_action(&initial, ReminderAction::Snooze, at("2026-07-13T10:00:00Z")).unwrap();
        assert!(repo.claim_due_snooze("2026-07-13", at("2026-07-13T10:29:59Z"), "app").unwrap().is_none());
        assert!(repo.claim_due_snooze("2026-07-13", at("2026-07-13T10:30:00Z"), "app").unwrap().is_some());
        assert_eq!(
            repo.complete_action(&initial, ReminderAction::Snooze, at("2026-07-13T10:01:00Z")).unwrap_err(),
            "STALE_REMINDER_CLAIM",
        );
    }

    #[test]
    fn pending_claim_commands_fail_as_soon_as_the_lease_expires() {
        let repo = test_repository();
        let claim = repo.claim_initial("2026-07-13", at("2026-07-13T10:00:00Z"), "app-a").unwrap();
        assert_eq!(
            repo.validate_pending_claim(&claim, at("2026-07-13T10:00:30Z")).unwrap_err(),
            "STALE_REMINDER_CLAIM",
        );
    }

Add `ack_response_loss_is_idempotently_replayable`: first exact ACK changes pending to shown and returns `replayed=false`; repeating the identical full claim while still shown returns accepted with `replayed=true`, while any different delivery/token/owner/fence or a hidden/actioned row returns `STALE_REMINDER_CLAIM`. Add `startup_recovers_orphaned_shown_as_unread`: seed shown, simulate process restart before the 12-second timer/action, run Ready reconciliation, and assert hidden/unread=1, no automatic fourth delivery, and one persisted unread snapshot.

- [ ] **Step 2: Run repository tests and confirm RED**

Before running the filter, add `pub mod repository;` to `reminder/mod.rs`; otherwise the intended module may not compile.

Run:

    & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml reminder::repository::tests

Expected: compile FAIL because repository.rs and its types do not exist.

- [ ] **Step 3: Bind the repository to the foundation-owned schema**

ReminderRepository stores an Arc<crate::db::Database> and uses `with_connection` for reads and `with_immediate_transaction` for claim/reopen/fenced mutations. It has no migrate method. kind is dailyTasks; timestamps are UTC RFC 3339 and local_date is YYYY-MM-DD. Reminder title/body are derived from current daily-task data for `ReminderPreparePayload` and are not added to reminder_log.

- [ ] **Step 4: Implement the atomic claim statement**

Scheduler claim runs in an IMMEDIATE transaction. Insert the row if absent, then update only a pending row whose lease is absent or expired:

    UPDATE reminder_log
       SET state = 'pending',
           attempt_token = ?1,
           owner_id = ?2,
           attempt_count = attempt_count + 1,
           lease_expires_at = ?3,
           claimed_at = ?4
     WHERE kind = ?5
       AND local_date = ?6
       AND phase = ?7
       AND state = 'pending'
       AND attempt_count < 3
       AND (lease_expires_at IS NULL OR lease_expires_at <= ?4)
     RETURNING id, delivery_id, attempt_token, owner_id, attempt_count;

The lease expires after exactly 30 seconds. Initial insert supplies UUID id and delivery_id, state pending, and attempt_count=0 so the same transaction can claim attempt 1. Each unACKed reclaim keeps delivery_id, generates a new UUID attempt_token, changes owner_id, and increments attempt_count up to 3. shown/hidden/actioned/cancelled rows are not scheduler claims.

When attempt 3 reaches lease expiry without ACK, `mark_exhausted_delivery_unread(now)` atomically changes that same pending row to `hidden`, sets `unread=1`, clears lease/owner/token, and returns whether the unread event must be emitted. It never inserts a phase or performs an automatic fourth display. Apply this identically to initial and snooze rows; add tests proving attempt 1/2 remain reclaimable, attempt 3 expiry produces one unread transition/yellow dot, and repeated ticks are idempotent.

`claim_latest_unread_for_manual_activation` uses a separate IMMEDIATE transaction. It selects the newest `unread=1` row; to close the scheduler/button race it may also select an attempt-3 pending row whose lease is already expired and mark it unread inside this transaction. A hidden/exhausted row becomes pending with a new `delivery_id`/attempt token/owner, `attempt_count=1`, and a 30-second lease. A pending row with a live lease, or a currently shown/prepared runtime delivery, returns `ReminderPriorityInFlight` unchanged. It deliberately keeps unread=1 until the new render ACK succeeds. Rotating `delivery_id` only on a user click makes exhausted automatic attempts recoverable without counting as another scheduled initial/snooze phase; every old fence remains stale.

Add repository tests for both initial and snooze that exhaust three unACKed attempts, advance to exact lease expiry, run the scheduler unread transition, observe one yellow-dot event, invoke manual activation, and assert a new delivery ID with attempt 1 can render/ACK. All three old claims fail stale, unread clears only after the new ACK, and no extra reminder row/phase or automatic display is inserted. Repeat with the user click racing just before the scheduler tick.

- [ ] **Step 5: Implement fenced ACK, hidden, and action updates**

Every update includes:

    WHERE kind = ? AND local_date = ? AND phase = ?
      AND delivery_id = ? AND attempt_token = ?
      AND owner_id = ? AND attempt_count = ?

Transitions:

- Pending validation used by prepare/layout/show and ACK additionally requires `state='pending' AND lease_expires_at > now`; equality at expiry is stale. This closes the gap before the scheduler's next reclaim.
- ACK: pending with a live lease to shown; set shown_at and acknowledged_at; set unread=0; clear lease_expires_at.
- ACK replay: when the full claim still matches a `shown` row, change no DB field and return `replayed=true`; this is the only changed-row-zero success path. Hidden/actioned/cancelled or a mismatched claim is stale.
- Auto-hide: shown to hidden; unread=1.
- Dismiss: shown or hidden to actioned; unread=0.
- Settle: shown or hidden to actioned; unread=0.
- Snooze initial only: action initial and insert one snooze row with snoozed_until=now+30 minutes in one transaction.
- Cancel due snooze: pending to cancelled when tasks disappear or the day settles.

If changed-row count is zero, first perform the exact shown-ACK replay check above; otherwise return STALE_REMINDER_CLAIM.

- [ ] **Step 6: Expose unread query and emit helper**

Add `latest_unread`, `has_any_unread`, and `recover_orphaned_shown`. On every Ready startup, before scheduler start and before showing floatbtn, atomically change every process-orphaned `shown` row to `hidden, unread=1`, clear its lease/owner runtime fence as appropriate, and clear any in-memory prepared/rendered/deadline state. It must not create/reclaim a phase or count a fourth attempt.

Manage one default-empty `ReminderUnreadRuntime(Mutex<{ unread, revision }>)` before startup classification in all modes, then seed `unread` from the reconciled repository and `revision=1` only on Ready before floatbtn mounts. `get_reminder_unread_state`/activation authorized envelopes and Ready gate run before reading it, so Recovery returns `STARTUP_NOT_READY` with the empty runtime untouched rather than a missing State. A single helper rereads the repository after every auto-hide, dismiss, settle, snooze, reopen ACK, day mutation, exhaustion, and startup reconciliation; only when the Boolean changes it increments the process-local revision and emits `reminder-unread-changed`, while an explicit startup seed is queryable even if no event fires. Add snapshot/event ordering, preseeded unread, Recovery handler, and ACK-crash restart tests.

- [ ] **Step 7: Register reminder backup semantics**

Implement `ReminderBackupDomainValidator` with stable foundation-registry name `reminder-v1` and register it exactly once before `StartupCoordinator::run` in every mode. Reuse a pure repository `validate_persisted_row` so backup validation and normal load agree. It requires `kind='dailyTasks'`, canonical ISO source date backed by `daily_task_days`, unique initial/snooze phase semantics, valid RFC3339 timestamps/order, and one legal state tuple: unclaimed future snooze (`pending`, attempt 0, no lease/owner/token); claimed pending attempt 1..3 with a complete delivery/token/owner/claim/live-or-expired-lease tuple; acknowledged shown; hidden/unread; actioned; or cancelled. It validates phase-specific `snoozed_until`, shown/ack timestamps, unread/manual-reopen rules, delivery/attempt fence ranges, and forbids partial claim tuples or repository-impossible combinations without repairing them.

Before its focused test, add `pub mod backup_validator;` to `reminder/mod.rs` and call its one registration function from the shared pre-startup registry assembly. Assert it uses the same managed registry Arc as Foundation/Production/Storyboard and never creates a reminder-local registry.

Add safe-ID fixtures for bad kind/date/source day, initial with snooze time, snooze without due time, partial owner/token/lease, attempt 0/4 mismatch, shown without ACK, illegal unread state, timestamp inversion, and duplicate phase. Missing/duplicate `reminder-v1` blocks inspect/pre-switch/startup/ack; valid expired leases, exhausted hidden unread, manual-reopen pending unread, and cross-midnight/weekend snooze pass.

- [ ] **Step 8: Run repository tests and commit**

Run:

    & "$env:USERPROFILE\.cargo\bin\cargo.exe" fmt --manifest-path src-tauri/Cargo.toml
    & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml reminder::repository::tests
    git add src-tauri/src/reminder/repository.rs src-tauri/src/reminder/backup_validator.rs src-tauri/src/reminder/mod.rs src-tauri/src/lib.rs
    git commit -m "feat: persist fenced reminder claims"

Expected: lease expiry, stale ACK, unread reopen, and one-snooze tests pass; commit succeeds.

### Task 10: Schedule Weekday 18:00 Initial And Snooze Reminders

**Files:**

- Create: src-tauri/src/reminder/scheduler.rs
- Create: src-tauri/src/reminder/eligibility.rs
- Modify: src-tauri/src/reminder/mod.rs
- Modify: src-tauri/src/daily_tasks/model.rs
- Modify: src-tauri/src/daily_tasks/repository.rs
- Modify: src-tauri/src/daily_tasks/carry.rs
- Modify: src-tauri/src/daily_tasks/mod.rs
- Modify: src-tauri/src/lib.rs

- [ ] **Step 1: Write failing eligibility tests with an injected clock/source**

Define FakeClock and FakeDailyTaskSource in scheduler tests. Cover:

    #[test]
    fn claims_initial_at_or_after_18_on_a_weekday_with_unsettled_tasks() {
        let harness = Harness::at_local("2026-07-13T18:00:00+08:00"); // Monday
        harness.tasks.set(DayStatus { has_tasks: true, settled: false, previously_settled: false });
        harness.tick();
        assert_eq!(harness.shown_phases(), vec![ReminderPhase::Initial]);
    }

    #[test]
    fn never_claims_on_weekends_or_without_tasks_or_after_settlement() {
        for (time, status) in [
            ("2026-07-12T18:00:00+08:00", DayStatus { has_tasks: true, settled: false, previously_settled: false }),
            ("2026-07-13T18:00:00+08:00", DayStatus { has_tasks: false, settled: false, previously_settled: false }),
            ("2026-07-13T18:00:00+08:00", DayStatus { has_tasks: true, settled: true, previously_settled: true }),
            ("2026-07-13T18:00:00+08:00", DayStatus { has_tasks: true, settled: false, previously_settled: true }),
        ] {
            let harness = Harness::at_local(time);
            harness.tasks.set(status);
            harness.tick();
            assert!(harness.shown_phases().is_empty());
        }
    }

    #[test]
    fn a_tick_after_sleep_catches_up_once_on_the_same_workday() {
        let harness = Harness::at_local("2026-07-13T17:59:50+08:00");
        harness.tasks.set(DayStatus { has_tasks: true, settled: false, previously_settled: false });
        harness.tick();
        harness.clock.jump_to("2026-07-13T19:20:00+08:00");
        harness.tick();
        harness.tick();
        assert_eq!(harness.shown_phases(), vec![ReminderPhase::Initial]);
    }

    #[test]
    fn a_due_snooze_is_the_only_second_display() {
        let harness = shown_and_snoozed_harness();
        harness.clock.jump_to("2026-07-13T18:30:00+08:00");
        harness.tick();
        harness.tick();
        assert_eq!(
            harness.shown_phases(),
            vec![ReminderPhase::Initial, ReminderPhase::Snooze],
        );
    }

Add `snooze_crossing_midnight_is_delivered_at_due_utc` (Monday 23:50 snooze due Tuesday 00:20) and `friday_snooze_due_on_saturday_is_still_delivered`. In both, the original source day still has tasks and is neither settled nor previously settled; assert one snooze display at its UTC due time and no new weekend initial delivery.

Add eligibility linearization tests: pause after an initial claim but before native show, then delete the last task or settle the day through the real production hook; releasing show must produce no native window. The unACKed initial row is rearmed rather than consumed, so re-adding a task to a never-settled weekday after 18:00 yields exactly one fresh initial claim; a pending snooze is cancelled and never rearmed. Also pause a task mutation under the global operation permit while restore starts, prove restore waits, then prove a later tick/mutation skips or returns `RESTORE_PENDING` without a claim/write.

Add attention-order tests for automatic initial, due snooze, and manual unread reopen: `reminder-attention` reaches floatbtn, matching ACK (or 260 ms bounded timeout) occurs, then and only then `reminder-prepare` may reach the hidden reminder WebView. A stale attention ACK changes nothing. Frontend reduced-motion ACKs immediately with no transform; a later prepare/show failure clears the class and never leaves the banana tilted.

Add global single-flight tests with the real repository and `ReminderWindowRuntime`: keep an ACKed reminder hovered for longer than two 15-second ticks and prove no second row is claimed or payload replaced; seed two due snoozes from different source dates and prove only the oldest `(snoozed_until, delivery_id)` enters attention while the other waits until the first is hidden/actioned; leave a failed-show claim pending with a live lease while a current-day initial is eligible and prove neither the initial nor another snooze is claimed until that lease expires. At every completion/auto-hide/action/ACK-deadline cleanup, clear runtime state only for the matching full claim and let the next tick resume the same due-snooze-first ordering. An old Vue timer or Rust deadline firing after a newer claim starts must be a no-op and cannot hide or replace the newer claim.

    #[test]
    fn scheduler_start_guard_allows_exactly_one_loop() {
        let guard = SchedulerStartGuard::default();
        assert!(guard.try_start());
        assert!(!guard.try_start());
    }

- [ ] **Step 2: Run scheduler tests and confirm RED**

Before running the filter, register `pub mod eligibility; pub mod scheduler;` from `reminder/mod.rs`; keep `repository` registered from Task 9.

Run:

    & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml reminder::scheduler::tests

Expected: compile FAIL because scheduler.rs does not exist.

- [ ] **Step 3: Implement Clock and DailyTaskSource interfaces**

Use:

    pub trait Clock: Send + Sync {
        fn utc_now(&self) -> chrono::DateTime<chrono::Utc>;
        fn local_now(&self) -> chrono::DateTime<chrono::Local>;
    }

    pub struct DayStatus {
        pub has_tasks: bool,
        pub settled: bool,
        pub previously_settled: bool,
    }

    pub trait DailyTaskSource: Send + Sync {
        fn status_for(&self, local_date: &str) -> Result<DayStatus, String>;
    }

SqliteDailyTaskSource uses the joined `daily_tasks -> daily_task_groups -> daily_task_days` query fixed in this plan's contract. It derives `settled` from `settled_at IS NOT NULL` and `previously_settled` from `report_snapshot IS NOT NULL`; a missing date returns all three fields false.

Implement exactly one `ReminderEligibilityFence(tokio::sync::Mutex<()>)` and manage it before startup classification in **all** modes, including Recovery, shared by scheduler claim and pre-show critical sections, `show_prepared_reminder`, reminder actions, and the final production daily-task mutation commands. A reminder command may accept its always-managed State, but must authorize and pass `StartupGate::require_ready()` before locking it; Recovery therefore returns `STARTUP_NOT_READY`, never a missing-state framework error or a touched fence. Do not hold its guard across the 220 ms attention animation: release after the atomic claim, then reacquire and revalidate immediately before preparation/native show, allowing a task deletion/settlement to linearize between them. `ReminderDailyTaskMutationHook` implements the production-owned `DailyTaskMutationHook` without making production depend on reminder types. Within the same task transaction, it rereads post-mutation DayStatus. When ineligible it:

- rearms only an unACKed pending initial by rotating `delivery_id`, clearing token/owner/lease/claim timestamps, and resetting `attempt_count=0`, so the stale prepared claim cannot show but a later eligible tick may claim attempt 1;
- changes pending snooze to `cancelled`;
- changes an already shown/hidden now-ineligible delivery to `cancelled, unread=0`; an ACKed initial is never rearmed.

Rewire the shipped daily create/update/delete/reorder/reopen/settle command assembly to acquire `ReminderEligibilityFence` before its `spawn_blocking` transaction, pass the real hook, and retain the fence plus its existing `AppOperationGate` user permit until post-commit runtime hide/unread reconciliation finishes. Focused production checkpoints may keep their no-op hook, but final `lib.rs` registrations may not. Add transaction rollback tests for hook failure and command-level tests proving no final handler uses `NoopDailyTaskMutationHook`.

After a successful task mutation commits, reconcile runtime by full claim while still holding the guards. If the now-cancelled delivery is currently `rendered`, call nonblocking `begin_reminder_exit` so the normal 160/80ms fenced animation runs; if it is only in attention/awaiting-layout/prepared and has never been shown, cancel/clear that exact runtime and emit final cleanup without an exit animation. Then publish the unread snapshot and release both guards; exit ACK/fallback acquires its own permit -> eligibility fence later. Add hover-visible reminder -> delete last task and -> external settlement fixtures: DB becomes cancelled immediately, bubble exits visibly, and an old exit ACK cannot affect the next claim.

- [ ] **Step 4: Implement one deterministic scheduler tick**

ReminderScheduler::tick:

1. obtains `services.operations.try_enter_background()`; if maintenance is pending, return without reading DayStatus or reminder rows. Get UTC/Local now and acquire `ReminderEligibilityFence` around each eligibility/claim transaction, releasing it after a successful claim;
2. while holding `ReminderEligibilityFence`, reconcile expired runtime claims, then call one repository query that returns any row in `shown` or in `pending` with `lease_expires_at > utc_now`. If either that durable blocker or any `ReminderWindowRuntime.active` stage exists, return before any claim. A future snooze with no live owner/lease is not a blocker. This DB-plus-runtime check is mandatory even when unread is false and is repeated in the same fence used for the claim, so attention, prepare, render, hover, ACK wait, and failed-show live leases are one global delivery slot;
3. atomically mark every attempt-3 pending delivery whose lease is expired as hidden/unread and emit one repository-derived unread change when the aggregate bit changes; this creates no display/phase;
4. before any current-day weekday/hour gate, queries due snoozes ordered by `(snoozed_until, delivery_id)` across all source dates. For each candidate, load `DayStatus` for that delivery's stored `local_date`; cancel invalid candidates with no tasks, settled state, or a previous settlement, and atomically claim the first valid snooze whose `snoozed_until <= utc_now`. Before releasing the fence, reserve that exact claim as the sole `ReminderWindowRuntime.active` delivery in `attention`, then continue to step 8;
5. only when no due snooze was claimed, derive the current local YYYY-MM-DD and return on Saturday/Sunday or before 18:00 local time;
6. read current-day `DayStatus` and return when there are no tasks, the day is settled, or it was previously settled and reopened;
7. atomically claim the current day's initial delivery only if it has never been ACKed/shown, and before releasing the fence reserve that exact claim as the sole runtime delivery in `attention`;
8. for the reserved exact claim, store one fenced attention waiter, emit `reminder-attention { claim, durationMs: 220 }` to floatbtn, and wait for matching `ack_reminder_attention` or a hard 260 ms event-loss timeout. The ACK command is pure runtime state and never touches panel visibility/DB. Then reacquire the eligibility fence and re-read DayStatus, the exact claim, and the global runtime ownership. If tasks disappeared, the day settled/was previously settled, the hook rotated/cancelled the row, or the active claim changed, emit matching cleanup/hide and return without preparation; otherwise advance only that claim and call `show_reminder_window` so `reminder-prepare` follows the sway;
9. leave pending lease state intact if window/event rendering fails, allowing reclaim after 30 seconds and the exhausted-unread transition after attempt 3. Clear the matching runtime slot after the failure, but the durable live lease from step 2 continues to block unrelated claims until expiry.

Snooze due time is an absolute UTC instant, so a 23:50 snooze still appears at 00:20 and a Friday snooze may appear on Saturday; the weekend rule applies only to creating an initial reminder. Repository phase uniqueness prevents duplicate rows, while the durable/runtime single-flight gate prevents two different phases or dates from occupying the reminder pipeline together. Manual unread activation reuses the identical reservation and attention helper. `show_prepared_reminder` uses the same fence and repeats exact-claim, active-slot, durable-blocker ownership, and DayStatus checks immediately before its native call, closing both task-mutation TOCTOU and cross-claim replacement.

- [ ] **Step 5: Start a 15-second wake-safe scheduler loop**

Implement SchedulerStartGuard with AtomicBool::compare_exchange(false, true, SeqCst, SeqCst). In setup, first require `StartupGate::require_ready()`, complete orphan-shown/unread reconciliation, then require `try_start()` before creating the thread. Create one app-instance UUID as owner_id and start a dedicated loop using a stop channel with recv_timeout(Duration::from_secs(15)). Call tick immediately, then after every timeout. A sleeping computer resumes the blocked timeout and the next tick evaluates current wall time; no sequence of missed minute callbacks is replayed. Recovery mode creates no reminder repository/thread/channel and keeps both auxiliary windows hidden. A second `try_start()` returns false and creates no channel/thread.

Store the stop sender in managed ReminderSchedulerHandle. Signal it on RunEvent::ExitRequested. Do not run the scheduler after the user fully exits.

For deterministic desktop QA, add debug_set_reminder_now behind cfg(debug_assertions). It accepts rfc3339: Option<String>, parses a full offset timestamp into the injected clock override, and triggers one scheduler tick; null restores the system clock. Use separate cfg-gated invoke_handler lists so release builds do not register this command.

- [ ] **Step 6: Run scheduler and repository tests**

Run:

    & "$env:USERPROFILE\.cargo\bin\cargo.exe" fmt --manifest-path src-tauri/Cargo.toml
    & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml reminder::scheduler::tests
    & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml reminder::repository::tests

Expected: weekday, no-task, settled, weekend-initial, cross-midnight/weekend snooze, sleep catch-up, lease, and fencing tests all pass.

- [ ] **Step 7: Commit the scheduler**

Run:

    git add src-tauri/src/reminder src-tauri/src/daily_tasks src-tauri/src/lib.rs
    git commit -m "feat: schedule weekday task reminders"

Expected: commit succeeds.

### Task 11: Wire Reminder Commands, Unread Reopen, And Daily-Task Navigation

**Files:**

- Modify: src-tauri/src/reminder/mod.rs
- Modify: src-tauri/src/lib.rs
- Modify: src/components/FloatButton.vue
- Modify: tests/components/FloatButton.test.ts
- Modify: tests/components/ReminderWindow.test.ts

- [ ] **Step 1: Add failing unread-priority and double-click tests**

Mock activate_float_button to return unreadReminderReopened and assert no local panel state changes before a Rust event:

    it('does not open the panel when Rust reopens an unread reminder', async () => {
      mocks.invoke.mockResolvedValueOnce({
        generation: 1,
        desiredVisible: false,
        actualVisible: false,
      })
      mocks.invoke.mockResolvedValueOnce({ action: 'unreadReminderReopened' })
      const wrapper = mount(FloatButton)
      await flushPromises()
      await wrapper.find('button').trigger('click')
      expect(mocks.invoke).toHaveBeenLastCalledWith('activate_float_button')
      expect(wrapper.find('[data-frame="0"]').exists()).toBe(true)
    })

Add a second test whose first `activate_float_button` invoke is a deferred promise. Trigger two non-drag clicks before resolving it and assert exactly one invoke, no `toggle_panel`, `aria-busy=true`, and no local target-frame change. Resolve it, assert busy clears, and prove a later click can invoke once more.

- [ ] **Step 2: Implement activate_float_button**

The command performs one repository decision before touching panel state:

1. before inspecting unread, check the process-wide runtime and repository under `ReminderEligibilityFence`. If an attention waiter, `awaitingLayout`, prepared, or rendered stage exists, or any durable delivery is `shown`/live `pending`, return `reminderPriorityInFlight` without issuing another claim/show and without calling `WindowStateService.toggle`. This rule applies even when unread is `0`, closing the 220 ms automatic-attention click window;
2. if no delivery owns the global slot and the latest unread delivery is `hidden` or `pending` with an expired lease, atomically rotate it through `claim_latest_unread_for_manual_activation`, reserve that exact claim as the sole active `attention` delivery before releasing the fence, show the reminder, keep unread true until ACK, and return `unreadReminderReopened`;
3. only when the global slot is idle and no unread delivery exists, call `WindowStateService.toggle` with reason banana and return `panelToggleRequested`.

If show fails, keep the new claim pending until lease expiry and keep unread=true so the reminder remains recoverable.

`activate_float_button` is floatbtn-only and performs authorization → Ready → user operation permit before any unread query. Its reminder branch also acquires `ReminderEligibilityFence` through claim/show; only the no-unread fallback delegates to the pure `WindowStateService.toggle`. Maintenance returns `RESTORE_PENDING` without claiming or toggling the panel, so restore cannot race the priority decision.

Add a Rust race test that blocks the first hidden-to-pending activation, calls activation again after the transition is visible, and asserts the second result is `reminderPriorityInFlight`, with one reminder claim/show and zero panel toggles. Add the exhausted-attempt path: after three expired failures, the next explicit click rotates the delivery and shows once rather than returning an in-flight no-op. For an automatic initial, click the banana at attention times 0 ms, 110 ms, and 219 ms and again between prepare and native show; every click returns `reminderPriorityInFlight`, exactly one reminder proceeds, and `WindowStateService.toggle` is never called. Together with the frontend single-flight test, rapid double-click or scheduler/button interleaving can never open the reminder and main panel at the same time.

- [ ] **Step 3: Register fenced reminder IPC commands**

Register:

    prepare_reminder_layout(args: ReminderArgs<PrepareReminderLayoutCommandArgs>) -> ReminderPlacement
    show_prepared_reminder(args: ReminderArgs<ClaimCommandArgs>)
    ack_reminder_rendered(args: ReminderArgs<ClaimCommandArgs>) -> ReminderMutationResult
    ack_reminder_exit(args: ReminderArgs<ClaimCommandArgs>) -> ReminderMutationResult
    mark_reminder_auto_hidden(args: ReminderArgs<ClaimCommandArgs>) -> ReminderMutationResult
    complete_reminder_action(args: ReminderArgs<CompleteReminderActionCommandArgs>) -> ReminderMutationResult

Render ACK marks shown. Auto-hide marks hidden/unread. Dismiss marks actioned/unread=false. For those DB mutations, the database transition is the commit boundary: exit event/animation/native hide afterward is best-effort, a failure sets `uiSyncWarning=true`, schedules the exact-claim fallback/reconciliation, and never converts committed success into IPC rejection. Only a failed database transition rejects. Render-ACK warning keeps the shown bubble active; accepted hidden/terminal mutations disable it and enter `begin_reminder_exit` rather than clearing/hiding synchronously. `ack_reminder_exit` is a pure fenced runtime/native completion and never changes the already-committed row. This prevents a false retry while allowing the specified 160/80ms exit to be visible.

Every listed reminder IPC injects `WebviewWindow`, always-managed `StartupGate`, always-managed `ReminderEligibilityFence`, and exactly one foundation non-deserializable `ReminderArgs<WholeCommandArgs>`; every inner DTO is camelCase/deny-unknown and preserves the existing flat invoke shape. Envelope extraction authorizes the reminder label before parsing; the body then executes Ready → `window.app_handle().try_state::<AppServices>()` → `services.operations.enter_user()` before repository/runtime/clock/business-input access and retains the permit across navigation/native/event reconciliation. Float-button commands analogously use `FloatArgs`, and dual-surface pure panel commands use `MainOrFloatArgs`; no protected command has an ordinary deserializable payload or required `State<AppServices>`. Malformed payloads from a wrong surface return `FORBIDDEN_WINDOW`, malformed authorized payloads return `INVALID_INPUT`, and Recovery returns `STARTUP_NOT_READY`. Layout/show/action take `ReminderEligibilityFence` only after those checks, in the consistent order “operation permit, then eligibility fence”. An identical shown-ACK replay returns `{ accepted: true, replayed: true, uiSyncWarning: false }`, cancels the exact Rust deadline, and lets Vue start interaction; no other stale mutation is replayable.

Add injected-failure backend tests for each post-commit side effect: render ACK plus unread-event failure returns accepted/warning and leaves `shown`; auto-hide plus exit-event failure returns accepted/warning and leaves `hidden`, then exact fallback hides; dismiss, snooze, and settle failures after commit return accepted/warning and leave their committed rows while the exit fallback remains reliable. Every queued exit/reconciliation callback first acquires a background operation permit and retains it to its final repo/runtime/event/native action; maintenance makes it skip and startup durable reconciliation repairs the view. Add pre-commit DB failure tests proving each returns `Err`, leaves rows unchanged, and performs no exit event/native hide, plus normal/reduced-motion exit ACK, event loss, ACK loss, stale-exit/new-show, restore-versus-ACK-deadline, and restore-versus-warning-reconcile barriers. Gstack recording must show at least 160ms of visible normal exit frames and 80ms opacity-only reduced-motion frames before native disappearance.

Add maintenance barrier tests for unread snapshot/activation, prepare/show/ACK, auto-hide, and each action. If a user permit exists first, restore waits through the last commit/reconciliation; if maintenance wins, the command returns `RESTORE_PENDING` with no repo/native/navigation call. Scheduler uses only its background permit and skips. Pure banana panel transition commands remain usable and do not enter the DB gate.

- [ ] **Step 4: Reuse daily_tasks navigation for the settle action**

For ReminderAction::Settle:

    crate::daily_tasks::navigation::navigate_to_daily_tasks(
        &app,
        claim.local_date.clone(),
    ).await?;
    repository.complete_action(&claim, ReminderAction::Settle, clock.utc_now())?;
    let ui_sync_warning = reconcile_unread_best_effort(&app, &repository)
        | begin_reminder_exit_nonblocking(&app, &claim).is_err();
    Ok(ReminderMutationResult { accepted: true, replayed: false, ui_sync_warning })

Do not emit open-daily-tasks directly here; navigate_to_daily_tasks owns that stable event and main-window state. Validate the claim before navigation, but persist `actioned` only after navigation succeeds. If showing/focusing/navigating the main window fails, return a retryable error and keep the bubble, unread state, and claim intact. If navigation succeeds but the fenced database action fails, the main page may already be open, yet the bubble remains visible with retry; repeated navigation is idempotent and emits the same local date.

- [ ] **Step 5: Implement the one-snooze action**

For Snooze, accept only phase initial. In one repository transaction mark initial actioned/unread=false and insert the snooze row due exactly 30 minutes after the action. After commit, reconcile unread and schedule the same nonblocking `begin_reminder_exit`; return committed success even if either UI sync step warns, and do not navigate main. Snooze on phase snooze returns SNOOZE_ALREADY_USED.

- [ ] **Step 6: Reconcile frontend stale/error handling**

ReminderWindow:

- captures `deliveryId + attemptToken + ownerId + fence` before every ACK/timer/action call and applies the response only if the currently rendered payload still has that exact claim;
- on `STALE_REMINDER_CLAIM`, hides only when the stale claim is still rendered; if a newer `reminder-show` already replaced it, ignore the old response so a late owner cannot hide the current delivery;
- before show, clears hidden prepare/show failures for lease recovery; before ACK success, disables actions/timer and follows the bounded ACK retry policy; after ACK, keeps the bubble visible only for true database/navigation rejection;
- treats render ACK `accepted=true` with warning as shown success and continues interaction, while accepted hidden/terminal mutations (warning or not) disable and retain matching content through `reminder-hide-request`; neither path retries the committed mutation;
- clears its payload only on matching final `reminder-hide`/exit cleanup, never on the committed mutation response alone;
- never invokes a second action while one is pending.

FloatButton updates only through panel/unread events.

- [ ] **Step 7: Run frontend and Rust integration-focused tests**

Run:

    pnpm test -- tests/components/FloatButton.test.ts tests/components/ReminderWindow.test.ts
    & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml reminder::
    & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml window_state::

Expected: frontend reminder/float tests pass; Rust reminder and window-state tests pass.

- [ ] **Step 8: Commit command wiring**

Run:

    git add src-tauri/src/reminder/mod.rs src-tauri/src/lib.rs src/components/FloatButton.vue tests/components/FloatButton.test.ts tests/components/ReminderWindow.test.ts
    git commit -m "feat: connect reminder actions and unread recovery"

Expected: commit succeeds.

### Task 12: Full Verification And Desktop QA

**Files:**

- Modify only files required to fix failures found by the commands below.

- [ ] **Step 1: Run formatting and Rust tests**

Run:

    & "$env:USERPROFILE\.cargo\bin\cargo.exe" fmt --manifest-path src-tauri/Cargo.toml -- --check
    & "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri/Cargo.toml

Expected: formatter reports no diff; all Rust unit/integration tests pass.

- [ ] **Step 2: Run Rust lint**

Run:

    & "$env:USERPROFILE\.cargo\bin\cargo.exe" clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings

Expected: exit code 0 with no warnings. Fix only warnings introduced by this plan.

- [ ] **Step 3: Run the complete frontend gate**

Run:

    pnpm check
    pnpm build

Expected: typecheck, ESLint, Vitest, and the production Vite build all pass.

- [ ] **Step 4: Start the real desktop development build**

Run:

    pnpm tauri dev

Expected: one 64x64 transparent banana window appears after its position is restored; main and reminder start hidden.

- [ ] **Step 5: Verify banana behavior manually**

Check at Windows scaling 100%, 150%, and 200%:

1. initial state is closed;
2. click opens over 12 frames/360ms and main appears at frame 6;
3. second click reverses from the current frame;
4. tray, Ctrl+Shift+B, file drop, pin, and focus loss keep native and sprite state aligned;
5. dragging more than 3px moves without toggling;
6. restarting restores and clamps the 64x64 position;
7. unplugging the saved monitor moves the banana onto an available monitor.
8. while the first dev executable is running, execute Start-Process -FilePath '.\src-tauri\target\debug\banana-box.exe' -WindowStyle Hidden from a second PowerShell; the second process exits, the existing main window is focused, and Task Manager still shows one Banana Box process tree, one float window, and one scheduler loop.

Expected: no clipped sprite, position jump, stale delayed open, or broken file drop.

- [ ] **Step 6: Verify reminder behavior manually**

Seed one unsettled daily task in the application UI. From the Tauri devtools console invoke debug_set_reminder_now with rfc3339 set to 2026-07-13T18:00:00+08:00, then verify:

1. one initial bubble appears beside the banana without stealing focus;
2. left/right mirror and vertical tail clamp follow the banana;
3. hover and keyboard focus pause, rather than reset, the 12-second timer;
4. auto-hide sets the 6px unread dot;
5. the next banana click reopens unread instead of opening main;
6. stale actions from the old claim are rejected after reopen;
7. snooze creates exactly one display 30 minutes later;
8. “去结算” calls navigate_to_daily_tasks and main receives open-daily-tasks;
9. settlement clears unread and cancels due snooze;
10. a simulated sleep jump from 17:59 to 19:20 produces one initial display, never two.

The debug clock is compiled under cfg(debug_assertions), accepts a full RFC 3339 instant, and is absent from release invoke_handler registrations.

- [ ] **Step 7: Run Gstack visual and interaction review**

Use Gstack browse/qa/design-review against the Vite reminder surface and the live Tauri windows. Capture the banana closed/open endpoints, reminder on both sides, long Chinese title/body, keyboard focus, and 200% scaling. For every one of the 12 sprite frames at 100% and 200% DPI, take a canvas-pixel screenshot and assert the visible mascot/shadow stays inside the intended 52×52 visual box with transparent clearance before the fixed 64×64 native edge; compare consecutive screenshots against the Rust change/centroid thresholds. Record one automatic and one manual-unread sequence proving “220 ms light sway → bubble tail/enter”, plus a reduced-motion run with no sway; record normal/reduced exit duration and two consecutive programmatic reminder shows with foreground HWND unchanged. Verify the 64×64 window, 12-frame panel state, and final transform never shift.

Expected: B-style colors/radius/one-shadow rule match the spec; text and buttons do not overlap; reminder does not resemble a large dark card or cloud; no high-severity QA finding remains.

- [ ] **Step 8: Review the final diff for scope**

Run:

    git status --short
    git diff --stat HEAD~10
    git diff --check

Expected: only mapped desktop-interaction files and necessary dependency lock changes appear; diff-check reports no whitespace errors.

- [ ] **Step 9: Commit any verification-only corrections**

If verification changed files, run:

    git add src tests src-tauri package.json pnpm-lock.yaml
    git commit -m "fix: harden desktop reminder integration"

Expected: commit succeeds. If no corrections were required, do not create an empty commit.

## Completion Checklist

- [ ] Float window is exactly 64x64, hidden until restore, persisted atomically, and clamped after monitor/DPI changes.
- [ ] Banana uses one approved 12-frame asset, 360ms total duration, frame-6 reveal, and interruption-safe reversal.
- [ ] Rust desired/actual/generation state drives banana, tray, shortcut, file-drop, and focus-loss behavior.
- [ ] Main display is triggered by the current generation's actual frame-6 ACK, including mid-animation reversal; fixed-delay stale opens are impossible.
- [ ] tauri-plugin-single-instance is initialized first; a second launch only focuses the existing app and cannot start another scheduler.
- [ ] Recovery startup mounts only MainRoot/RecoveryPage; floatbtn/reminder stay hidden and no reminder scheduler or normal DB command starts.
- [ ] Reminder B styling, tail mirror/clamp, no-focus display, and active-time pause/resume behavior match the approved spec.
- [ ] Weekday 18:00 initial, one 30-minute snooze, no-task/settled cancellation, and sleep catch-up are deterministic.
- [ ] Owner ID, delivery ID, unique attempt token, per-delivery fence counter, 30-second lease, render ACK within 5 seconds, stale-action rejection, and atomic claims are tested.
- [ ] Auto-hide unread state survives in SQLite; banana click reopens the latest unread reminder before toggling main.
- [ ] Settle uses daily_tasks::navigation::navigate_to_daily_tasks and does not duplicate open-daily-tasks.
- [ ] pnpm check, pnpm build, cargo test, cargo clippy, Tauri manual QA, and Gstack review pass.

## Execution Handoff

Plan complete and saved to docs/superpowers/plans/2026-07-11-banana-box-v1-desktop-interaction.md. Two execution options:

1. Subagent-Driven (recommended): use superpowers:subagent-driven-development, dispatch a fresh worker per task, and review spec compliance plus code quality after each task.
2. Inline Execution: use superpowers:executing-plans and execute in batches with review checkpoints.
