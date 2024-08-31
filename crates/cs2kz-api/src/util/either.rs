use serde::{Deserialize, Serialize};

/// A value that is either of type `A` or `B`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Either<A, B>
{
	A(A),
	B(B),
}

impl<A, B> Either<A, B>
{
	pub const fn as_a(&self) -> Option<&A>
	{
		match self {
			Self::A(a) => Some(a),
			Self::B(_) => None,
		}
	}

	pub const fn as_b(&self) -> Option<&B>
	{
		match self {
			Self::A(_) => None,
			Self::B(b) => Some(b),
		}
	}
}
