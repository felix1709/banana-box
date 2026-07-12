pub mod model;
pub mod report;
pub mod repository;

use crate::{
    app_state::{AppServices, StartupGate},
    command_auth::MainArgs,
};
use model::{
    CreateDailyTaskInput, DailyTaskDayDto, ReorderDailyGroupsInput, ReorderDailyTasksInput,
    UpdateDailyTaskInput,
};
use std::sync::Arc;
use tauri::Manager;

#[cfg(test)]
mod tests;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoadDailyTaskDayCommandArgs {
    local_date: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateDailyTaskCommandArgs {
    input: CreateDailyTaskInput,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateDailyTaskCommandArgs {
    input: UpdateDailyTaskInput,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteDailyTaskCommandArgs {
    task_id: String,
    local_date: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReorderDailyGroupsCommandArgs {
    input: ReorderDailyGroupsInput,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReorderDailyTasksCommandArgs {
    input: ReorderDailyTasksInput,
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
pub async fn load_daily_task_day(
    window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    args: MainArgs<LoadDailyTaskDayCommandArgs>,
) -> Result<DailyTaskDayDto, String> {
    let local_date = args.0.local_date;
    let (db, _permit) = ready_services(&window, &gate)?;
    run_db(db, move |db| repository::load_day(db, &local_date)).await
}

#[tauri::command]
pub async fn create_daily_task(
    window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    args: MainArgs<CreateDailyTaskCommandArgs>,
) -> Result<DailyTaskDayDto, String> {
    let input = args.0.input;
    let (db, _permit) = ready_services(&window, &gate)?;
    run_db(db, move |db| repository::create_task(db, input)).await
}

#[tauri::command]
pub async fn update_daily_task(
    window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    args: MainArgs<UpdateDailyTaskCommandArgs>,
) -> Result<DailyTaskDayDto, String> {
    let input = args.0.input;
    let (db, _permit) = ready_services(&window, &gate)?;
    run_db(db, move |db| repository::update_task(db, input)).await
}

#[tauri::command]
pub async fn delete_daily_task(
    window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    args: MainArgs<DeleteDailyTaskCommandArgs>,
) -> Result<DailyTaskDayDto, String> {
    let DeleteDailyTaskCommandArgs {
        task_id,
        local_date,
    } = args.0;
    let (db, _permit) = ready_services(&window, &gate)?;
    run_db(db, move |db| {
        repository::delete_task(db, &local_date, &task_id)
    })
    .await
}

#[tauri::command]
pub async fn reorder_daily_groups(
    window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    args: MainArgs<ReorderDailyGroupsCommandArgs>,
) -> Result<DailyTaskDayDto, String> {
    let input = args.0.input;
    let (db, _permit) = ready_services(&window, &gate)?;
    run_db(db, move |db| {
        repository::reorder_groups(db, &input.local_date, input.group_ids)
    })
    .await
}

#[tauri::command]
pub async fn reorder_daily_tasks(
    window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    args: MainArgs<ReorderDailyTasksCommandArgs>,
) -> Result<DailyTaskDayDto, String> {
    let input = args.0.input;
    let (db, _permit) = ready_services(&window, &gate)?;
    run_db(db, move |db| {
        repository::reorder_tasks(db, &input.local_date, &input.group_id, input.task_ids)
    })
    .await
}
