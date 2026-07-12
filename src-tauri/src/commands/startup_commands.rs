use crate::{
    app_state::{StartupGate, StartupStatus},
    command_auth::MainArgs,
    migration,
};
use tauri::Manager;

const STARTUP_DATA_DIR_UNAVAILABLE: &str = "STARTUP_DATA_DIR_UNAVAILABLE";

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GetStartupStatusCommandArgs {}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AcknowledgeMigrationSummaryCommandArgs {}

#[tauri::command]
pub fn get_startup_status(
    gate: tauri::State<StartupGate>,
    _args: MainArgs<GetStartupStatusCommandArgs>,
) -> Result<StartupStatus, String> {
    gate.status()
}

#[tauri::command]
pub fn acknowledge_migration_summary(
    app: tauri::AppHandle,
    gate: tauri::State<StartupGate>,
    _args: MainArgs<AcknowledgeMigrationSummaryCommandArgs>,
) -> Result<(), String> {
    gate.require_ready()?;
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| STARTUP_DATA_DIR_UNAVAILABLE.to_string())?;
    migration::acknowledge_migration_summary(&data_dir)?;
    gate.clear_migration_summary()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_status_command_accepts_only_an_empty_payload() {
        assert!(
            serde_json::from_value::<GetStartupStatusCommandArgs>(serde_json::json!({})).is_ok()
        );
        assert!(serde_json::from_value::<GetStartupStatusCommandArgs>(
            serde_json::json!({ "unexpected": true }),
        )
        .is_err());
    }

    #[test]
    fn migration_acknowledgement_command_accepts_only_an_empty_payload() {
        assert!(serde_json::from_value::<AcknowledgeMigrationSummaryCommandArgs>(
            serde_json::json!({}),
        )
        .is_ok());
        assert!(
            serde_json::from_value::<AcknowledgeMigrationSummaryCommandArgs>(
                serde_json::json!({ "unexpected": true }),
            )
            .is_err()
        );
    }
}
