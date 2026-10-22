//! The French text catalogue (Fluent), the single home of every word the app shows.

use std::collections::BTreeMap;

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};
use fluent_syntax::ast::{Entry, Expression, InlineExpression, Message, PatternElement};
use unic_langid::langid;

const FRENCH: &str = include_str!("../locales/fr.ftl");
/// The texts of the windows, as opposed to those the core composes itself.
const INTERFACE_PREFIX: &str = "ui-";

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
        let text = self
            .bundle
            .format_pattern(pattern, args, &mut errors)
            .into_owned();
        debug_assert!(errors.is_empty(), "`{key}` failed to format: {errors:?}");
        text
    }

    /// Every interface text, rendered, each variable left as `{name}` for the window to fill.
    /// O(c) for a catalogue of c characters: it is read again, once per window.
    ///
    /// # Panics
    ///
    /// Panics when the bundled catalogue is malformed, as `load` does.
    #[must_use]
    pub fn list_interface_texts(&self) -> BTreeMap<String, String> {
        let resource = FluentResource::try_new(FRENCH.to_owned()).expect("a well-formed catalogue");
        resource
            .entries()
            .filter_map(|entry| match entry {
                Entry::Message(message) if message.id.name.starts_with(INTERFACE_PREFIX) => {
                    Some(message)
                }
                _ => None,
            })
            .map(|message| {
                let key = message.id.name;
                (
                    key.to_owned(),
                    self.render(key, Some(&create_open_slots(message))),
                )
            })
            .collect()
    }
}

/// Each variable of the message, valued with its own `{name}`. O(e) for e pattern elements.
fn create_open_slots(message: &Message<&str>) -> FluentArgs<'static> {
    let mut slots = FluentArgs::new();
    let elements = message.value.iter().flat_map(|pattern| &pattern.elements);
    for element in elements {
        if let PatternElement::Placeable {
            expression: Expression::Inline(InlineExpression::VariableReference { id }),
        } = element
        {
            slots.set(id.name.to_owned(), format!("{{{}}}", id.name));
        }
    }
    slots
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
    fn hands_over_every_interface_text_with_its_slots_open() {
        // Arrange
        let catalog = TextCatalog::load();

        // Act
        let texts = catalog.list_interface_texts();

        // Assert
        let keys: Vec<&str> = list_messages()
            .into_iter()
            .map(|(key, _)| key)
            .filter(|key| key.starts_with(INTERFACE_PREFIX))
            .collect();
        assert_eq!(texts.keys().map(String::as_str).collect::<Vec<_>>(), {
            let mut sorted = keys;
            sorted.sort_unstable();
            sorted
        });
        assert_eq!(texts["ui-cancel"], "Annuler");
        assert_eq!(texts["ui-updated-at"], "Mis à jour à {time}");
        assert_eq!(
            texts["ui-throughput-last"],
            "Dernier test, {when}\u{a0}: réception {download}, envoi {upload}"
        );
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
                !has_missing_no_break_space_before_sign(text),
                "`{key}`: no-break space before % and :"
            );
            assert!(!has_decimal_point(text), "`{key}`: decimal comma");
            assert!(
                !has_missing_no_break_space_before_unit(text),
                "`{key}`: no-break space before units"
            );
        }
    }

    fn is_emoji(character: char) -> bool {
        matches!(u32::from(character), 0x1F000..=0x1FAFF | 0x2600..=0x27BF)
    }

    fn has_decimal_point(text: &str) -> bool {
        text.as_bytes().windows(3).any(|triple| {
            triple[0].is_ascii_digit() && triple[1] == b'.' && triple[2].is_ascii_digit()
        })
    }

    fn has_missing_no_break_space_before_sign(text: &str) -> bool {
        text.char_indices().any(|(index, character)| {
            matches!(character, '%' | ':') && !text[..index].ends_with('\u{a0}')
        })
    }

    /// A unit right after a number, glued to it ("60s") or after a plain space ("60 s").
    fn has_missing_no_break_space_before_unit(text: &str) -> bool {
        ["ms", "s", "min", "h", "j", "Mbit/s", "dBm", "GHz"]
            .iter()
            .any(|unit| {
                text.match_indices(unit).any(|(index, _)| {
                    let is_whole_word = text[index + unit.len()..]
                        .chars()
                        .next()
                        .is_none_or(|c| !c.is_alphanumeric());
                    let mut before = text[..index].chars().rev();
                    let follows_a_number = match before.next() {
                        Some(' ') => before.next().is_some_and(is_number_end),
                        Some(c) => is_number_end(c),
                        None => false,
                    };
                    is_whole_word && follows_a_number
                })
            })
    }

    fn is_number_end(character: char) -> bool {
        character.is_ascii_digit() || character == '}'
    }
}
