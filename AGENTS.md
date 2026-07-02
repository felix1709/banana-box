# Banana Box Codex Memory

## Language And Collaboration

- Default to Chinese when explaining work to the user.
- The user is a programming beginner, so explain changes in small, concrete steps.
- When changing code, explain what changed, why it changed, and how to verify it.

## Submit And Publish Rule

When the user says "提交发布", "提交&发布", "发布", or asks to ship a new app version:

0. First read `fabu.MD` in the project root and follow it as the concrete release checklist for Banana Box. If `fabu.MD` and this section differ, stop and explain the difference before publishing.
1. Treat this as a request to both commit code to the Git repository and publish a new application version.
2. First verify the current folder is a Git repository with `git rev-parse --show-toplevel`.
3. If the folder is not a Git repository, do not initialize or rewrite Git history automatically. Stop the Git commit/push part and clearly tell the user that `.git` is missing.
4. If Git is available, configure/verify the preferred identity:
   - `user.name`: `felix1709`
   - `user.email`: `331631382@qq.com`
5. Before release, bump the app version. Do not publish a new build with the same version number as the previous release.
6. Keep these version files in sync:
   - `package.json`
   - `src-tauri/tauri.conf.json`
   - `src-tauri/Cargo.toml`
   - `src-tauri/Cargo.lock`
7. Run fresh verification before committing or publishing:
   - `pnpm check`
   - `cargo test --manifest-path src-tauri\Cargo.toml`
   - set updater signing env vars from `C:\Users\admin\.tauri\banana-box-updater.key` and `C:\Users\admin\.tauri\banana-box-updater.password`
   - `pnpm tauri build`
   - `pnpm release:manifest`
8. Commit the verified changes with a clear release message, then push to the configured remote.
9. Create or update the GitHub Release for the new tag and upload the generated installer assets.
10. Never use `git push --force` unless the user explicitly asks for it.

## UI Overflow Rule

- Any page, modal, panel, list, menu, or form that can exceed the visible app area must provide a scrollbar, mouse-wheel scrolling, pagination, or a split page.
- Prompt library pages must remain browseable when many prompts or categories exist.
- Settings must remain browseable when API, import/export, version update, or future sections exceed the window height.
- Prefer internal scrolling inside the overflowing panel instead of making controls disappear outside the app window.

## Release Update Rule

- A release is not complete if the installed app cannot detect or obtain the newest version.
- Every release must verify:
  - the local version is newer than the previous release,
  - the GitHub Release tag matches the local version,
  - installer assets are attached to the release,
  - the app's update check can see that release.
- If automatic in-app updating is implemented, the updater manifest, signature files, and endpoint must be published together with installer assets.
- The updater private key and password live outside the repo:
  - `C:\Users\admin\.tauri\banana-box-updater.key`
  - `C:\Users\admin\.tauri\banana-box-updater.password`
- Never commit the private key or password.
