pub struct PaletteEmoji {
    pub name: &'static str,
    pub glyph: &'static str,
}

pub const REACTION_PALETTE: &[PaletteEmoji] = &[
    PaletteEmoji {
        name: "+1",
        glyph: "👍",
    },
    PaletteEmoji {
        name: "heart",
        glyph: "❤️",
    },
    PaletteEmoji {
        name: "tada",
        glyph: "🎉",
    },
    PaletteEmoji {
        name: "smile",
        glyph: "😄",
    },
    PaletteEmoji {
        name: "eyes",
        glyph: "👀",
    },
    PaletteEmoji {
        name: "rocket",
        glyph: "🚀",
    },
    PaletteEmoji {
        name: "confused",
        glyph: "😕",
    },
    PaletteEmoji {
        name: "-1",
        glyph: "👎",
    },
];

pub fn glyph(name: &str) -> String {
    emojis::get_by_shortcode(name)
        .map(|emoji| emoji.as_str().to_string())
        .unwrap_or_else(|| format!(":{name}:"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_gemoji_shortcodes() {
        assert_eq!(glyph("eggplant"), "🍆");
        assert_eq!(glyph("+1"), "👍");
        assert_eq!(glyph("-1"), "👎");
        assert_eq!(glyph("fire"), "🔥");
    }

    #[test]
    fn unknown_shortcodes_fall_back_to_the_colon_form() {
        assert_eq!(
            glyph("definitely_not_an_emoji"),
            ":definitely_not_an_emoji:"
        );
    }

    #[test]
    fn palette_glyphs_match_their_shortcodes() {
        for entry in REACTION_PALETTE {
            assert_eq!(
                glyph(entry.name),
                entry.glyph,
                "palette entry {}",
                entry.name
            );
        }
    }
}
