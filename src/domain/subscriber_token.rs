use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(try_from = "String")]
pub struct SubscriberToken(String);

impl TryFrom<String> for SubscriberToken {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() != 25 {
            return Err(format!("Wrong token lenght, got {}", value.len()));
        }
        if !value.bytes().all(|b| b.is_ascii_alphabetic()) {
            return Err(format!("The token has wrong char {}", value));
        }

        Ok(Self(value))
    }
}

impl AsRef<str> for SubscriberToken {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
