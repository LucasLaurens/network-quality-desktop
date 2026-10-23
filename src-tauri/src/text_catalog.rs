//! The French text catalogue (Fluent), the single home of every word the app shows.

use std::collections::BTreeMap;

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource, FluentValue};
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

    /// A message without variables. O(length of the message).
    ///
    /// # Panics
    ///
    /// Panics on a key missing from the catalogue, as `render` does.
    #[must_use]
    pub fn render_text(&self, key: &str) -> String {
        self.render(key, None)
    }

    /// A message with one variable, `name`, set to `value`. O(length of the message).
    ///
    /// # Panics
    ///
    /// Panics on a key missing from the catalogue, as `render` does.
    #[must_use]
    pub fn render_value<'a>(
        &self,
        key: &str,
        name: &'a str,
        value: impl Into<FluentValue<'a>>,
    ) -> String {
        let mut args = FluentArgs::new();
        args.set(name, value);
        self.render(key, Some(&args))
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

    /// Literals shaped like catalogue keys that name something else: the tray menu's item ids, the
    /// tray icon's id and a thread's name. No key is built at runtime.
    const NON_CATALOGUE_LITERALS: [&str; 7] = [
        "open-dashboard",
        "open-help",
        "open-settings",
        "run-quick-test",
        "run-full-test",
        "connection-state",
        "color-scheme",
    ];
    /// A literal right after one of these is a key, whatever its shape.
    const CATALOGUE_CALLS: [&str; 3] = ["render(", "render_text(", "render_value("];

    /// Every `.rs` file of the shell, with its name.
    fn list_shell_sources() -> Vec<(String, String)> {
        let directory = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
        std::fs::read_dir(directory)
            .expect("the shell sources")
            .map(|entry| entry.expect("a source entry").path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
            .map(|path| {
                let source = std::fs::read_to_string(&path).expect("a readable source");
                (path.display().to_string(), source)
            })
            .collect()
    }

    /// The string literals of `source` that are catalogue keys: those passed to a render call,
    /// and every literal shaped like one (`word-word`). A literal spans one line, as every key
    /// does. O(c) for c characters.
    fn list_literal_keys(source: &str) -> Vec<&str> {
        let mut keys = Vec::new();
        for line in source.lines() {
            let mut rest = line;
            while let Some(start) = rest.find('"') {
                let after = &rest[start + 1..];
                let Some(end) = after.find('"') else {
                    break;
                };
                let literal = &after[..end];
                let before = rest[..start].trim_end();
                if CATALOGUE_CALLS.iter().any(|call| before.ends_with(call))
                    || is_key_shaped(literal)
                {
                    keys.push(literal);
                }
                rest = &after[end + 1..];
            }
        }
        keys
    }

    fn is_key_shaped(literal: &str) -> bool {
        literal.starts_with(|c: char| c.is_ascii_lowercase())
            && literal.contains('-')
            && !literal.ends_with('-')
            && !literal.contains("--")
            && literal
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    }

    #[test]
    fn finds_every_key_the_shell_names_in_the_catalogue() {
        // Arrange
        let catalog = TextCatalog::load();
        let sources = list_shell_sources();

        // Act
        let missing: Vec<String> = sources
            .iter()
            .flat_map(|(path, source)| {
                list_literal_keys(source)
                    .into_iter()
                    .filter(|key| !NON_CATALOGUE_LITERALS.contains(key))
                    .filter(|key| catalog.bundle.get_message(key).is_none())
                    .map(move |key| format!("{path}: {key}"))
            })
            .collect();

        // Assert
        assert!(sources.len() > 1, "the shell sources were not found");
        assert!(missing.is_empty(), "missing from fr.ftl: {missing:#?}");
    }

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
        let mut keys: Vec<&str> = list_messages()
            .into_iter()
            .map(|(key, _)| key)
            .filter(|key| key.starts_with(INTERFACE_PREFIX))
            .collect();
        keys.sort_unstable();

        // Act
        let texts = catalog.list_interface_texts();

        // Assert
        assert_eq!(texts.keys().map(String::as_str).collect::<Vec<_>>(), keys);
        assert_eq!(texts["ui-cancel"], "Annuler");
        assert_eq!(
            texts["ui-chart-label"], "{metric} sur {period}, avec les seuils Instable et Dégradée",
            "a referenced message is rendered"
        );
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
