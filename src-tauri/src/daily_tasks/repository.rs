use super::model::{
    validate_create, validate_local_date, validate_update, CreateDailyTaskInput, DailyTaskDayDto,
    DailyTaskDto, DailyTaskGroupDto, UpdateDailyTaskInput,
};
use super::report::{format_full_report, format_group_report, ReportGroup, ReportTask};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

pub fn load_day(db: &crate::db::Database, local_date: &str) -> Result<DailyTaskDayDto, String> {
    validate_local_date(local_date)?;
    db.with_connection(|connection| load_day_from_connection(connection, local_date))
}

pub fn report_for_day(
    db: &crate::db::Database,
    local_date: &str,
    group_id: Option<&str>,
) -> Result<super::model::DailyReportResult, String> {
    let day = load_day(db, local_date)?;
    let groups = day
        .groups
        .iter()
        .map(|group| ReportGroup {
            code: group.code.clone(),
            tasks: group
                .tasks
                .iter()
                .map(|task| ReportTask {
                    title: task.title.clone(),
                    progress: task.progress,
                })
                .collect(),
        })
        .collect::<Vec<_>>();
    let text = match group_id {
        Some(id) => {
            let group = day
                .groups
                .iter()
                .find(|group| group.id == id)
                .ok_or_else(|| "DAILY_GROUP_NOT_FOUND".to_string())?;
            format_group_report(&ReportGroup {
                code: group.code.clone(),
                tasks: group
                    .tasks
                    .iter()
                    .map(|task| ReportTask {
                        title: task.title.clone(),
                        progress: task.progress,
                    })
                    .collect(),
            })
        }
        None => format_full_report(&groups),
    };
    Ok(super::model::DailyReportResult {
        task_count: groups.iter().map(|group| group.tasks.len()).sum(),
        text,
    })
}

pub fn create_task(
    db: &crate::db::Database,
    input: CreateDailyTaskInput,
) -> Result<DailyTaskDayDto, String> {
    let code = validate_create(&input)?;
    db.with_immediate_transaction(|transaction| {
        let now = timestamp();
        let day_id = ensure_day(transaction, &input.local_date, &now)?;
        reject_settled(transaction, &day_id)?;
        let group_id = ensure_group(transaction, &day_id, &code, input.project_id.as_deref(), &now)?;
        let position: i64 = transaction
            .query_row(
                "SELECT COALESCE(MAX(position) + 1, 0) FROM daily_tasks WHERE group_id = ?1",
                [&group_id],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "INSERT INTO daily_tasks
                 (id, group_id, title, progress, note, invested_minutes, position, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
                params![
                    uuid::Uuid::new_v4().to_string(),
                    group_id,
                    input.title.trim(),
                    input.progress,
                    input.note,
                    input.invested_minutes,
                    position,
                    now,
                ],
            )
            .map_err(|error| error.to_string())?;
        load_day_from_connection(transaction, &input.local_date)
    })
}

pub fn reorder_groups(
    db: &crate::db::Database,
    local_date: &str,
    codes: Vec<String>,
) -> Result<DailyTaskDayDto, String> {
    validate_local_date(local_date)?;
    db.with_immediate_transaction(|transaction| {
        let day_id: Option<String> = transaction
            .query_row(
                "SELECT id FROM daily_task_days WHERE local_date = ?1",
                [local_date],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        let Some(day_id) = day_id else {
            return if codes.is_empty() {
                load_day_from_connection(transaction, local_date)
            } else {
                Err("DAILY_GROUP_ORDER_INVALID".into())
            };
        };
        reject_settled(transaction, &day_id)?;
        let actual = group_ids(transaction, &day_id)?;
        let requested = codes.iter().collect::<std::collections::HashSet<_>>();
        let expected = actual.iter().collect::<std::collections::HashSet<_>>();
        if codes.len() != actual.len() || requested.len() != codes.len() || requested != expected {
            return Err("DAILY_GROUP_ORDER_INVALID".into());
        }
        transaction
            .execute(
                "UPDATE daily_task_groups SET position = position + 1000000 WHERE day_id = ?1",
                [&day_id],
            )
            .map_err(|error| error.to_string())?;
        for (position, group_id) in codes.iter().enumerate() {
            transaction
                .execute(
                    "UPDATE daily_task_groups SET position = ?1, updated_at = ?2 WHERE id = ?3",
                    params![position as i64, timestamp(), group_id],
                )
                .map_err(|error| error.to_string())?;
        }
        load_day_from_connection(transaction, local_date)
    })
}

pub fn update_task(
    db: &crate::db::Database,
    input: UpdateDailyTaskInput,
) -> Result<DailyTaskDayDto, String> {
    validate_update(&input)?;
    db.with_immediate_transaction(|transaction| {
        let (local_date, day_id, _group_id) = find_task_day(transaction, &input.task_id)?;
        reject_settled(transaction, &day_id)?;
        transaction
            .execute(
                "UPDATE daily_tasks SET title = ?1, progress = ?2, note = ?3, invested_minutes = ?4, updated_at = ?5 WHERE id = ?6",
                params![input.title.trim(), input.progress, input.note, input.invested_minutes, timestamp(), input.task_id],
            )
            .map_err(|error| error.to_string())?;
        load_day_from_connection(transaction, &local_date)
    })
}

pub fn delete_task(
    db: &crate::db::Database,
    local_date: &str,
    task_id: &str,
) -> Result<DailyTaskDayDto, String> {
    validate_local_date(local_date)?;
    uuid::Uuid::parse_str(task_id).map_err(|_| "DAILY_TASK_ID_INVALID")?;
    db.with_immediate_transaction(|transaction| {
        let (task_date, day_id, group_id) = find_task_day(transaction, task_id)?;
        if task_date != local_date {
            return Err("DAILY_TASK_DATE_MISMATCH".into());
        }
        reject_settled(transaction, &day_id)?;
        transaction
            .execute("DELETE FROM daily_tasks WHERE id = ?1", [task_id])
            .map_err(|error| error.to_string())?;
        let remaining: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM daily_tasks WHERE group_id = ?1",
                [&group_id],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        if remaining == 0 {
            transaction
                .execute("DELETE FROM daily_task_groups WHERE id = ?1", [&group_id])
                .map_err(|error| error.to_string())?;
        }
        load_day_from_connection(transaction, local_date)
    })
}

pub fn reorder_tasks(
    db: &crate::db::Database,
    local_date: &str,
    group_id: &str,
    task_ids: Vec<String>,
) -> Result<DailyTaskDayDto, String> {
    validate_local_date(local_date)?;
    db.with_immediate_transaction(|transaction| {
        let day_id: String = transaction
            .query_row(
                "SELECT id FROM daily_task_days WHERE local_date = ?1",
                [local_date],
                |row| row.get(0),
            )
            .map_err(|_| "DAILY_DATE_NOT_FOUND".to_string())?;
        reject_settled(transaction, &day_id)?;
        let belongs: Option<String> = transaction
            .query_row(
                "SELECT id FROM daily_task_groups WHERE id = ?1 AND day_id = ?2",
                params![group_id, day_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        if belongs.is_none() {
            return Err("DAILY_GROUP_NOT_FOUND".into());
        }
        let actual = task_ids_for_group(transaction, group_id)?;
        let requested = task_ids.iter().collect::<std::collections::HashSet<_>>();
        let expected = actual.iter().collect::<std::collections::HashSet<_>>();
        if task_ids.len() != actual.len()
            || requested.len() != task_ids.len()
            || requested != expected
        {
            return Err("DAILY_TASK_ORDER_INVALID".into());
        }
        transaction
            .execute(
                "UPDATE daily_tasks SET position = position + 1000000 WHERE group_id = ?1",
                [group_id],
            )
            .map_err(|error| error.to_string())?;
        for (position, task_id) in task_ids.iter().enumerate() {
            transaction
                .execute(
                    "UPDATE daily_tasks SET position = ?1, updated_at = ?2 WHERE id = ?3",
                    params![position as i64, timestamp(), task_id],
                )
                .map_err(|error| error.to_string())?;
        }
        load_day_from_connection(transaction, local_date)
    })
}

fn find_task_day(
    connection: &Connection,
    task_id: &str,
) -> Result<(String, String, String), String> {
    connection.query_row(
        "SELECT days.local_date, days.id, groups.id FROM daily_tasks tasks JOIN daily_task_groups groups ON groups.id = tasks.group_id JOIN daily_task_days days ON days.id = groups.day_id WHERE tasks.id = ?1",
        [task_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).map_err(|_| "DAILY_TASK_NOT_FOUND".to_string())
}

fn ensure_day(connection: &Connection, local_date: &str, now: &str) -> Result<String, String> {
    if let Some(id) = connection
        .query_row(
            "SELECT id FROM daily_task_days WHERE local_date = ?1",
            [local_date],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
    {
        return Ok(id);
    }
    let id = uuid::Uuid::new_v4().to_string();
    connection
        .execute(
            "INSERT INTO daily_task_days (id, local_date, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
            params![id, local_date, now],
        )
        .map_err(|error| error.to_string())?;
    Ok(id)
}

fn ensure_group(
    connection: &Connection,
    day_id: &str,
    code: &str,
    project_id: Option<&str>,
    now: &str,
) -> Result<String, String> {
    if let Some(id) = connection
        .query_row(
            "SELECT id FROM daily_task_groups WHERE day_id = ?1 AND code = ?2",
            params![day_id, code],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
    {
        return Ok(id);
    }
    let id = uuid::Uuid::new_v4().to_string();
    let position: i64 = connection
        .query_row(
            "SELECT COALESCE(MAX(position) + 1, 0) FROM daily_task_groups WHERE day_id = ?1",
            [day_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "INSERT INTO daily_task_groups (id, day_id, code, project_id, position, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![id, day_id, code, project_id, position, now],
        )
        .map_err(|error| error.to_string())?;
    Ok(id)
}

fn reject_settled(connection: &Connection, day_id: &str) -> Result<(), String> {
    let settled: Option<String> = connection
        .query_row(
            "SELECT settled_at FROM daily_task_days WHERE id = ?1",
            [day_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if settled.is_some() {
        return Err("当天已结算，请先重新打开结算".into());
    }
    Ok(())
}

fn load_day_from_connection(
    connection: &Connection,
    local_date: &str,
) -> Result<DailyTaskDayDto, String> {
    let day: Option<(String, Option<String>, Option<String>)> = connection
        .query_row(
            "SELECT id, settled_at, report_snapshot FROM daily_task_days WHERE local_date = ?1",
            [local_date],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some((id, settled_at, report_snapshot)) = day else {
        return Ok(DailyTaskDayDto {
            id: String::new(),
            local_date: local_date.into(),
            settled_at: None,
            report_snapshot: None,
            groups: vec![],
        });
    };
    let groups = read_groups(connection, &id)?;
    Ok(DailyTaskDayDto {
        id,
        local_date: local_date.into(),
        settled_at,
        report_snapshot,
        groups,
    })
}

fn read_groups(connection: &Connection, day_id: &str) -> Result<Vec<DailyTaskGroupDto>, String> {
    let mut statement = connection
        .prepare("SELECT id, code, project_id, position FROM daily_task_groups WHERE day_id = ?1 ORDER BY position, id")
        .map_err(|error| error.to_string())?;
    let groups = statement
        .query_map([day_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    drop(statement);
    groups
        .into_iter()
        .map(|(id, code, project_id, position)| {
            Ok(DailyTaskGroupDto {
                tasks: read_tasks(connection, &id)?,
                id,
                code,
                project_id,
                position,
            })
        })
        .collect()
}

fn read_tasks(connection: &Connection, group_id: &str) -> Result<Vec<DailyTaskDto>, String> {
    let mut statement = connection.prepare("SELECT id, title, progress, note, invested_minutes, position, source_task_id, source_snapshot_hash, created_at, updated_at FROM daily_tasks WHERE group_id = ?1 ORDER BY position, id").map_err(|error| error.to_string())?;
    let tasks = statement
        .query_map([group_id], |row| {
            Ok(DailyTaskDto {
                id: row.get(0)?,
                title: row.get(1)?,
                progress: row.get(2)?,
                note: row.get(3)?,
                invested_minutes: row.get(4)?,
                position: row.get(5)?,
                source_task_id: row.get(6)?,
                source_snapshot_hash: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(tasks)
}

fn group_ids(connection: &Connection, day_id: &str) -> Result<Vec<String>, String> {
    let mut statement = connection
        .prepare("SELECT id FROM daily_task_groups WHERE day_id = ?1")
        .map_err(|error| error.to_string())?;
    let ids = statement
        .query_map([day_id], |row| row.get(0))
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(ids)
}

fn task_ids_for_group(connection: &Connection, group_id: &str) -> Result<Vec<String>, String> {
    let mut statement = connection
        .prepare("SELECT id FROM daily_tasks WHERE group_id = ?1")
        .map_err(|error| error.to_string())?;
    let ids = statement
        .query_map([group_id], |row| row.get(0))
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(ids)
}

fn timestamp() -> String {
    Utc::now().to_rfc3339()
}
