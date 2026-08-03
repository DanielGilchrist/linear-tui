use ratatui::text::{Line, Span};

use super::super::theme;
use crate::api::Reaction;
use crate::tui::emoji;

pub fn reaction_chips(reactions: &[Reaction]) -> Option<Line<'static>> {
    if reactions.is_empty() {
        return None;
    }

    let mut groups: Vec<(String, usize, bool)> = Vec::new();

    for reaction in reactions {
        match groups
            .iter_mut()
            .find(|(emoji, _, _)| emoji == &reaction.emoji)
        {
            Some((_, count, mine)) => {
                *count += 1;
                *mine |= reaction.mine;
            }
            None => groups.push((reaction.emoji.clone(), 1, reaction.mine)),
        }
    }

    let mut spans: Vec<Span<'static>> = Vec::new();

    for (name, count, mine) in groups {
        let style = if mine {
            theme::reaction_mine()
        } else {
            theme::REACTION
        };

        spans.push(Span::styled(
            format!("{} {count}", emoji::glyph(&name)),
            style,
        ));

        spans.push(Span::raw("  "));
    }

    spans.pop();

    Some(Line::from(spans))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ReactionId;

    fn reaction(emoji: &str, mine: bool) -> Reaction {
        Reaction {
            id: ReactionId::from_raw("r"),
            emoji: emoji.into(),
            mine,
        }
    }

    #[test]
    fn no_reactions_render_nothing() {
        assert!(reaction_chips(&[]).is_none());
    }

    #[test]
    fn groups_by_emoji_name_with_counts_and_mine_highlight() {
        let reactions = vec![
            reaction("+1", true),
            reaction("+1", false),
            reaction("heart", false),
        ];

        let line = reaction_chips(&reactions).expect("chips");
        let chips: Vec<(&str, ratatui::style::Style)> = line
            .spans
            .iter()
            .filter(|span| span.content.trim() != "")
            .map(|span| (span.content.as_ref(), span.style))
            .collect();

        assert_eq!(chips.len(), 2);
        assert_eq!(chips[0].0, "👍 2");
        assert_eq!(chips[0].1, theme::reaction_mine());
        assert_eq!(chips[1].0, "❤️ 1");
        assert_eq!(chips[1].1, theme::REACTION);
    }
}
