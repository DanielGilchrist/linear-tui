pub fn parse_issue_ref(input: &str) -> String {
    let trimmed = input.trim();
    let candidate = extract_from_url(trimmed).unwrap_or_else(|| trimmed.to_string());

    if looks_like_identifier(&candidate) {
        candidate.to_uppercase()
    } else {
        candidate
    }
}

fn extract_from_url(input: &str) -> Option<String> {
    let id = input.split("/issue/").nth(1)?.split('/').next()?;
    (!id.is_empty()).then(|| id.to_string())
}

fn looks_like_identifier(candidate: &str) -> bool {
    match candidate.split_once('-') {
        Some((team, number)) => {
            !team.is_empty()
                && team.chars().all(|c| c.is_ascii_alphanumeric())
                && !number.is_empty()
                && number.chars().all(|c| c.is_ascii_digit())
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_issue_ref;

    #[test]
    fn parses_a_bare_identifier() {
        assert_eq!(parse_issue_ref("DAN2-7"), "DAN2-7");
    }

    #[test]
    fn uppercases_a_lowercase_identifier() {
        assert_eq!(parse_issue_ref("dan-14"), "DAN-14");
        assert_eq!(parse_issue_ref("  dan2-7  "), "DAN2-7");
    }

    #[test]
    fn extracts_the_identifier_from_a_url() {
        assert_eq!(
            parse_issue_ref("https://linear.app/dans-donuts/issue/DAN2-7/wood-fired-oven"),
            "DAN2-7"
        );
    }

    #[test]
    fn keeps_non_identifier_text_as_is() {
        assert_eq!(parse_issue_ref("not an id"), "not an id");
        assert_eq!(parse_issue_ref("dan-abc"), "dan-abc");
    }
}
