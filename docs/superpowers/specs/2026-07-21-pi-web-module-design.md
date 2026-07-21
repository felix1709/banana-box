# PI-Web Module Design

## Goal

Add a dedicated PI-Web module to Banana Box so non-technical users can launch the `@agegr/pi-web` experience with one click. Banana Box will manage startup, environment checks, status display, and app-level updates. PI-Web itself will open in a separate window or browser page because it is a full web application and needs more space than the Banana Box floating panel.

## User Experience

The Banana Box sidebar gets a new `PI-Web` entry. Selecting it opens a compact control page inside Banana Box using the existing dark cyan Banana Box visual language.

The control page shows:

- Current PI-Web service status: not started, checking, starting, running, stopped, or error.
- Primary action button: start PI-Web, open PI-Web, retry, or stop service depending on status.
- Local access address, expected to be `http://127.0.0.1:30141` for the first version.
- Diagnostics area shown only when something is missing or startup fails.
- Plain Chinese guidance for missing software, with official download links where external installation is unavoidable.

When the user clicks the main action:

1. Banana Box checks whether the bundled PI-Web runtime is available.
2. Banana Box checks whether the required local environment is usable.
3. Banana Box starts PI-Web as a background child process.
4. Banana Box waits for the local endpoint to respond.
5. Banana Box opens PI-Web in a separate window or the user's default browser.
6. Banana Box remains available as a small control console.

## Product Decision

PI-Web should not be embedded directly inside the Banana Box content area. The current Banana Box panel is optimized for compact prompt, project, and task workflows. PI-Web is a full Web UI and would become cramped if rendered inside the 720x520 floating panel.

Banana Box should act as the launcher and manager. PI-Web should act as the main working surface.

## Update Strategy

PI-Web updates follow Banana Box app releases.

For the first version, Banana Box will not hot-update PI-Web independently from the installed app. This keeps the open-source app easier to support because each Banana Box release contains a known PI-Web package version that can be tested before publishing.

When publishing a new Banana Box version, the release process should include updating the bundled PI-Web package and verifying that the installed app can launch it.

## Runtime Strategy

The user-facing goal is that ordinary users do not need to run:

```powershell
npx @agegr/pi-web@latest
```

Banana Box should bundle the real PI-Web runtime needed by the app. If any external dependency remains unavoidable, Banana Box must detect it before launch and show a clear install link instead of failing silently.

The first implementation should prefer a conservative bundled-runtime approach over relying on the user's global Node/npm installation. If a fully bundled runtime proves too large or fragile, the fallback must still provide environment detection and beginner-friendly install links.

## Backend Responsibilities

The Tauri backend owns the PI-Web service lifecycle:

- Inspect bundled PI-Web runtime presence.
- Detect required tools and versions.
- Detect whether the configured port is already in use.
- Start PI-Web as a background process.
- Poll the local HTTP endpoint until ready or timeout.
- Stop the process when requested or when the app exits.
- Return structured status objects to the frontend.

Errors should be stable machine-readable codes with friendly Chinese messages in the UI.

## Frontend Responsibilities

The Vue frontend owns the PI-Web control page:

- Add `pi-web` to the UI active tool list.
- Add a sidebar item labeled `PI-Web`.
- Render a focused control page, not a marketing page.
- Use existing Banana Box tokens such as `--bb-bg`, `--bb-surface`, `--bb-primary`, `--bb-border`, and `--bb-text`.
- Show loading, running, missing environment, port conflict, and error states.
- Avoid text overflow in the small panel.
- Keep the page scrollable when diagnostics are long.

## Environment Detection

The first version should report at least:

- Bundled PI-Web runtime present or missing.
- Whether a launch command can be resolved.
- Whether the configured port is free or already serving something.
- Whether the PI-Web endpoint becomes healthy after startup.

If the implementation depends on external Node.js, the diagnostics must include a Node.js install link and a short explanation that Node.js is the JavaScript runtime PI-Web needs.

## Failure Handling

Port conflict:

- Show that port `30141` is already in use.
- Try to detect whether the existing service looks like PI-Web.
- If it looks like PI-Web, offer to open it.
- If it does not, ask the user to close the other program or retry later.

Startup timeout:

- Stop the child process if Banana Box started it.
- Show a short diagnostic log excerpt.
- Offer retry.

Missing runtime:

- Show that the bundled PI-Web files are missing.
- Ask the user to reinstall or update Banana Box.

Missing external dependency:

- Show the missing dependency name.
- Provide a direct official install link.
- Do not ask the user to run complex terminal commands.

## Testing And Verification

Implementation should include focused tests for:

- PI-Web status types and status transitions.
- Environment detection result parsing.
- Port conflict behavior.
- Frontend rendering for running, missing dependency, and error states.

Manual verification should include:

- Start Banana Box.
- Open the `PI-Web` sidebar item.
- Click the main button.
- Confirm PI-Web opens separately.
- Confirm the Banana Box control page shows running status.
- Stop or close Banana Box and confirm the child process is cleaned up.

## Out Of Scope For First Version

- Independent PI-Web hot updates.
- Multi-version PI-Web management.
- User-selectable PI-Web port.
- Deep embedding of PI-Web inside the Banana Box panel.
- Editing PI-Web source code inside Banana Box.
