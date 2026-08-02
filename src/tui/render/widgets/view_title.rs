pub fn view_title(name: &str, mode: Option<&str>, count: Option<usize>, truncated: bool) -> String {
    let name = match mode {
        Some(mode) => format!("{name}  ·  {mode}"),
        None => name.to_string(),
    };

    match count {
        Some(count) => {
            let more = if truncated { "+" } else { "" };
            format!(" {name}  ·  {count}{more} issues ")
        }
        None => format!(" {name} "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_loaded_count_reads_n_issues() {
        assert_eq!(
            view_title("Sprint", None, Some(3), false),
            " Sprint  ·  3 issues "
        );
    }

    #[test]
    fn a_truncated_feed_marks_the_count_with_a_plus() {
        assert_eq!(
            view_title("Sprint", None, Some(3), true),
            " Sprint  ·  3+ issues "
        );
    }

    #[test]
    fn an_empty_or_absent_feed_is_just_the_name() {
        assert_eq!(view_title("Sprint", None, None, false), " Sprint ");
        assert_eq!(view_title("Sprint", None, None, true), " Sprint ");
    }
}
