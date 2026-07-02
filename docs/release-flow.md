# Banana Box Release Flow

## Root Cause Found

The app could not reliably update to the newest build for three reasons:

1. The local project folder currently has no `.git` directory, so code cannot be committed or pushed from this folder.
2. The app version was still `0.1.1`, and the latest GitHub Release was also `v0.1.1`, so the app correctly reported that no newer version existed.
3. The previous update flow was a manual installer download flow. It checked GitHub Releases and opened a browser link instead of installing inside the app.

## Required Release Steps

Every release must do these steps in order:

1. Confirm Git is available:
   ```powershell
   git rev-parse --show-toplevel
   git status --short
   git remote -v
   ```
2. Bump the version. Do not reuse an old version.
3. Keep these files in sync:
   - `package.json`
   - `src-tauri/tauri.conf.json`
   - `src-tauri/Cargo.toml`
   - `src-tauri/Cargo.lock`
4. Run verification:
   ```powershell
   pnpm check
   cargo test --manifest-path src-tauri\Cargo.toml
   ```
5. Build signed updater artifacts:
   ```powershell
   $env:TAURI_SIGNING_PRIVATE_KEY = Get-Content -Raw C:\Users\admin\.tauri\banana-box-updater.key
   $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = Get-Content -Raw C:\Users\admin\.tauri\banana-box-updater.password
   pnpm tauri build
   pnpm release:manifest
   ```
6. Commit and push:
   ```powershell
   git add .
   git commit -m "chore: release vX.Y.Z"
   git push
   ```
7. Create the GitHub Release with tag `vX.Y.Z`.
8. Upload generated installer, signature, and updater manifest assets:
   - `src-tauri/target/release/bundle/nsis/banana-box_X.Y.Z_x64-setup.exe`
   - `src-tauri/target/release/bundle/nsis/banana-box_X.Y.Z_x64-setup.exe.sig`
   - `src-tauri/target/release/bundle/msi/banana-box_X.Y.Z_x64_en-US.msi`
   - `src-tauri/target/release/bundle/msi/banana-box_X.Y.Z_x64_en-US.msi.sig`
   - `src-tauri/target/release/bundle/latest.json`
9. Verify the app update check sees the new release and can download/install it inside the app.

## Current In-App Update Behavior

The app currently checks the Tauri updater endpoint configured in `src-tauri/tauri.conf.json`:

```text
https://github.com/felix1709/banana-box/releases/latest/download/latest.json
```

The settings page calls the Tauri updater plugin. When an update exists, it downloads and installs the signed NSIS installer, then relaunches the app.

## Future Automatic Update Requirements

The updater private key and password are stored outside the repository:

```text
C:\Users\admin\.tauri\banana-box-updater.key
C:\Users\admin\.tauri\banana-box-updater.password
```

Do not commit these files. Losing either file means already-installed apps cannot trust future updates signed by a different key.
