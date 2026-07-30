use crate::domain::subscriber_email::SubscriberEnail;
use crate::domain::subscriber_name::SubscriberName;

pub struct NewSubscriber {
    pub email: SubscriberEnail,
    pub name: SubscriberName,
}
impl NewSubscriber {
    pub fn build(email: String, name: String) -> Result<Self, String> {
        let name = SubscriberName::parse(name)?;
        let email = SubscriberEnail::parse(email)?;

        Ok(NewSubscriber { email, name })
    }
}
