use std::sync::Arc;

use serde::Deserialize;

#[derive(Debug, Display, Clone, Deserialize)]
pub struct Token(Arc<str>);

impl Token
{
	pub fn as_str(&self) -> &str
	{
		&self.0
	}
}
