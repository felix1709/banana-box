# Frontend Visual Refresh Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refresh Banana Box's frontend into a polished compact desktop-tool interface without changing existing behavior.

**Architecture:** Keep the current Vue component structure and Pinia/Tauri data flow. Add shared CSS tokens and primitive control styles in `src/styles/main.css`, then update component-level scoped CSS and only make small template text/accessibility refinements where needed.

**Tech Stack:** Vue 3, TypeScript, Pinia, Vite, Tauri, Vitest, CSS custom properties.

---

## Files

- Modify `src/styles/main.css`: global design tokens, primitive button/input/select/textarea styling, scrollbars, focus states.
- Modify `src/App.vue`: app shell, topbar, content, prompt list, toast, preview overlay styling; fix visible garbled labels.
- Modify `src/components/AppSidebar.vue`: navigation item styling, create button, category scroll container; fix visible labels.
- Modify `src/components/CategoryTree.vue`: compact category row styling; fix visible labels.
- Modify `src/components/SearchBar.vue`: refined search input; fix placeholder.
- Modify `src/components/PromptCard.vue`: compact prompt card surface, tags, image thumb, actions, favorite, drag/category states; fix visible labels.
- Modify `src/components/PromptEditor.vue`: modal/form styling; fix visible labels.
- Modify `src/components/SettingsModal.vue`: modal/form grouped sections and scroll; fix visible labels.
- Modify `src/components/ReverseImagePanel.vue`: upload work surface, actions, result styling; fix visible labels.
- Modify `src/components/FastCompressionPanel.vue`: upload work surface, target input, progress styling; fix visible labels.
- Modify `src/components/FloatingActionDialog.vue`: action modal styling; fix visible labels.
- Modify `src/components/ConfirmDialog.vue` and `src/components/CategoryDialog.vue` only if needed for modal consistency.

## Task 1: Baseline Verification

- [ ] **Step 1: Run existing automated checks before editing**

Run:

```powershell
pnpm check
```

Expected: typecheck, lint, and tests pass before visual work starts.

- [ ] **Step 2: Start the existing dev server**

Run:

```powershell
pnpm dev
```

Expected: Vite serves the app on `http://127.0.0.1:1422/`.

## Task 2: Add Global Design Tokens

- [ ] **Step 1: Update `src/styles/main.css`**

Add CSS custom properties for colors, typography, spacing, radius, shadow, focus, disabled state, scrollbars, and primitive controls.

Use tokens such as:

```css
:root {
  --bb-bg: #eef3f8;
  --bb-surface: #ffffff;
  --bb-surface-soft: #f8fafc;
  --bb-border: #d9e2ec;
  --bb-primary: #2563eb;
  --bb-danger: #dc2626;
  --bb-favorite: #d97706;
  --bb-radius-md: 8px;
  --bb-shadow-dialog: 0 20px 48px rgba(15, 23, 42, 0.24);
}
```

- [ ] **Step 2: Verify no behavior changed**

Run:

```powershell
pnpm test -- tests/components/App.test.ts tests/components/SettingsModal.test.ts
```

Expected: tests pass.

## Task 3: Refresh Shell, Sidebar, Search, And Category Tree

- [ ] **Step 1: Update app shell styles in `src/App.vue`**

Refine `.app`, `.window-drag-strip`, `.topbar`, `.body`, `.sidebar`, `.content`, `.prompt-list`, `.empty`, `.toast`, `.preview-mask`, and `.preview-img`.

- [ ] **Step 2: Update visible labels in `src/App.vue`**

Use clear Chinese text for drag strip, settings, create, and empty state. Do not change click handlers or conditions.

- [ ] **Step 3: Update `src/components/SearchBar.vue`**

Use a plain searchable placeholder like `搜索提示词、标签...` and refined input styling.

- [ ] **Step 4: Update `src/components/AppSidebar.vue` and `src/components/CategoryTree.vue`**

Use clear Chinese labels for the three tools, favorites, all, and new category. Refresh active/hover/delete/create states.

- [ ] **Step 5: Run sidebar and app tests**

Run:

```powershell
pnpm test -- tests/components/AppSidebar.test.ts tests/components/App.test.ts tests/stores/library.test.ts
```

Expected: tests pass and no functional assertions need to change.

## Task 4: Refresh Prompt Cards

- [ ] **Step 1: Update `src/components/PromptCard.vue` visible text**

Fix visible Chinese labels for title icon removal/replacement, copy/edit/delete/category/upload/preview/favorite states, toast messages, and confirm text.

- [ ] **Step 2: Update prompt card styles**

Refresh `.card`, `.title`, `.content`, `.tag`, `.thumb-zone`, `.favorite-button`, `.actions`, `.category-menu`, drag states, expanded states, and focus states while preserving collapsed height and thumbnail dimensions.

- [ ] **Step 3: Run prompt-card tests**

Run:

```powershell
pnpm test -- tests/components/PromptCard.test.ts tests/stores/library.test.ts
```

Expected: tests pass. In particular, single click expansion, double click copy, favorite click isolation, and drag sorting behavior remain intact.

## Task 5: Refresh Dialogs And Settings

- [ ] **Step 1: Update `src/components/PromptEditor.vue`**

Improve modal, form controls, action row, and labels. Preserve `onSave`, `onPickImage`, and close behavior.

- [ ] **Step 2: Update `src/components/SettingsModal.vue`**

Improve grouped API/import/version panels, inputs, buttons, scroll behavior, long-text wrapping, and labels. Preserve all handlers and `disabled` bindings.

- [ ] **Step 3: Update `src/components/CategoryDialog.vue` and `src/components/ConfirmDialog.vue` if needed**

Match modal surface, buttons, input, and overlay styling. Preserve emitted events.

- [ ] **Step 4: Run dialog tests**

Run:

```powershell
pnpm test -- tests/components/PromptEditor.test.ts tests/components/SettingsModal.test.ts tests/components/FloatingActionDialog.test.ts
```

Expected: tests pass.

## Task 6: Refresh Tool Panels And Floating Dialog

- [ ] **Step 1: Update `src/components/ReverseImagePanel.vue`**

Improve upload zone, image preview frame, action row, result textarea, error state, and labels.

- [ ] **Step 2: Update `src/components/FastCompressionPanel.vue`**

Improve upload zone, file name, target row, button, progress bar, status/error states, and labels.

- [ ] **Step 3: Update `src/components/FloatingActionDialog.vue`**

Improve modal action cards and labels. Preserve image/video branching and disabled reverse-video button.

- [ ] **Step 4: Run tool tests**

Run:

```powershell
pnpm test -- tests/components/ReverseImagePanel.test.ts tests/components/FastCompressionPanel.test.ts tests/components/FloatingActionDialog.test.ts tests/components/FloatButton.test.ts
```

Expected: tests pass.

## Task 7: Full Verification And Browser QA

- [ ] **Step 1: Run full automated verification**

Run:

```powershell
pnpm check
```

Expected: pass.

- [ ] **Step 2: Use browser verification**

Open `http://127.0.0.1:1422/` and inspect:

- Main prompt library at 720px by 520px.
- Sidebar active states and category scroll.
- Search bar text fit.
- Prompt card collapsed and expanded states.
- Settings modal scroll and grouped sections.
- Prompt editor modal.
- Reverse image panel.
- Compression panel.
- Floating action dialog when reachable from tests or UI state.

Expected: no overlapping text, no missing controls, no horizontal app-shell scroll, all overflowing panels remain scrollable.

- [ ] **Step 3: Final git diff review**

Run:

```powershell
git diff -- src docs
```

Expected: changes are visual/text/accessibility only, with no store or IPC behavior changes unless explicitly justified.
