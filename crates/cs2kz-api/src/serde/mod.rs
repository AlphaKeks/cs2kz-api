//! Custom [`serde`] helper types and functions used via `#[serde(with = "…")]`
//! / `#[serde(serialize_with = "…")]` / `[serde(deserialize_with = "…")]` or custom `Serialize`
//! / `Deserialize` implementations.

mod non_empty;
#[expect(unused_imports)]
pub use non_empty::deserialize_non_empty;

mod either;
pub use either::Either;
