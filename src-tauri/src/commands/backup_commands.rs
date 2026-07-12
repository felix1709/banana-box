use crate::{
    app_state::{AppServices, StartupGate},
    command_auth::MainArgs,
    legacy_import::{BackupStagingCoordinator, LegacyImportCommit, LegacyImportPreview},
};
use tauri::{Manager, WebviewWindow};

const APP_SERVICES_UNAVAILABLE: &str = "STARTUP_NOT_READY";
const IMPORT_COORDINATOR_UNAVAILABLE: &str = "IMPORT_STAGING_UNAVAILABLE";
const DATA_DIR_UNAVAILABLE: &str = "IMPORT_STAGING_UNAVAILABLE";

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct InspectLegacyImportCommandArgs {
    path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CommitLegacyImportCommandArgs {
    token: String,
    overwrite_credential: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DiscardLegacyImportPreviewCommandArgs {
    token: String,
}

#[tauri::command]
pub fn inspect_legacy_import(
    window: WebviewWindow,
    gate: tauri::State<StartupGate>,
    args: MainArgs<InspectLegacyImportCommandArgs>,
) -> Result<LegacyImportPreview, String> {
    gate.require_ready()?;
    let services = window
        .app_handle()
        .try_state::<AppServices>()
        .ok_or_else(|| APP_SERVICES_UNAVAILABLE.to_string())?;
    let _permit = services.operations.enter_user()?;
    let staging = window
        .app_handle()
        .try_state::<BackupStagingCoordinator>()
        .ok_or_else(|| IMPORT_COORDINATOR_UNAVAILABLE.to_string())?;
    let mut preview = staging.inspect(std::path::Path::new(&args.0.path), false)?;
    if preview.has_api_key {
        preview.credential_conflict = !services.providers.get("reverse-image")?.needs_credentials;
    }
    Ok(preview)
}

#[tauri::command]
pub fn commit_legacy_import(
    window: WebviewWindow,
    gate: tauri::State<StartupGate>,
    args: MainArgs<CommitLegacyImportCommandArgs>,
) -> Result<LegacyImportCommit, String> {
    gate.require_ready()?;
    let services = window
        .app_handle()
        .try_state::<AppServices>()
        .ok_or_else(|| APP_SERVICES_UNAVAILABLE.to_string())?;
    let _permit = services.operations.enter_user()?;
    let staging = window
        .app_handle()
        .try_state::<BackupStagingCoordinator>()
        .ok_or_else(|| IMPORT_COORDINATOR_UNAVAILABLE.to_string())?;
    let data_dir = window
        .app_handle()
        .path()
        .app_data_dir()
        .map_err(|_| DATA_DIR_UNAVAILABLE.to_string())?;
    staging.commit(
        &data_dir,
        &services.providers,
        &args.0.token,
        args.0.overwrite_credential,
    )
}

#[tauri::command]
pub fn discard_legacy_import_preview(
    window: WebviewWindow,
    gate: tauri::State<StartupGate>,
    args: MainArgs<DiscardLegacyImportPreviewCommandArgs>,
) -> Result<(), String> {
    gate.require_ready()?;
    let services = window
        .app_handle()
        .try_state::<AppServices>()
        .ok_or_else(|| APP_SERVICES_UNAVAILABLE.to_string())?;
    let _permit = services.operations.enter_user()?;
    let staging = window
        .app_handle()
        .try_state::<BackupStagingCoordinator>()
        .ok_or_else(|| IMPORT_COORDINATOR_UNAVAILABLE.to_string())?;
    staging.discard(&args.0.token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_command_arguments_reject_unknown_fields() {
        assert!(serde_json::from_value::<InspectLegacyImportCommandArgs>(
            serde_json::json!({ "path": "C:/temp/legacy.zip" }),
        )
        .is_ok());
        assert!(serde_json::from_value::<InspectLegacyImportCommandArgs>(
            serde_json::json!({ "sourcePath": "C:/temp/legacy.zip" }),
        )
        .is_err());
        assert!(serde_json::from_value::<CommitLegacyImportCommandArgs>(
            serde_json::json!({ "token": "token", "overwriteCredential": false, "key": "no" }),
        )
        .is_err());
        assert!(
            serde_json::from_value::<DiscardLegacyImportPreviewCommandArgs>(
                serde_json::json!({ "token": "token", "extra": true }),
            )
            .is_err()
        );
    }
}
