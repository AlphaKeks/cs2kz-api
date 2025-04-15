use {super::EmailAddress, serde::Deserialize};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config
{
	pub relay: Box<str>,
	pub username: Box<str>,
	pub password: Box<str>,
	pub outgoing_address: EmailAddress,
}

impl Config
{
	pub fn credentials(&self) -> lettre::transport::smtp::authentication::Credentials
	{
		(String::from(&*self.username), String::from(&*self.password)).into()
	}

	pub fn outgoing_mailbox(&self) -> lettre::message::Mailbox
	{
		lettre::message::Mailbox {
			name: None,
			email: <EmailAddress as AsRef<lettre::Address>>::as_ref(&self.outgoing_address).clone(),
		}
	}
}
