# Prompt Card Interactions Design

## Goal

Improve prompt card interactions in Banana Box:

- single click expands a prompt and shows the full content;
- double click copies the prompt;
- long press starts drag sorting;
- a fixed Favorites category filters starred prompts;
- each prompt card has a star toggle in the top-right corner.

## Behavior

Prompt cards keep their compact collapsed layout by default. A single left click expands one card at a time and shows the full prompt content. Double click copies the prompt content and does not open the editor or change categories.

Each card has a star button in the top-right corner. The star is hollow when the prompt is not favorited and filled when it is favorited. Clicking the star only toggles favorite state and does not expand the card.

Long pressing the card for about 400ms enters drag mode. While dragging, the user can drop the prompt on another prompt to reorder it. Sorting is stored on prompt data so the order persists.

The category tree gets a fixed Favorites entry. This is a filter, not a normal category, so it cannot be deleted. It shows prompts whose favorite state is true.

## Data Model

`Prompt` adds:

- `favorite: boolean`
- `order: number`

Old library data is normalized on load and hydrate. Missing `favorite` becomes `false`; missing `order` uses the prompt's current list index. Adding a prompt assigns the next order value.

## Implementation Notes

The store owns filtering, sorting, favorite toggling, and reorder persistence. `PromptCard` owns local pointer timing and emits a reorder event. `App.vue` connects card drag events to the store.

## Testing

Vitest tests cover:

- old prompt normalization;
- favorites filtering;
- star toggling;
- ordered prompt output;
- drag reorder store behavior;
- single click full-content expansion;
- double click copy behavior;
- star click not expanding the card.
