use super::model::{normalize_group_code, validate_task_fields};
use super::report::{format_full_report, format_group_report, ReportGroup, ReportTask};
use super::{
    model::{CreateDailyTaskInput, UpdateDailyTaskInput},
    repository,
};

fn group(code: &str, tasks: &[(&str, i64)]) -> ReportGroup {
    ReportGroup {
        code: code.into(),
        tasks: tasks
            .iter()
            .map(|(title, progress)| ReportTask {
                title: (*title).into(),
                progress: *progress,
            })
            .collect(),
    }
}

#[test]
fn full_report_matches_the_confirmed_string_exactly() {
    let groups = [
        group("L36", &[("三丽鸥跟进", 100), ("412漫画发型跟进", 100)]),
        group("L50", &[("混厄录像带切片制作", 50)]),
    ];

    assert_eq!(
        format_full_report(&groups),
        "@日报\n#L36\n1.【L36】【三丽鸥跟进】【100%】\n2.【L36】【412漫画发型跟进】【100%】\n#L50\n1.【L50】【混厄录像带切片制作】【50%】"
    );
}

#[test]
fn group_report_omits_daily_header_and_keeps_incomplete_tasks() {
    let group = group("L36", &[("未开始", 0), ("制作中", 35), ("完成", 100)]);

    assert_eq!(
        format_group_report(&group),
        "#L36\n1.【L36】【未开始】【0%】\n2.【L36】【制作中】【35%】\n3.【L36】【完成】【100%】"
    );
}

#[test]
fn task_input_keeps_chinese_titles_but_rejects_report_structure_injection() {
    assert_eq!(normalize_group_code(" l36 ").unwrap(), "L36");
    assert!(
        validate_task_fields("三丽鸥跟进 #角色", 50, "工作记录", 95, "09:30", "提醒")
            .is_ok()
    );
    assert!(normalize_group_code("#L36").is_err());
    assert!(validate_task_fields("第一行\n第二行", 50, "", 0, "", "").is_err());
    assert!(validate_task_fields("【伪造】", 50, "", 0, "", "").is_err());
    assert!(validate_task_fields("正常任务", 50, "", 0, "24:00", "").is_err());
}

#[test]
fn daily_groups_keep_explicit_order_and_tasks_keep_work_metadata() {
    let db = test_database();
    repository::create_task(&db, input("2026-07-11", "L50", "录像带", 50, 95)).unwrap();
    repository::create_task(&db, input("2026-07-11", "L36", "三丽鸥", 100, 40)).unwrap();
    let unordered = repository::load_day(&db, "2026-07-11").unwrap();
    repository::reorder_groups(
        &db,
        "2026-07-11",
        vec![
            unordered.groups[1].id.clone(),
            unordered.groups[0].id.clone(),
        ],
    )
    .unwrap();

    let day = repository::load_day(&db, "2026-07-11").unwrap();
    assert_eq!(day.groups[0].code, "L36");
    assert_eq!(day.groups[1].tasks[0].invested_minutes, 95);
    assert!(!day.groups[1].tasks[0].updated_at.is_empty());
}

#[test]
fn loading_a_history_date_never_merges_tasks_from_another_day() {
    let db = test_database();
    repository::create_task(&db, input("2026-07-10", "L36", "昨天", 100, 20)).unwrap();
    repository::create_task(&db, input("2026-07-11", "L36", "今天", 0, 0)).unwrap();

    let history = repository::load_day(&db, "2026-07-10").unwrap();
    assert_eq!(history.groups[0].tasks[0].title, "昨天");
}

#[test]
fn tasks_can_update_reorder_and_delete_without_leaving_stale_rows() {
    let db = test_database();
    let initial = repository::create_task(&db, input("2026-07-11", "L36", "任务一", 0, 0)).unwrap();
    let first = initial.groups[0].tasks[0].id.clone();
    let second_day =
        repository::create_task(&db, input("2026-07-11", "L36", "任务二", 50, 10)).unwrap();
    let group = &second_day.groups[0];
    let second = group.tasks[1].id.clone();

    let changed = repository::update_task(
        &db,
        UpdateDailyTaskInput {
            task_id: first.clone(),
            title: "任务一已更新".into(),
            progress: 80,
            note: "备注".into(),
            invested_minutes: 35,
            reminder_time: "10:30".into(),
            reminder_content: "检查进度".into(),
        },
    )
    .unwrap();
    assert_eq!(changed.groups[0].tasks[0].progress, 80);
    assert_eq!(changed.groups[0].tasks[0].reminder_time, "10:30");

    let reordered = repository::reorder_tasks(
        &db,
        "2026-07-11",
        &group.id,
        vec![second.clone(), first.clone()],
    )
    .unwrap();
    assert_eq!(reordered.groups[0].tasks[0].id, second);

    let deleted = repository::delete_task(&db, "2026-07-11", &first).unwrap();
    assert_eq!(deleted.groups[0].tasks.len(), 1);
    assert_eq!(deleted.groups[0].tasks[0].id, second);
}

fn test_database() -> crate::db::Database {
    let dir = tempfile::tempdir().unwrap();
    crate::db::Database::open(dir.keep().join("banana.db")).unwrap()
}

fn input(
    local_date: &str,
    code: &str,
    title: &str,
    progress: i64,
    invested_minutes: i64,
) -> CreateDailyTaskInput {
    CreateDailyTaskInput {
        local_date: local_date.into(),
        code: code.into(),
        project_id: None,
        title: title.into(),
        progress,
        note: String::new(),
        invested_minutes,
        reminder_time: String::new(),
        reminder_content: String::new(),
    }
}
