# Night Lab Visual Refresh Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn Banana Box into the approved A direction: a dark AI command-center interface with stronger depth, glow, and premium tool texture without changing existing features.

**Architecture:** This is a visual-only pass. Keep Vue component state, Pinia stores, IPC calls, persistence, import/export, update checking, drag sorting, image reverse, and compression behavior unchanged. Implement the direction through CSS variables and scoped component style updates.

**Tech Stack:** Vue 3, Tauri, Pinia, Vite, Vitest, CSS.

---

### Task 1: Dark Design Tokens

**Files:**
- Modify: `src/styles/main.css`

- [x] Replace the light Banana Box variables with dark Night Lab variables: deep navy surfaces, cyan primary accents, muted blue-gray text, dark modal surfaces, stronger shadows, and glow focus rings.
- [x] Keep the existing variable names so existing components continue to work.
- [x] Run `pnpm check` after style changes to verify no Vue or TypeScript behavior broke.

### Task 2: Main Shell And Navigation

**Files:**
- Modify: `src/App.vue`
- Modify: `src/components/SearchBar.vue`
- Modify: `src/components/AppSidebar.vue`
- Modify: `src/components/CategoryTree.vue`

- [x] Restyle the app shell as a dark command center with subtle radial glow, luminous drag strip, dark topbar, and darker sidebar.
- [x] Restyle tool buttons and category rows so active states are obvious without changing click handlers or data attributes.
- [x] Keep panel dimensions and scroll behavior stable.

### Task 3: Prompt Cards And Tool Panels

**Files:**
- Modify: `src/components/PromptCard.vue`
- Modify: `src/components/ReverseImagePanel.vue`
- Modify: `src/components/FastCompressionPanel.vue`

- [x] Make prompt cards feel raised and technical with dark gradients, cyan borders, improved hover/expanded states, and readable tags.
- [x] Restyle reverse-image and compression panels with dark upload zones, glowing primary buttons, and matching progress/result surfaces.
- [x] Keep drag, paste, upload, copy, edit, delete, favorite, category, reverse, and compress handlers unchanged.

### Task 4: Modals And Floating Dialogs

**Files:**
- Modify: `src/components/SettingsModal.vue`
- Modify: `src/components/PromptEditor.vue`
- Modify: `src/components/FloatingActionDialog.vue`
- Modify: `src/components/CategoryDialog.vue`
- Modify: `src/components/ConfirmDialog.vue`

- [x] Apply the same Night Lab modal treatment: dark blurred mask, raised dark dialog, cyan focus rings, readable section panels, and clear disabled states.
- [x] Keep overflow scrolling for tall settings content.

### Task 5: Verification

**Files:**
- No production file changes.

- [x] Run `pnpm check`.
- [x] Start the Vite preview or reuse the running dev server.
- [x] Capture browser screenshots for prompt library, expanded card, settings, reverse image, and compression.
- [x] Inspect screenshots for text overlap, unreadable contrast, missing scroll areas, and blank screens.
