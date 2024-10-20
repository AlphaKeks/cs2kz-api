#![allow(unused)]

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub(crate) enum Either<A, B> {
	A(A),
	B(B),
}
