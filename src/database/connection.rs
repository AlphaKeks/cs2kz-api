use futures_util::TryFutureExt;
use sqlx::{MySql, MySqlConnection, pool::maybe::MaybePoolConnection};

use super::{DatabaseError, QueryBuilder};

/// A live database connection
#[must_use]
#[derive(Debug)]
pub struct DatabaseConnection<'c, 'args>
{
	#[debug(ignore)]
	inner: MaybePoolConnection<'c, MySql>,

	#[debug("{:?}", query.sql())]
	query: QueryBuilder<'args>,
}

impl<'c, 'args> DatabaseConnection<'c, 'args>
{
	pub(super) fn new(raw: impl Into<MaybePoolConnection<'c, MySql>>) -> Self
	{
		Self { inner: raw.into(), query: QueryBuilder::new("") }
	}

	#[must_use]
	pub(crate) fn as_raw(&mut self) -> &mut MySqlConnection
	{
		&mut self.inner
	}

	#[must_use]
	pub(crate) fn as_parts(&mut self) -> (&mut MySqlConnection, &mut QueryBuilder<'args>)
	{
		(&mut self.inner, &mut self.query)
	}

	/// Executes the given closure `f` inside the context of a transaction.
	#[tracing::instrument(level = "trace", skip_all)]
	pub async fn in_transaction<F, T, E>(&mut self, f: F) -> Result<T, E>
	where
		F: AsyncFnOnce(&mut DatabaseConnection<'_, '_>) -> Result<T, E>,
		DatabaseError: Into<E>,
	{
		let mut txn = sqlx::Connection::begin(&mut *self.inner)
			.map_err(DatabaseError::from)
			.map_err(Into::<E>::into)
			.await?;

		match f(&mut DatabaseConnection::new(&mut *txn)).await {
			Ok(value) => {
				tracing::trace!("committing transaction");
				txn.commit().map_err(DatabaseError::from).map_err(Into::<E>::into).await?;

				Ok(value)
			},
			Err(error) => {
				tracing::trace!("rolling back transaction");
				txn.rollback()
					.map_err(DatabaseError::from)
					.map_err(Into::<E>::into)
					.await?;

				Err(error)
			},
		}
	}
}
