use futures::stream::Stream;

use crate::database;

/// A [`Stream`] of database rows.
///
/// This is a "trait alias", used to simplify type signature.
/// `impl RowStream<'a, T>` is equivalent to
/// `impl Stream<Item = database::Result<T>> + Unpin + Send + 'a`.
pub trait RowStream<'a, T>: Stream<Item = database::Result<T>> + Unpin + Send + 'a {}

impl<'a, S, T> RowStream<'a, T> for S where S: Stream<Item = database::Result<T>> + Unpin + Send + 'a
{}
