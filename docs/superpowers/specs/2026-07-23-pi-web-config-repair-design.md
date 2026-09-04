# PI-Web Config Repair Design

## Goal

Add a PI-Web page repair flow that helps each Banana Box user configure their own PI-Web API key when PI reports "No API key found for the selected model."

## User Flow

1. The user opens the PI-Web page in Banana Box.
2. The user clicks "检测配置" to inspect the current Windows user's PI config directory.
3. Banana Box reports whether `settings.json`, `models.json`, and `auth.json` exist, which provider/model is selected, and whether an API key is available.
4. If the API key is missing, the user enters their own API key in Banana Box.
5. The user clicks "一键修复".
6. Banana Box writes only the missing PI-Web-compatible config values for the current Windows user, stops PI-Web if needed, refreshes diagnostics, and lets the user start PI-Web again.

## Data And Files

PI-Web reads config from the current OS user's home directory:

- `~/.pi/agent/settings.json`
- `~/.pi/agent/models.json`
- `~/.pi/agent/auth.json`

Banana Box will not store the PI-Web API key in its own database. It will write the key only to PI's local `auth.json` using the selected provider key.

## Default Provider

The first implementation supports the current Banana Box PI-Web default:

- Provider: `雷火`
- Model: `glm-5.2`
- Base URL: `https://ai.leihuo.netease.com/v1`
- API: `openai-completions`

If the user already has another provider/model, Banana Box will show it and avoid silently overwriting it. The repair action creates or fills the Banana Box default only when the selected PI config is missing or incomplete.

## Safety Rules

- Do not print API keys.
- Do not log API keys.
- Do not include API keys in release assets.
- Do not overwrite unrelated PI providers.
- Do not delete user PI config.
- Mask paths and state clearly enough for non-programmers to understand.

## UI

The PI-Web page gets a compact "配置修复" card with:

- Current config status.
- "检测配置" button.
- Password-style API key input.
- "一键修复" button.
- Short success/error feedback in Chinese.

The card uses Banana Box's existing dark cyan tool-panel style and internal scrolling where needed.

## Verification

- Rust tests cover missing config, repair file creation, preserving existing provider config, and API key not appearing in diagnostic output.
- Vue tests cover showing missing API key guidance and submitting the repair form.
- Existing PI-Web start/stop and chat health tests continue to pass.
