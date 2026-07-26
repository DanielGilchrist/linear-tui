use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::api::IssueSummary;

pub fn width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

pub fn fit(text: &str, max_width: usize) -> String {
    if width(text) <= max_width {
        return text.to_string();
    }

    if max_width == 0 {
        return String::new();
    }

    let budget = max_width - 1;
    let mut output = String::new();
    let mut used = 0;

    for ch in text.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);

        if used + ch_width > budget {
            break;
        }

        output.push(ch);
        used += ch_width;
    }

    output.push('…');

    output
}

pub fn id_column_width(issues: &[IssueSummary]) -> usize {
    issues
        .iter()
        .map(|issue| width(&issue.identifier))
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_measures_display_width_not_char_count() {
        assert_eq!(width("日本"), 4);
        assert_eq!(fit("日本語", 10), "日本語");

        let fitted = fit("日本語のタイトル", 5);
        assert!(
            width(&fitted) <= 5,
            "fitted width {} exceeds 5",
            width(&fitted)
        );
        assert!(fitted.ends_with('…'));
    }

    #[test]
    fn fit_leaves_short_ascii_untouched() {
        assert_eq!(fit("DAN2-7", 10), "DAN2-7");
        assert_eq!(fit("a very long title", 8), "a very …");
    }
}
