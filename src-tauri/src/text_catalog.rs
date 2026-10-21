//! The French text catalogue (Fluent), the single home of every word the app shows.

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};
use unic_langid::langid;

const FRENCH: &str = include_str!("../locales/fr.ftl");

pub struct TextCatalog {
    bundle: FluentBundle<FluentResource>,
}

impl TextCatalog {
    /// # Panics
    ///
    /// Panics when the bundled catalogue is malformed, which a test rules out before any release.
    #[must_use]
    pub fn load() -> Self {
        let resource = FluentResource::try_new(FRENCH.to_owned()).expect("a well-formed catalogue");
        let mut bundle = FluentBundle::new_concurrent(vec![langid!("fr")]);
        // Isolation marks around placeables would end up in the window as invisible characters.
        bundle.set_use_isolating(false);
        bundle
            .add_resource(resource)
            .expect("no duplicate key in the catalogue");
        Self { bundle }
    }

    /// O(length of the message).
    ///
    /// # Panics
    ///
    /// Panics on a key missing from the catalogue: a programming error the tests catch.
    #[must_use]
    pub fn render(&self, key: &str, args: Option<&FluentArgs>) -> String {
        let pattern = self
            .bundle
            .get_message(key)
            .and_then(|message| message.value())
            .unwrap_or_else(|| panic!("`{key}` is missing from the catalogue"));
        let mut errors = Vec::new();
        self.bundle
            .format_pattern(pattern, args, &mut errors)
            .into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list_messages() -> Vec<(&'static str, &'static str)> {
        FRENCH
            .lines()
            .filter(|line| !line.starts_with('#') && line.contains(" = "))
            .filter_map(|line| line.split_once(" = "))
            .collect()
    }

    #[test]
    fn loads_without_any_malformed_entry() {
        // Act
        let resource = FluentResource::try_new(FRENCH.to_owned());

        // Assert
        assert!(resource.is_ok(), "{:?}", resource.err());
    }

    #[test]
    fn keeps_causes_and_actions_short() {
        // Act & Assert
        for (key, text) in list_messages() {
            let limit = if key.starts_with("cause-") {
                80
            } else if key.starts_with("action-") {
                60
            } else {
                continue;
            };
            assert!(
                text.chars().count() <= limit,
                "`{key}` is longer than {limit} characters"
            );
        }
    }

    #[test]
    fn follows_the_french_typography_rules() {
        // Act & Assert
        for (key, text) in list_messages() {
            assert!(!text.contains('!'), "`{key}`: no exclamation mark");
            assert!(!text.contains('\u{2014}'), "`{key}`: no em dash");
            assert!(!text.chars().any(is_emoji), "`{key}`: no emoji");
            assert!(
                !text.contains(" %") && !text.contains(" :"),
                "`{key}`: no-break space before % and :"
            );
            assert!(!has_decimal_point(text), "`{key}`: decimal comma");
            assert!(
                !has_plain_space_before_unit(text),
                "`{key}`: no-break space before units"
            );
        }
    }

    fn is_emoji(character: char) -> bool {
        matches!(u32::from(character), 0x1F000..=0x1FAFF | 0x2600..=0x27BF)
    }

    fn has_decimal_point(text: &str) -> bool {
        text.as_bytes()
            .windows(3)
            .any(|w| w[0].is_ascii_digit() && w[1] == b'.' && w[2].is_ascii_digit())
    }

    fn has_plain_space_before_unit(text: &str) -> bool {
        ["ms", "s", "Mbit/s", "dBm", "GHz"].iter().any(|unit| {
            text.match_indices(&format!(" {unit}")).any(|(index, _)| {
                let before = text[..index].chars().last();
                let after = text[index + 1 + unit.len()..].chars().next();
                before.is_some_and(|c| c.is_ascii_digit() || c == '}')
                    && after.is_none_or(|c| !c.is_alphanumeric())
            })
        })
    }
}
