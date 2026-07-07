# Banana Box Frontend Visual Refresh Design

## Goal

Improve Banana Box's frontend visual quality while preserving all current behavior.

This is a visual refresh, not a feature rewrite. The app should feel more polished, compact, and professional, but existing prompt library, category, favorite, drag sorting, image reverse prompt, compression, settings, import/export, update, toast, preview, and floating action flows must continue to work the same way.

## Recommended Direction

Use a refined lightweight desktop-tool style with a small amount of macOS-like surface depth.

The interface should feel like a focused utility panel: calm, fast, readable, and made for repeated daily use. It should not become a marketing-style page, a decorative dashboard, or a highly animated AI showcase.

## Non-Functional Boundaries

The implementation must not change:

- Pinia store data shape or persistence logic.
- Tauri IPC command names or payloads.
- Prompt copy, edit, delete, favorite, category assignment, image attachment, image preview, and drag sorting behavior.
- Reverse-image API settings or request flow.
- Compression file picking, target size input, save dialog, and progress flow.
- Settings import/export, batch import, hotkey, API model, and update-check behavior.
- Existing tests' behavioral expectations.

Allowed changes:

- CSS and visual class refinements.
- Small template changes for better hierarchy, labels, and accessibility.
- Correcting visible text where it is currently garbled, as long as the meaning matches the existing UI.
- Adding design tokens through CSS custom properties.
- Adding focus, hover, disabled, loading, empty, and error state styling.

## Visual System

### Color

Use a restrained light theme:

- App background: cool off-white.
- Main surfaces: white and subtle blue-gray panels.
- Borders: soft slate-gray, visible but quiet.
- Primary action: confident blue.
- Danger action: red, reserved for destructive actions.
- Favorite accent: amber.
- Success/status: green or blue depending on context.

Colors should be defined as CSS variables so components share one vocabulary.

### Typography

Use the existing system font stack for now to reduce runtime risk and avoid external font loading in the desktop app.

Improve perceived polish through:

- Clear size scale: 11px, 12px, 13px, 14px, 16px.
- Stronger title/body contrast.
- Better line-height for prompt content.
- Tabular numbers where progress, counts, or file sizes are introduced later.

### Layout And Density

Keep the current 720px by 520px panel model.

Maintain compact density, but make spacing more intentional:

- 4px base spacing for tight controls.
- 8px standard gaps.
- 12px to 16px section padding.
- Stable dimensions for prompt cards and thumbnails.
- Scrollable panels wherever content can exceed the visible area.

### Radius And Depth

Use a restrained radius scale:

- 4px for small chips and compact controls.
- 6px for buttons and inputs.
- 8px for cards and panels.
- 10px to 12px for dialogs.

Use shadows only for elevated layers: app shell, dialogs, dropdown menus, drag-floating cards, toast, and preview overlays.

## Component Treatment

### App Shell

The app shell keeps the current top drag strip, topbar, sidebar, and content split. Visual updates should make the shell feel integrated:

- Cleaner topbar with subtle border and search prominence.
- Softer drag strip marker.
- Main content background slightly separated from sidebar.
- Consistent scrollbar behavior.

### Sidebar

The sidebar remains compact and functional:

- Tool buttons should look like navigation items, not plain buttons.
- Active tool has a clear selected state.
- The create-prompt button remains immediately visible.
- Category list remains scrollable and compact.
- Favorites remains visually distinct but not oversized.

### Prompt Cards

Prompt cards are the main product surface.

Collapsed cards must preserve their fixed height and thumbnail layout. Expanded cards must continue to reveal full content and action buttons without clipping.

Visual improvements:

- Softer border and hover elevation.
- Cleaner title/content/tag hierarchy.
- More refined thumbnail empty state.
- Favorite star should look intentional and accessible.
- Drag-floating state should feel clearly lifted.
- Category dropdown should remain scrollable and layered above cards.

### Tool Panels

Reverse image and compression panels should look like work surfaces:

- Upload zones get clearer drop affordance.
- Primary actions stand out.
- Disabled buttons look disabled.
- Error messages use consistent danger styling.
- Progress bar remains stable and readable.

### Dialogs

Prompt editor, settings, category dialog, confirm dialog, and floating action dialog should share a modal language:

- Stronger overlay scrim.
- Consistent dialog radius, border, padding, and shadow.
- Clear section grouping.
- Inputs and selects share the same control styling.
- Settings remains vertically scrollable.
- Long paths, model names, API text, and import status wrap safely.

### Toast And Preview

Toast should be compact and readable with a refined shadow.

Image preview should keep the current click-to-close behavior and preserve original aspect ratio. The mask can become more polished, but image sizing must not change functionally.

## Accessibility And Interaction

Required checks:

- Focus states visible on buttons, inputs, cards, and upload zones.
- Disabled states visually distinct and non-interactive.
- Button text must not overflow.
- Touch/click targets stay at least 28px in this compact desktop panel where feasible.
- No content hidden behind fixed areas.
- No horizontal scrolling in the app shell.
- All scrollable areas remain reachable by mouse wheel.
- Motion stays short, around 120ms to 180ms, and avoids layout-shifting animations.

## Implementation Plan Preview

Implementation should happen in small, low-risk slices:

1. Add global CSS tokens and shared primitive styling in `src/styles/main.css`.
2. Refresh app shell, topbar, sidebar, and content surfaces.
3. Refresh prompt cards while preserving card height, expansion, drag, and category dropdown behavior.
4. Refresh dialogs and form controls.
5. Refresh reverse-image and compression tool panels.
6. Refresh toast, image preview, and floating action dialog.
7. Run tests and browser visual verification.

## Verification

Run:

```powershell
pnpm check
```

Then visually verify:

- Prompt list scrolls.
- Search still filters prompts.
- Single click expands a card.
- Double click copies a prompt.
- Favorite toggle does not expand the card.
- Long press drag sorting still works.
- Category dropdown appears above the card and scrolls if needed.
- Prompt editor can create and edit prompts.
- Settings modal scrolls and all sections remain reachable.
- Reverse image upload/drop/paste states remain visible.
- Compression target input, progress bar, and errors remain visible.
- Floating action dialog still routes image/video actions correctly.
- Toast appears and disappears.
- Image preview opens and closes.
