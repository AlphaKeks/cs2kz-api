use super::macros;

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, From, Into, Deref)]
pub struct Offset(i64);

macros::wrap!(Offset as i64);
