use super::report::{format_full_report, format_group_report, ReportGroup, ReportTask};

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
