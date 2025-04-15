pub use self::config::Config;
use {
	super::EmailAddress,
	futures_util::TryFutureExt,
	lettre::AsyncTransport as _,
	std::{error::Error, fmt, sync::Arc},
};

mod config;

#[derive(Debug, Clone)]
pub struct Client
{
	transport: lettre::AsyncSmtpTransport<lettre::Tokio1Executor>,
	outgoing_mailbox: Arc<lettre::message::Mailbox>,
}

#[derive(Debug, Display, Error, From)]
#[display("failed to create email client")]
pub struct CreateClientError(lettre::transport::smtp::Error);

#[derive(Debug, Display, Error, From)]
#[display("failed to test connection")]
pub struct TestConnectionError(lettre::transport::smtp::Error);

#[derive(Debug, Display, Error, From)]
#[display("failed to send email")]
pub struct SendEmailError(lettre::transport::smtp::Error);

#[bon::bon]
impl Client
{
	pub fn new(email_config: &Config) -> Result<Self, CreateClientError>
	{
		let transport =
			lettre::AsyncSmtpTransport::<lettre::Tokio1Executor>::relay(&*email_config.relay)?
				.credentials(email_config.credentials())
				.build::<lettre::Tokio1Executor>();

		Ok(Self { transport, outgoing_mailbox: Arc::new(email_config.outgoing_mailbox()) })
	}

	#[instrument(level = "trace", ret(level = "debug"), err(Debug))]
	pub async fn test_connection(&self) -> Result<bool, TestConnectionError>
	{
		self.transport.test_connection().map_err(TestConnectionError::from).await
	}

	#[instrument(level = "debug", skip(subject, body), fields(%subject), err(Debug))]
	#[builder(start_fn = build_message, finish_fn = send)]
	pub async fn send_message(
		&self,
		#[builder(start_fn)] subject: impl fmt::Display,
		#[builder(finish_fn)] to: &EmailAddress,
		body: impl fmt::Display,
	) -> Result<(), SendEmailError>
	{
		let to = lettre::message::Mailbox {
			name: None,
			email: <EmailAddress as AsRef<lettre::Address>>::as_ref(to).clone(),
		};

		let message = lettre::Message::builder()
			.from((*self.outgoing_mailbox).clone())
			.to(to)
			.subject(subject.to_string())
			.body(body.to_string())
			.unwrap_or_else(|err| panic!("failed to construct email: {err}"));

		match self.transport.send(message).await {
			Ok(response) => {
				info!(response.code = ?response.code(), "email sent");
				Ok(())
			},
			Err(err) => {
				error!(error = &err as &dyn Error, "failed to send email");
				Err(SendEmailError(err))
			},
		}
	}
}
