use crate::api::IssueSummary;

pub fn fit(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }

    if width == 0 {
        return String::new();
    }

    let mut output: String = text.chars().take(width - 1).collect();
    output.push('…');

    output
}

pub fn id_column_width(issues: &[IssueSummary]) -> usize {
    issues
        .iter()
        .map(|issue| issue.identifier.chars().count())
        .max()
        .unwrap_or(0)
}
