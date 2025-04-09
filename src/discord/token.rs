use {serde::Deserialize, std::sync::Arc};

/// A Discord bot token
#[derive(Debug, Display, Clone, Deserialize)]
pub struct Token(Arc<str>);

impl Token
{
	pub fn as_str(&self) -> &str
	{
		&self.0
	}
}
