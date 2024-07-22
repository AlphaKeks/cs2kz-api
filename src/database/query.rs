//! Helpers for dealing with SQL queries.

use std::fmt;
use std::future::Future;
use std::ops::{Deref, DerefMut};

use sqlx::{MySql, QueryBuilder, Transaction};

/// Extension trait for [`sqlx::QueryBuilder`].
///
/// Provides some helpful methods.
#[sealed]
pub trait QueryBuilderExt
{
	/// Pushes `LIMIT` and `OFFSET` clauses into the query.
	fn push_limits(&mut self, limit: u64, offset: u64) -> &mut Self;
}

#[sealed]
impl<'q, DB> QueryBuilderExt for QueryBuilder<'q, DB>
where
	DB: sqlx::Database,
	u64: sqlx::Type<DB> + sqlx::Encode<'q, DB>,
	i64: sqlx::Type<DB> + sqlx::Encode<'q, DB>,
{
	fn push_limits(&mut self, limit: u64, offset: u64) -> &mut Self
	{
		self.push(" LIMIT ")
			.push_bind(limit)
			.push(" OFFSET ")
			.push_bind(offset)
	}
}

/// Extension trait for [`sqlx::Transaction`].
///
/// Provides some helpful methods.
#[sealed]
pub trait TransactionExt
{
	/// Returns the **total** amount of rows that _could have been_ fetched by
	/// the previous `SELECT` query, ignoring `LIMIT`.
	///
	/// NOTE: **this only works if the query contained `SQL_CALC_FOUND_ROWS`**
	fn total_rows(&mut self) -> impl Future<Output = sqlx::Result<u64>> + Send;
}

#[sealed]
impl<'c> TransactionExt for Transaction<'c, MySql>
{
	async fn total_rows(&mut self) -> sqlx::Result<u64>
	{
		let total = sqlx::query_scalar!("SELECT FOUND_ROWS() as total")
			.fetch_one(self.as_mut())
			.await?
			.try_into()
			.expect("positive count");

		Ok(total)
	}
}

/// A wrapper around [`sqlx::QueryBuilder`] that allows easily building queries
/// with one or more `WHERE` filters.
pub struct FilteredQueryBuilder<'args, DB>
where
	DB: sqlx::Database,
{
	/// The underlying query builder.
	query: QueryBuilder<'args, DB>,

	/// Whether we already pushed `WHERE`.
	has_where: bool,
}

impl<'args, DB> FilteredQueryBuilder<'args, DB>
where
	DB: sqlx::Database,
{
	/// Creates a new [`FilteredQueryBuilder`].
	pub fn new(query: impl Into<String>) -> Self
	{
		Self { query: QueryBuilder::new(query), has_where: false }
	}

	/// Adds a filter into the query.
	///
	/// # Example
	///
	/// ```rust,ignore
	/// let mut query = FilteredQuery::new("SELECT * FROM table");
	///
	/// if condition1 {
	///     query.filter("foo = ", bar);
	/// }
	///
	/// if condition2 {
	///     query.filter("baz > ", 69);
	/// }
	///
	/// let result = query.build().fetch_all(&database).await?;
	/// ```
	pub fn filter<V>(&mut self, column: impl fmt::Display, value: V) -> &mut Self
	where
		V: sqlx::Type<DB> + sqlx::Encode<'args, DB> + Send + 'args,
	{
		self.query
			.push(if self.has_where { " AND " } else { " WHERE " })
			.push(column)
			.push_bind(value);

		self.has_where = true;
		self
	}

	/// Adds an `IS (NOT) NULL` filter into the query.
	///
	/// # Example
	///
	/// ```rust,ignore
	/// let mut query = FilteredQuery::new("SELECT * FROM table");
	///
	/// if let Some(should_be_null) = some_param {
	///     query.filter_is_null("foo ", should_be_null);
	/// }
	///
	/// let result = query.build().fetch_all(&database).await?;
	/// ```
	pub fn filter_is_null(&mut self, column: impl fmt::Display, is_null: bool) -> &mut Self
	{
		self.query
			.push(if self.has_where { " AND " } else { " WHERE " })
			.push(column)
			.push(" IS ");

		if !is_null {
			self.query.push(" NOT ");
		}

		self.query.push(" NULL ");
		self.has_where = true;
		self
	}

	/// Returns the underlying query builder.
	pub fn into_inner(self) -> QueryBuilder<'args, DB>
	{
		self.query
	}
}

impl<'args, DB> Deref for FilteredQueryBuilder<'args, DB>
where
	DB: sqlx::Database,
{
	type Target = QueryBuilder<'args, DB>;

	fn deref(&self) -> &Self::Target
	{
		&self.query
	}
}

impl<'args, DB> DerefMut for FilteredQueryBuilder<'args, DB>
where
	DB: sqlx::Database,
{
	fn deref_mut(&mut self) -> &mut Self::Target
	{
		&mut self.query
	}
}

/// A wrapper around [`sqlx::QueryBuilder`] that allows easily building an
/// `UPDATE` query.
pub struct UpdateQueryBuilder<'args, DB>
where
	DB: sqlx::Database,
{
	/// The underlying query builder.
	query: QueryBuilder<'args, DB>,

	/// Whether we already pushed `SET`.
	has_set: bool,
}

impl<'args, DB> UpdateQueryBuilder<'args, DB>
where
	DB: sqlx::Database,
{
	/// Creates a new [`UpdateQueryBuilder`].
	pub fn new(table: impl fmt::Display) -> Self
	{
		Self { query: QueryBuilder::new(format!("UPDATE {table} ")), has_set: false }
	}

	/// Adds an update into the query.
	///
	/// # Example
	///
	/// ```rust,ignore
	/// let mut query = UpdateQuery::new("table");
	///
	/// if condition1 {
	///     query.set("foo", bar);
	/// }
	///
	/// if condition2 {
	///     query.set("baz", 69);
	/// }
	/// let result = query.build().execute(&database).await?;
	/// ```
	pub fn set<V>(&mut self, column: impl fmt::Display, value: V) -> &mut Self
	where
		V: sqlx::Type<DB> + sqlx::Encode<'args, DB> + Send + 'args,
	{
		self.query
			.push(if self.has_set { ", " } else { " SET " })
			.push(column)
			.push(" = ")
			.push_bind(value);

		self.has_set = true;
		self
	}

	/// Returns the underlying query builder.
	pub fn into_inner(self) -> QueryBuilder<'args, DB>
	{
		self.query
	}
}

impl<'args, DB> Deref for UpdateQueryBuilder<'args, DB>
where
	DB: sqlx::Database,
{
	type Target = QueryBuilder<'args, DB>;

	fn deref(&self) -> &Self::Target
	{
		&self.query
	}
}

impl<'args, DB> DerefMut for UpdateQueryBuilder<'args, DB>
where
	DB: sqlx::Database,
{
	fn deref_mut(&mut self) -> &mut Self::Target
	{
		&mut self.query
	}
}
