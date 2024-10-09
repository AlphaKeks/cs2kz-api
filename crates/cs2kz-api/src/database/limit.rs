use super::macros;

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, From, Into, Deref)]
pub struct Limit(u64);

macros::wrap!(Limit as u64);
