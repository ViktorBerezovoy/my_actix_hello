use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct SubscriberName(String);

impl SubscriberName {
    pub fn parse(s: String) -> Result<Self, String> {
        let is_empty_or_whitespace = s.trim().is_empty();
        let is_too_long = s.graphemes(true).count() > 256;
        let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
        let contains_forbidden_characters = s.chars().any(|c| forbidden_characters.contains(&c));

        if is_empty_or_whitespace || is_too_long || contains_forbidden_characters {
            Err(format!("{} is not a valid subsriber name.", s))
        } else {
            Ok(Self(s))
        }
    }
}
impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberName;

    #[test]
    fn a_256_graphem_long_name_is_valid() {
        let name = "a".repeat(256);
        SubscriberName::parse(name).unwrap();
    }
    #[test]
    fn a_name_longer_then_256_graphemes_is_rejected() {
        let name = "a".repeat(257);
        SubscriberName::parse(name).unwrap_err();
    }
    #[test]
    fn whitespace_only_names_are_rejected() {
        let name = " ".to_string();
        SubscriberName::parse(name).unwrap_err();
    }
    #[test]
    fn empty_string_is_rejected() {
        let name = "".to_string();
        SubscriberName::parse(name).unwrap_err();
    }
    #[test]
    fn names_contains_an_invalid_character_are_rejected() {
        for name in ['/', '(', ')', '"', '<', '>', '\\', '{', '}'] {
            let name = name.to_string();
            SubscriberName::parse(name).unwrap_err();
        }
    }
    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "Ursula Le Guin".to_string();
        SubscriberName::parse(name).unwrap();
    }
}
