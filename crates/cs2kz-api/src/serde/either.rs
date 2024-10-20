/// A deserialization helper that attempts to deserialize two types `A` and `B`, in that order.
///
/// Because [`serde::Deserialize`] methods all take `self` by ownership, it is difficult to just
/// "try and deserialize 1 of N different types".
///
/// `#[derive(serde::Deserialize)]` + `#[serde(untagged)]` on an enum does exactly what we want,
/// and if more than 2 possible types exist, you can simply nest this enum.
///
/// See usages for examples.
#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum Either<A, B> {
	A(A),
	B(B),
}
