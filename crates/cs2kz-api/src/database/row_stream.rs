use futures::stream::Stream;

use crate::database;

/// A [`Stream`] of database rows.
///
/// This is a "trait alias", used to simplify type signatures. `impl RowStream<'c, T>` is
/// equivalent to `impl Stream<Item = database::Result<T>> + Unpin + Send + 'c`.
pub trait RowStream<'c, T>: Stream<Item = database::Result<T>> + Unpin + Send + 'c {}

impl<'c, S, T> RowStream<'c, T> for S where S: Stream<Item = database::Result<T>> + Unpin + Send + 'c
{}
