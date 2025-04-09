use {
	super::{DatabaseError, QueryBuilder},
	futures_util::TryFutureExt,
	sqlx::{MySql, MySqlConnection, pool::maybe::MaybePoolConnection},
	std::{fmt, mem},
};

/// A live database connection
#[must_use]
#[derive(Debug)]
pub struct Connection<'c, 'q>
{
	#[debug(ignore)]
	raw: MaybePoolConnection<'c, MySql>,

	#[debug("{:?}", query.sql())]
	query: QueryBuilder<'q>,
}

impl<'c, 'q> Connection<'c, 'q>
{
	pub(super) fn from_raw(raw: impl Into<MaybePoolConnection<'c, MySql>>) -> Self
	{
		Self { raw: raw.into(), query: QueryBuilder::default() }
	}

	#[must_use]
	pub(crate) fn raw_mut(&mut self) -> &mut MySqlConnection
	{
		&mut self.raw
	}

	#[must_use]
	pub(crate) fn parts(&mut self) -> (&mut MySqlConnection, &mut QueryBuilder<'q>)
	{
		(&mut self.raw, &mut self.query)
	}

	/// Executes an `async` closure `f` inside the context of a transaction.
	///
	/// If the closure returns `Ok` the transaction is committed.
	/// If the closure returns `Err` the transaction is rolled back.
	#[instrument(level = "trace", skip_all, err(level = "debug"))]
	pub async fn in_transaction<F, T, E>(&mut self, f: F) -> Result<T, E>
	where
		F: AsyncFnOnce(&mut Connection<'_, '_>) -> Result<T, E>,
		E: fmt::Display,
		DatabaseError: Into<E>,
	{
		let mut txn = sqlx::Connection::begin(&mut *self.raw)
			.map_err(DatabaseError::from)
			.map_err(Into::<E>::into)
			.await?;

		let mut conn = Connection {
			raw: MaybePoolConnection::Connection(&mut *txn),
			query: mem::take(&mut self.query),
		};

		let result = f(&mut conn).await;
		let Connection { query, .. } = conn;

		self.query = query;

		match result {
			Ok(value) => {
				trace!("committing transaction");
				txn.commit().map_err(DatabaseError::from).map_err(Into::<E>::into).await?;

				Ok(value)
			},
			Err(error) => {
				trace!("rolling back transaction");
				txn.rollback()
					.map_err(DatabaseError::from)
					.map_err(Into::<E>::into)
					.await?;

				Err(error)
			},
		}
	}
}
