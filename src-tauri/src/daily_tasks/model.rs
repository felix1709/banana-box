use chrono::NaiveDate;

pub const MAX_GROUP_CODE_BYTES: usize = 32;
pub const MAX_TASK_TITLE_SCALARS: usize = 200;
pub const MAX_TASK_TITLE_BYTES: usize = 800;
pub const MAX_TASK_NOTE_BYTES: usize = 64 * 1024;
pub const MAX_REMINDER_CONTENT_BYTES: usize = 800;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyTaskDayDto {
    pub id: String,
    pub local_date: String,
    pub settled_at: Option<String>,
    pub report_snapshot: Option<String>,
    pub groups: Vec<DailyTaskGroupDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyReportResult {
    pub text: String,
    pub task_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyTaskGroupDto {
    pub id: String,
    pub code: String,
    pub project_id: Option<String>,
    pub position: i64,
    pub tasks: Vec<DailyTaskDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyTaskDto {
    pub id: String,
    pub title: String,
    pub progress: i64,
    pub note: String,
    pub invested_minutes: i64,
    pub reminder_time: String,
    pub reminder_content: String,
    pub position: i64,
    pub source_task_id: Option<String>,
    pub source_snapshot_hash: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateDailyTaskInput {
    pub local_date: String,
    pub code: String,
    pub project_id: Option<String>,
    pub title: String,
    pub progress: i64,
    pub note: String,
    pub invested_minutes: i64,
    #[serde(default)]
    pub reminder_time: String,
    #[serde(default)]
    pub reminder_content: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateDailyTaskInput {
    pub task_id: String,
    pub title: String,
    pub progress: i64,
    pub note: String,
    pub invested_minutes: i64,
    pub reminder_time: String,
    pub reminder_content: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReorderDailyGroupsInput {
    pub local_date: String,
    pub group_ids: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReorderDailyTasksInput {
    pub local_date: String,
    pub group_id: String,
    pub task_ids: Vec<String>,
}

pub fn validate_local_date(value: &str) -> Result<(), String> {
    if value.len() != 10 || NaiveDate::parse_from_str(value, "%Y-%m-%d").is_err() {
        return Err("DAILY_DATE_INVALID".into());
    }
    Ok(())
}

pub fn normalize_group_code(value: &str) -> Result<String, String> {
    let code = value.trim().to_ascii_uppercase();
    let valid = !code.is_empty()
        && code.len() <= MAX_GROUP_CODE_BYTES
        && code.as_bytes()[0].is_ascii_alphanumeric()
        && code.bytes().all(|byte| {
            byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
        });
    valid
        .then_some(code)
        .ok_or_else(|| "DAILY_GROUP_CODE_INVALID".into())
}

pub fn validate_task_fields(
    title: &str,
    progress: i64,
    note: &str,
    invested_minutes: i64,
    reminder_time: &str,
    reminder_content: &str,
) -> Result<(), String> {
    let title = title.trim();
    if title.is_empty()
        || title.chars().count() > MAX_TASK_TITLE_SCALARS
        || title.len() > MAX_TASK_TITLE_BYTES
        || title
            .chars()
            .any(|character| character.is_control() || character == '【' || character == '】')
    {
        return Err("DAILY_TASK_TITLE_INVALID".into());
    }
    if !(0..=100).contains(&progress) {
        return Err("DAILY_TASK_PROGRESS_INVALID".into());
    }
    if invested_minutes < 0 {
        return Err("DAILY_TASK_MINUTES_INVALID".into());
    }
    if note.len() > MAX_TASK_NOTE_BYTES
        || note.chars().any(|character| {
            character == '\0'
                || (character.is_control()
                    && character != '\n'
                    && character != '\r'
                    && character != '\t')
        })
    {
        return Err("DAILY_TASK_NOTE_INVALID".into());
    }
    if !reminder_time.is_empty() {
        let bytes = reminder_time.as_bytes();
        let valid_time = bytes.len() == 5
            && bytes[2] == b':'
            && bytes[0].is_ascii_digit()
            && bytes[1].is_ascii_digit()
            && bytes[3].is_ascii_digit()
            && bytes[4].is_ascii_digit()
            && ((bytes[0] - b'0') * 10 + (bytes[1] - b'0')) <= 23
            && ((bytes[3] - b'0') * 10 + (bytes[4] - b'0')) <= 59;
        if !valid_time {
            return Err("DAILY_TASK_REMINDER_TIME_INVALID".into());
        }
    }
    if reminder_content.len() > MAX_REMINDER_CONTENT_BYTES
        || reminder_content.chars().any(|character| {
            character == '\0'
                || (character.is_control()
                    && character != '\n'
                    && character != '\r'
                    && character != '\t')
        })
    {
        return Err("DAILY_TASK_REMINDER_CONTENT_INVALID".into());
    }
    Ok(())
}

pub fn validate_create(input: &CreateDailyTaskInput) -> Result<String, String> {
    validate_local_date(&input.local_date)?;
    let code = normalize_group_code(&input.code)?;
    validate_task_fields(
        &input.title,
        input.progress,
        &input.note,
        input.invested_minutes,
        &input.reminder_time,
        &input.reminder_content,
    )?;
    Ok(code)
}

pub fn validate_update(input: &UpdateDailyTaskInput) -> Result<(), String> {
    uuid::Uuid::parse_str(&input.task_id).map_err(|_| "DAILY_TASK_ID_INVALID")?;
    validate_task_fields(
        &input.title,
        input.progress,
        &input.note,
        input.invested_minutes,
        &input.reminder_time,
        &input.reminder_content,
    )
}
