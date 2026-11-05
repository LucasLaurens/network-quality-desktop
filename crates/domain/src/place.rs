//! What the user calls the place a network serves, to tell where the connection holds.

/// "Maison", "Bureau", "Café de la gare": trimmed, never longer than a dashboard row can show.
/// Empty while the user has not named it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlaceName(String);

/// The name runs past `PlaceName::MAX_CHARS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaceNameTooLong;

impl PlaceName {
    pub const MAX_CHARS: usize = 60;

    /// Blank text forgets the name. O(n) for n characters.
    ///
    /// # Errors
    ///
    /// More than `MAX_CHARS` characters once trimmed.
    pub fn parse(text: &str) -> Result<Self, PlaceNameTooLong> {
        let trimmed = text.trim();
        if trimmed.chars().count() > Self::MAX_CHARS {
            return Err(PlaceNameTooLong);
        }
        Ok(Self(trimmed.to_owned()))
    }

    #[must_use]
    pub fn to_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub const fn is_named(&self) -> bool {
        !self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_the_name_the_user_typed() {
        // Act
        let place = PlaceName::parse("  Maison \t").expect("a short name is valid");

        // Assert
        assert_eq!(place.to_str(), "Maison");
        assert!(place.is_named());
    }

    #[test]
    fn reads_blank_text_as_no_name() {
        // Act
        let place = PlaceName::parse("   ").expect("a blank name is valid");

        // Assert
        assert!(!place.is_named());
        assert_eq!(place, PlaceName::default());
    }

    #[test]
    fn counts_characters_not_bytes() {
        // Arrange
        let longest = "é".repeat(PlaceName::MAX_CHARS);

        // Act & Assert
        assert_eq!(
            PlaceName::parse(&longest).map(|place| place.to_str().chars().count()),
            Ok(PlaceName::MAX_CHARS)
        );
        assert_eq!(
            PlaceName::parse(&format!("{longest}é")),
            Err(PlaceNameTooLong)
        );
    }
}
