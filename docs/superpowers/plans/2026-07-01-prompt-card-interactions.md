# Prompt Card Interactions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add full-content expansion, double-click copy, long-press sorting, and favorite filtering to prompt cards.

**Architecture:** Extend the prompt data model with `favorite` and `order`. Keep sorting/filtering in the Pinia library store, keep per-card interactions in `PromptCard.vue`, and let `App.vue` wire drag reorder events into store actions.

**Tech Stack:** Vue 3, Pinia, TypeScript, Vitest, Vue Test Utils.

---

### Task 1: Store Data And Filtering

**Files:**
- Modify: `src/types/index.ts`
- Modify: `src/stores/library.ts`
- Test: `tests/stores/library.test.ts`

- [ ] Add `favorite` and `order` to `Prompt`.
- [ ] Normalize old prompt data with default `favorite: false` and index-based `order`.
- [ ] Sort filtered prompts by `order`.
- [ ] Add fixed favorite filter id.
- [ ] Add `toggleFavorite(id)` and `movePromptBefore(draggedId, targetId)`.
- [ ] Run `pnpm vitest run tests/stores/library.test.ts`.

### Task 2: Card Interaction UI

**Files:**
- Modify: `src/components/PromptCard.vue`
- Test: `tests/components/PromptCard.test.ts`

- [ ] Expand card to full content on single click.
- [ ] Keep double click copy behavior without accidental single-click toggling.
- [ ] Add star button that toggles favorite without expanding.
- [ ] Add long-press drag events for prompt sorting.
- [ ] Run `pnpm vitest run tests/components/PromptCard.test.ts`.

### Task 3: Category Tree And App Wiring

**Files:**
- Modify: `src/components/CategoryTree.vue`
- Modify: `src/App.vue`
- Test: `tests/components/AppSidebar.test.ts`
- Test: `tests/components/App.test.ts`

- [ ] Add fixed Favorites entry to the category tree.
- [ ] Wire `reorder-before` from `PromptCard` to `lib.movePromptBefore`.
- [ ] Run `pnpm vitest run tests/components/AppSidebar.test.ts tests/components/App.test.ts`.

### Task 4: Full Verification

**Files:**
- No production file edits expected.

- [ ] Run `pnpm check`.
- [ ] Report changed files and verification results.
