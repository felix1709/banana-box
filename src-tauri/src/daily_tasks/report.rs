#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportTask {
    pub title: String,
    pub progress: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportGroup {
    pub code: String,
    pub tasks: Vec<ReportTask>,
}

pub fn format_group_report(group: &ReportGroup) -> String {
    let mut lines = Vec::with_capacity(group.tasks.len() + 1);
    lines.push(format!("#{}", group.code));
    lines.extend(group.tasks.iter().enumerate().map(|(index, task)| {
        format!(
            "{}.【{}】【{}】【{}%】",
            index + 1,
            group.code,
            task.title,
            task.progress
        )
    }));
    lines.join("\n")
}

pub fn format_full_report(groups: &[ReportGroup]) -> String {
    let mut sections = Vec::with_capacity(groups.len() + 1);
    sections.push("@日报".to_string());
    sections.extend(groups.iter().map(format_group_report));
    sections.join("\n")
}
