use super::{
    model::{
        main_stage_for_schedule, progress_for_schedule, CreateProjectInput, SaveProjectStageInput,
        SetProjectStageInput, StageKey, STAGE_KEYS,
    },
    repository,
};
use chrono::NaiveDate;

#[test]
fn schedule_dates_derive_stage_progress_and_current_main_stage() {
    assert_eq!(
        progress_for_schedule(
            "2026-07-01",
            "2026-07-10",
            NaiveDate::from_ymd_opt(2026, 6, 30).unwrap(),
        )
        .unwrap(),
        0,
    );
    assert_eq!(
        progress_for_schedule(
            "2026-07-01",
            "2026-07-10",
            NaiveDate::from_ymd_opt(2026, 7, 5).unwrap(),
        )
        .unwrap(),
        44,
    );
    assert_eq!(
        progress_for_schedule(
            "2026-07-01",
            "2026-07-10",
            NaiveDate::from_ymd_opt(2026, 7, 10).unwrap(),
        )
        .unwrap(),
        100,
    );

    let stages = vec![
        SaveProjectStageInput {
            stage_key: StageKey::Storyboard,
            start_date: "2026-07-01".to_string(),
            end_date: "2026-07-10".to_string(),
            progress: 0,
        },
        SaveProjectStageInput {
            stage_key: StageKey::FirstCut,
            start_date: "2026-07-05".to_string(),
            end_date: "2026-07-20".to_string(),
            progress: 0,
        },
    ];
    assert_eq!(
        main_stage_for_schedule(&stages, NaiveDate::from_ymd_opt(2026, 7, 8).unwrap()).unwrap(),
        StageKey::FirstCut,
    );
}

#[test]
fn creating_project_seeds_fixed_eight_stages_in_order() {
    let db = test_database();
    let project = repository::create_project(&db, create_input("L36")).unwrap();

    assert_eq!(project.stages.len(), 8);
    assert_eq!(project.stages[0].stage_key, StageKey::Storyboard);
    assert_eq!(project.stages[3].stage_key, StageKey::MiddleCut);
    assert_eq!(project.stages[7].stage_key, StageKey::FinalComposite);
}

#[test]
fn project_codes_may_repeat_ignoring_ascii_case() {
    let db = test_database();
    let first = repository::create_project(&db, create_input("L36")).unwrap();
    let second = repository::create_project(&db, create_input("l36")).unwrap();

    assert_ne!(first.id, second.id);
    assert_eq!(second.code, "l36");
}

#[test]
fn stages_may_overlap_but_each_stage_must_have_a_valid_range() {
    let db = test_database();
    let project = repository::create_project(&db, create_input("L36")).unwrap();

    set_range(
        &db,
        &project.id,
        StageKey::Storyboard,
        "2026-07-01",
        "2026-07-10",
        80,
    )
    .unwrap();
    set_range(
        &db,
        &project.id,
        StageKey::FirstCut,
        "2026-07-05",
        "2026-07-14",
        30,
    )
    .unwrap();

    let error = set_range(
        &db,
        &project.id,
        StageKey::Refinement,
        "2026-07-20",
        "2026-07-19",
        0,
    )
    .unwrap_err();

    assert_eq!(error, "阶段开始日期不能晚于结束日期");
}

fn test_database() -> crate::db::Database {
    let dir = tempfile::tempdir().unwrap();
    crate::db::Database::open(dir.keep().join("banana.db")).unwrap()
}

fn create_input(code: &str) -> CreateProjectInput {
    CreateProjectInput {
        code: code.to_string(),
        version: "v1".to_string(),
        name: "Project".to_string(),
        file_path: r"C:\work\L36".to_string(),
        release_date: "2026-07-31".to_string(),
        stages: STAGE_KEYS
            .iter()
            .enumerate()
            .map(|(position, &stage_key)| SaveProjectStageInput {
                stage_key,
                start_date: format!("2026-07-{:02}", position + 1),
                end_date: format!("2026-07-{:02}", position + 8),
                progress: 0,
            })
            .collect(),
    }
}

fn set_range(
    db: &crate::db::Database,
    project_id: &str,
    stage_key: StageKey,
    start_date: &str,
    end_date: &str,
    progress: i64,
) -> Result<super::model::ProjectDto, String> {
    repository::set_project_stage(
        db,
        SetProjectStageInput {
            project_id: project_id.to_string(),
            stage_key,
            start_date: start_date.to_string(),
            end_date: end_date.to_string(),
            progress,
        },
    )
}
