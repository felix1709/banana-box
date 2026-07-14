use super::model::{
    apply_schedule_progress, main_stage_for_project_stages, main_stage_for_schedule,
    validate_and_sort_stages, validate_project_fields, validate_project_id,
    validate_stage_values, CreateProjectInput, ProjectDto, ProjectStageDto, SaveProjectStageInput,
    SaveProjectWithStagesInput, SetProjectStageInput, StageKey, UpdateProjectInput,
};
use crate::db::Database;
use chrono::{Local, Utc};
use rusqlite::{params, Connection};
use std::path::Path;

pub fn list_projects(db: &Database) -> Result<Vec<ProjectDto>, String> {
    db.with_connection(|connection| {
        let mut statement = connection
            .prepare(
                "SELECT id FROM projects ORDER BY archived, release_date, code COLLATE NOCASE, id",
            )
            .map_err(|error| error.to_string())?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;

        ids.into_iter()
            .map(|id| read_project(connection, &id))
            .collect()
    })
}

pub fn create_project(db: &Database, input: CreateProjectInput) -> Result<ProjectDto, String> {
    validate_project_fields(
        &input.code,
        &input.version,
        &input.name,
        &input.file_path,
        &input.release_date,
    )?;
    let today = schedule_date();
    let stages = apply_schedule_progress(&validate_and_sort_stages(&input.stages)?, today)?;
    let main_stage_key = main_stage_for_schedule(&stages, today)?;
    let project_id = uuid::Uuid::new_v4().to_string();
    let now = timestamp();

    db.with_transaction(|transaction| {
        transaction
            .execute(
                "INSERT INTO projects
                 (id, code, version, name, file_path, release_date, main_stage_key, archived, owner_user_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9, ?9)",
                params![
                    project_id,
                    input.code.trim(),
                    input.version.trim(),
                    input.name.trim(),
                    input.file_path.trim(),
                    input.release_date,
                    main_stage_key.as_str(),
                    input.owner_user_id.trim(),
                    now,
                ],
            )
            .map_err(map_sql_error)?;
        insert_stages(transaction, &project_id, &stages, &now)?;
        read_project(transaction, &project_id)
    })
}

pub fn update_project(db: &Database, input: UpdateProjectInput) -> Result<ProjectDto, String> {
    validate_project_id(&input.project_id)?;
    validate_project_fields(
        &input.code,
        &input.version,
        &input.name,
        &input.file_path,
        &input.release_date,
    )?;
    let now = timestamp();

    db.with_transaction(|transaction| {
        let main_stage_key = read_project(transaction, &input.project_id)?.main_stage_key;
        let changed = transaction
            .execute(
                "UPDATE projects
                 SET code = ?1, version = ?2, name = ?3, file_path = ?4, release_date = ?5,
                     main_stage_key = ?6, updated_at = ?7
                 WHERE id = ?8",
                params![
                    input.code.trim(),
                    input.version.trim(),
                    input.name.trim(),
                    input.file_path.trim(),
                    input.release_date,
                    main_stage_key.as_str(),
                    now,
                    input.project_id,
                ],
            )
            .map_err(map_sql_error)?;
        if changed == 0 {
            return Err("PROJECT_NOT_FOUND".into());
        }
        read_project(transaction, &input.project_id)
    })
}

pub fn set_project_public(
    db: &Database,
    project_id: &str,
    is_public: bool,
) -> Result<ProjectDto, String> {
    validate_project_id(project_id)?;
    let now = timestamp();
    let summary = if is_public { "设为公共项目" } else { "设为个人项目" };

    db.with_transaction(|transaction| {
        let changed = transaction
            .execute(
                "UPDATE projects
                 SET is_public = ?1, last_activity_summary = ?2, updated_at = ?3
                 WHERE id = ?4",
                params![i64::from(is_public), summary, now, project_id],
            )
            .map_err(|error| error.to_string())?;
        if changed == 0 {
            return Err("PROJECT_NOT_FOUND".into());
        }
        insert_activity(transaction, project_id, "", "", summary, &now)?;
        read_project(transaction, project_id)
    })
}

pub fn save_project_with_stages(
    db: &Database,
    input: SaveProjectWithStagesInput,
) -> Result<ProjectDto, String> {
    validate_project_id(&input.project_id)?;
    validate_project_fields(
        &input.code,
        &input.version,
        &input.name,
        &input.file_path,
        &input.release_date,
    )?;
    let today = schedule_date();
    let stages = apply_schedule_progress(&validate_and_sort_stages(&input.stages)?, today)?;
    let main_stage_key = main_stage_for_schedule(&stages, today)?;
    let now = timestamp();

    db.with_immediate_transaction(|transaction| {
        transaction
            .execute(
                "INSERT INTO projects
                 (id, code, version, name, file_path, release_date, main_stage_key, archived, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
                 ON CONFLICT(id) DO UPDATE SET
                   code = excluded.code,
                   version = excluded.version,
                   name = excluded.name,
                   file_path = excluded.file_path,
                   release_date = excluded.release_date,
                   main_stage_key = excluded.main_stage_key,
                   archived = excluded.archived,
                   updated_at = excluded.updated_at",
                params![
                    input.project_id,
                    input.code.trim(),
                    input.version.trim(),
                    input.name.trim(),
                    input.file_path.trim(),
                    input.release_date,
                    main_stage_key.as_str(),
                    i64::from(input.archived),
                    now,
                ],
            )
            .map_err(map_sql_error)?;
        upsert_stages(transaction, &input.project_id, &stages, &now)?;
        read_project(transaction, &input.project_id)
    })
}

pub fn set_project_stage(db: &Database, input: SetProjectStageInput) -> Result<ProjectDto, String> {
    validate_project_id(&input.project_id)?;
    validate_stage_values(&input.start_date, &input.end_date, input.progress)?;
    let progress = input.progress;
    let now = timestamp();

    db.with_transaction(|transaction| {
        let previous_progress = transaction
            .query_row(
                "SELECT progress FROM project_stages WHERE project_id = ?1 AND stage_key = ?2",
                params![input.project_id, input.stage_key.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|error| error.to_string())?;
        let changed = transaction
            .execute(
                "UPDATE project_stages
                 SET start_date = ?1, end_date = ?2, progress = ?3, updated_at = ?4
                 WHERE project_id = ?5 AND stage_key = ?6",
                params![
                    input.start_date,
                    input.end_date,
                    progress,
                    now,
                    input.project_id,
                    input.stage_key.as_str(),
                ],
            )
            .map_err(|error| error.to_string())?;
        if changed == 0 {
            return Err("PROJECT_STAGE_NOT_FOUND".into());
        }
        let summary = format!(
            "修改了 {} 进度 {}% -> {}%",
            input.stage_key.as_str(),
            previous_progress,
            progress,
        );
        transaction
            .execute(
                "UPDATE projects
                 SET last_activity_summary = ?1, last_activity_actor_name = ?2, updated_at = ?3
                 WHERE id = ?4",
                params![summary, input.actor_name.trim(), now, input.project_id],
            )
            .map_err(|error| error.to_string())?;
        insert_activity(
            transaction,
            &input.project_id,
            input.actor_user_id.trim(),
            input.actor_name.trim(),
            &summary,
            &now,
        )?;
        read_project(transaction, &input.project_id)
    })
}

pub fn archive_project(
    db: &Database,
    project_id: &str,
    archived: bool,
) -> Result<ProjectDto, String> {
    validate_project_id(project_id)?;
    let now = timestamp();

    db.with_transaction(|transaction| {
        let changed = transaction
            .execute(
                "UPDATE projects SET archived = ?1, updated_at = ?2 WHERE id = ?3",
                params![i64::from(archived), now, project_id],
            )
            .map_err(|error| error.to_string())?;
        if changed == 0 {
            return Err("PROJECT_NOT_FOUND".into());
        }
        read_project(transaction, project_id)
    })
}

pub fn delete_project(db: &Database, project_id: &str) -> Result<(), String> {
    validate_project_id(project_id)?;
    db.with_transaction(|transaction| {
        let deleted = transaction
            .execute("DELETE FROM projects WHERE id = ?1", [project_id])
            .map_err(|error| error.to_string())?;
        if deleted == 0 {
            return Err("PROJECT_NOT_FOUND".into());
        }
        Ok(())
    })
}

fn insert_stages(
    connection: &Connection,
    project_id: &str,
    stages: &[SaveProjectStageInput],
    now: &str,
) -> Result<(), String> {
    for (position, stage) in stages.iter().enumerate() {
        connection
            .execute(
                "INSERT INTO project_stages
                 (id, project_id, stage_key, position, start_date, end_date, progress, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    uuid::Uuid::new_v4().to_string(),
                    project_id,
                    stage.stage_key.as_str(),
                    position as i64,
                    stage.start_date,
                    stage.end_date,
                    stage.progress,
                    now,
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn upsert_stages(
    connection: &Connection,
    project_id: &str,
    stages: &[SaveProjectStageInput],
    now: &str,
) -> Result<(), String> {
    for (position, stage) in stages.iter().enumerate() {
        connection
            .execute(
                "INSERT INTO project_stages
                 (id, project_id, stage_key, position, start_date, end_date, progress, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(project_id, stage_key) DO UPDATE SET
                   position = excluded.position,
                   start_date = excluded.start_date,
                   end_date = excluded.end_date,
                   progress = excluded.progress,
                   updated_at = excluded.updated_at",
                params![
                    uuid::Uuid::new_v4().to_string(),
                    project_id,
                    stage.stage_key.as_str(),
                    position as i64,
                    stage.start_date,
                    stage.end_date,
                    stage.progress,
                    now,
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn read_project(connection: &Connection, project_id: &str) -> Result<ProjectDto, String> {
    let (
        id,
        code,
        version,
        name,
        file_path,
        release_date,
        main_stage_key,
        archived,
        owner_user_id,
        is_public,
        last_activity_summary,
        last_activity_actor_name,
        created_at,
        updated_at,
    ): (
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        i64,
        String,
        i64,
        String,
        String,
        String,
        String,
    ) = connection
        .query_row(
            "SELECT id, code, version, name, file_path, release_date, main_stage_key, archived,
                    owner_user_id, is_public, last_activity_summary, last_activity_actor_name,
                    created_at, updated_at
             FROM projects WHERE id = ?1",
            [project_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                    row.get(12)?,
                    row.get(13)?,
                ))
            },
        )
        .map_err(|error| error.to_string())?;

    let mut statement = connection
        .prepare(
            "SELECT id, stage_key, position, start_date, end_date, progress, updated_at
             FROM project_stages WHERE project_id = ?1 ORDER BY position",
        )
        .map_err(|error| error.to_string())?;
    let raw_stages = statement
        .query_map([project_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
            ))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let stages = raw_stages
        .into_iter()
        .map(
            |(id, stage_key, position, start_date, end_date, progress, updated_at)| {
                Ok(ProjectStageDto {
                    id,
                    stage_key: StageKey::from_db(&stage_key)?,
                    position,
                    start_date,
                    end_date,
                    progress,
                    updated_at,
                })
            },
        )
        .collect::<Result<Vec<_>, String>>()?;
    StageKey::from_db(&main_stage_key)?;
    let main_stage_key = main_stage_for_project_stages(&stages, schedule_date())?;

    Ok(ProjectDto {
        id,
        code,
        version,
        name,
        file_exists: Path::new(&file_path).exists(),
        file_path,
        release_date,
        main_stage_key,
        archived: archived != 0,
        owner_user_id,
        is_public: is_public != 0,
        last_activity_summary,
        last_activity_actor_name,
        created_at,
        updated_at,
        stages,
    })
}

fn insert_activity(
    connection: &Connection,
    project_id: &str,
    actor_user_id: &str,
    actor_name: &str,
    summary: &str,
    now: &str,
) -> Result<(), String> {
    connection
        .execute(
            "INSERT INTO project_activity_log
             (id, project_id, actor_user_id, actor_name, summary, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                uuid::Uuid::new_v4().to_string(),
                project_id,
                actor_user_id,
                actor_name,
                summary,
                now,
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn map_sql_error(error: rusqlite::Error) -> String {
    if matches!(
        error.sqlite_error_code(),
        Some(rusqlite::ErrorCode::ConstraintViolation)
    ) && error.to_string().contains("projects.code")
    {
        "项目编号已存在".into()
    } else {
        error.to_string()
    }
}

fn timestamp() -> String {
    Utc::now().to_rfc3339()
}

fn schedule_date() -> chrono::NaiveDate {
    Local::now().date_naive()
}
