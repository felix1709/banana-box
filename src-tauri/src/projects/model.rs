use chrono::NaiveDate;
use std::collections::HashSet;

pub const MAX_PROJECT_CODE_BYTES: usize = 32;
pub const MAX_PROJECT_VERSION_BYTES: usize = 64;
pub const MAX_PROJECT_NAME_BYTES: usize = 800;
pub const MAX_PROJECT_FILE_PATH_BYTES: usize = 32 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageKey {
    Storyboard,
    FirstCut,
    Refinement,
    MiddleCut,
    Effects,
    ArtTitles,
    Music,
    FinalComposite,
}

pub const STAGE_KEYS: [StageKey; 8] = [
    StageKey::Storyboard,
    StageKey::FirstCut,
    StageKey::Refinement,
    StageKey::MiddleCut,
    StageKey::Effects,
    StageKey::ArtTitles,
    StageKey::Music,
    StageKey::FinalComposite,
];

impl StageKey {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Storyboard => "storyboard",
            Self::FirstCut => "first_cut",
            Self::Refinement => "refinement",
            Self::MiddleCut => "middle_cut",
            Self::Effects => "effects",
            Self::ArtTitles => "art_titles",
            Self::Music => "music",
            Self::FinalComposite => "final_composite",
        }
    }

    pub fn from_db(value: &str) -> Result<Self, String> {
        match value {
            "storyboard" => Ok(Self::Storyboard),
            "first_cut" => Ok(Self::FirstCut),
            "refinement" => Ok(Self::Refinement),
            "middle_cut" => Ok(Self::MiddleCut),
            "effects" => Ok(Self::Effects),
            "art_titles" => Ok(Self::ArtTitles),
            "music" => Ok(Self::Music),
            "final_composite" => Ok(Self::FinalComposite),
            _ => Err("PROJECT_STAGE_INVALID".into()),
        }
    }

    pub fn position(self) -> usize {
        STAGE_KEYS
            .iter()
            .position(|stage_key| *stage_key == self)
            .expect("stage key is part of the fixed definition")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStageDto {
    pub id: String,
    pub stage_key: StageKey,
    pub position: i64,
    pub start_date: String,
    pub end_date: String,
    pub progress: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDto {
    pub id: String,
    pub code: String,
    pub version: String,
    pub name: String,
    pub file_path: String,
    pub file_exists: bool,
    pub release_date: String,
    pub main_stage_key: StageKey,
    pub archived: bool,
    pub created_at: String,
    pub updated_at: String,
    pub stages: Vec<ProjectStageDto>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateProjectInput {
    pub code: String,
    pub version: String,
    pub name: String,
    pub file_path: String,
    pub release_date: String,
    pub main_stage_key: StageKey,
    pub stages: Vec<SaveProjectStageInput>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateProjectInput {
    pub project_id: String,
    pub code: String,
    pub version: String,
    pub name: String,
    pub file_path: String,
    pub release_date: String,
    pub main_stage_key: StageKey,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetProjectStageInput {
    pub project_id: String,
    pub stage_key: StageKey,
    pub start_date: String,
    pub end_date: String,
    pub progress: i64,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveProjectStageInput {
    pub stage_key: StageKey,
    pub start_date: String,
    pub end_date: String,
    pub progress: i64,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveProjectWithStagesInput {
    pub project_id: String,
    pub code: String,
    pub version: String,
    pub name: String,
    pub file_path: String,
    pub release_date: String,
    pub main_stage_key: StageKey,
    pub archived: bool,
    pub stages: Vec<SaveProjectStageInput>,
}

pub fn validate_project_id(project_id: &str) -> Result<(), String> {
    uuid::Uuid::parse_str(project_id)
        .map(|_| ())
        .map_err(|_| "PROJECT_ID_INVALID".into())
}

pub fn validate_project_fields(
    code: &str,
    version: &str,
    name: &str,
    file_path: &str,
    release_date: &str,
) -> Result<(), String> {
    validate_code(code)?;
    validate_required_text(
        version,
        MAX_PROJECT_VERSION_BYTES,
        "PROJECT_VERSION_INVALID",
    )?;
    validate_required_text(name, MAX_PROJECT_NAME_BYTES, "PROJECT_NAME_INVALID")?;
    validate_required_text(
        file_path,
        MAX_PROJECT_FILE_PATH_BYTES,
        "PROJECT_FILE_PATH_INVALID",
    )?;
    validate_date(release_date, "PROJECT_RELEASE_DATE_INVALID")
}

pub fn validate_stage(stage: &SaveProjectStageInput) -> Result<(), String> {
    validate_stage_values(&stage.start_date, &stage.end_date, stage.progress)
}

pub fn validate_stage_values(
    start_date: &str,
    end_date: &str,
    progress: i64,
) -> Result<(), String> {
    let start = parse_date(start_date, "STAGE_START_DATE_INVALID")?;
    let end = parse_date(end_date, "STAGE_END_DATE_INVALID")?;
    if start > end {
        return Err("阶段开始日期不能晚于结束日期".into());
    }
    if !(0..=100).contains(&progress) {
        return Err("STAGE_PROGRESS_INVALID".into());
    }
    Ok(())
}

pub fn validate_and_sort_stages(
    stages: &[SaveProjectStageInput],
) -> Result<Vec<SaveProjectStageInput>, String> {
    if stages.len() != STAGE_KEYS.len() {
        return Err("PROJECT_STAGES_INCOMPLETE".into());
    }

    let keys: HashSet<_> = stages.iter().map(|stage| stage.stage_key).collect();
    if keys.len() != STAGE_KEYS.len()
        || !STAGE_KEYS.iter().all(|stage_key| keys.contains(stage_key))
    {
        return Err("PROJECT_STAGES_INVALID".into());
    }

    let mut stages = stages.to_vec();
    for stage in &stages {
        validate_stage(stage)?;
    }
    stages.sort_by_key(|stage| stage.stage_key.position());
    Ok(stages)
}

fn validate_code(code: &str) -> Result<(), String> {
    let code = code.trim();
    if code.is_empty()
        || code.len() > MAX_PROJECT_CODE_BYTES
        || !code.is_ascii()
        || contains_control(code)
    {
        return Err("PROJECT_CODE_INVALID".into());
    }
    Ok(())
}

fn validate_required_text(
    value: &str,
    max_bytes: usize,
    error: &'static str,
) -> Result<(), String> {
    let value = value.trim();
    if value.is_empty() || value.len() > max_bytes || contains_control(value) {
        return Err(error.into());
    }
    Ok(())
}

fn contains_control(value: &str) -> bool {
    value
        .chars()
        .any(|character| character == '\0' || character.is_control())
}

fn validate_date(value: &str, error: &'static str) -> Result<(), String> {
    parse_date(value, error).map(|_| ())
}

fn parse_date(value: &str, error: &'static str) -> Result<NaiveDate, String> {
    if value.len() != 10 {
        return Err(error.into());
    }
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| error.into())
}
