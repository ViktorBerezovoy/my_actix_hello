use crate::domain::SubscriberEmail;
use crate::domain::subscriber_name::SubscriberName;

pub struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
}
impl NewSubscriber {
    pub fn build(email: String, name: String) -> Result<Self, String> {
        let name = SubscriberName::parse(name)?;
        let email = SubscriberEmail::parse(email)?;

        Ok(NewSubscriber { email, name })
    }
}
