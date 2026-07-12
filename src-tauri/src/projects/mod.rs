pub mod model;
pub mod repository;

use crate::{
    app_state::{AppServices, StartupGate},
    command_auth::MainArgs,
};
use model::{
    CreateProjectInput, ProjectDto, SaveProjectWithStagesInput, SetProjectStageInput,
    UpdateProjectInput,
};
use std::sync::Arc;
use tauri::Manager;

#[cfg(test)]
mod tests;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListProjectsCommandArgs {}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateProjectCommandArgs {
    input: CreateProjectInput,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateProjectCommandArgs {
    input: UpdateProjectInput,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveProjectWithStagesCommandArgs {
    input: SaveProjectWithStagesInput,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetProjectStageCommandArgs {
    input: SetProjectStageInput,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchiveProjectCommandArgs {
    project_id: String,
    archived: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteProjectCommandArgs {
    project_id: String,
}

async fn run_db<T, F>(db: Arc<crate::db::Database>, operation: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&crate::db::Database) -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || operation(&db))
        .await
        .map_err(|error| error.to_string())?
}

fn ready_services(
    window: &tauri::WebviewWindow,
    gate: &StartupGate,
) -> Result<(Arc<crate::db::Database>, crate::app_state::OperationPermit), String> {
    gate.require_ready()?;
    let services = window
        .app_handle()
        .try_state::<AppServices>()
        .ok_or_else(|| "STARTUP_NOT_READY".to_string())?;
    let permit = services.operations.enter_user()?;
    Ok((services.database.clone(), permit))
}

#[tauri::command]
pub async fn list_projects(
    window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    args: MainArgs<ListProjectsCommandArgs>,
) -> Result<Vec<ProjectDto>, String> {
    let ListProjectsCommandArgs {} = args.0;
    let (db, _permit) = ready_services(&window, &gate)?;
    run_db(db, repository::list_projects).await
}

#[tauri::command]
pub async fn create_project(
    window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    args: MainArgs<CreateProjectCommandArgs>,
) -> Result<ProjectDto, String> {
    let (db, _permit) = ready_services(&window, &gate)?;
    let input = args.0.input;
    run_db(db, move |db| repository::create_project(db, input)).await
}

#[tauri::command]
pub async fn update_project(
    window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    args: MainArgs<UpdateProjectCommandArgs>,
) -> Result<ProjectDto, String> {
    let (db, _permit) = ready_services(&window, &gate)?;
    let input = args.0.input;
    run_db(db, move |db| repository::update_project(db, input)).await
}

#[tauri::command]
pub async fn save_project_with_stages(
    window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    args: MainArgs<SaveProjectWithStagesCommandArgs>,
) -> Result<ProjectDto, String> {
    let (db, _permit) = ready_services(&window, &gate)?;
    let input = args.0.input;
    run_db(db, move |db| {
        repository::save_project_with_stages(db, input)
    })
    .await
}

#[tauri::command]
pub async fn set_project_stage(
    window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    args: MainArgs<SetProjectStageCommandArgs>,
) -> Result<ProjectDto, String> {
    let (db, _permit) = ready_services(&window, &gate)?;
    let input = args.0.input;
    run_db(db, move |db| repository::set_project_stage(db, input)).await
}

#[tauri::command]
pub async fn archive_project(
    window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    args: MainArgs<ArchiveProjectCommandArgs>,
) -> Result<ProjectDto, String> {
    let (db, _permit) = ready_services(&window, &gate)?;
    let ArchiveProjectCommandArgs {
        project_id,
        archived,
    } = args.0;
    run_db(db, move |db| {
        repository::archive_project(db, &project_id, archived)
    })
    .await
}

#[tauri::command]
pub async fn delete_project(
    window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    args: MainArgs<DeleteProjectCommandArgs>,
) -> Result<(), String> {
    let (db, _permit) = ready_services(&window, &gate)?;
    let project_id = args.0.project_id;
    run_db(db, move |db| repository::delete_project(db, &project_id)).await
}
