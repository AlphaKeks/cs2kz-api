//! SQL query utilities, such as extension traits and wrapper types.

use std::fmt::Display;

use derive_more::{Debug, Deref, DerefMut, From, Into};
use sqlx::{MySql, QueryBuilder, Transaction};

use crate::openapi::parameters::{Limit, Offset, SortingOrder};

/// Returns the total number of rows that _could_ be fetched from the previous query, ignoring
/// `LIMIT`.
///
/// NOTE: This only works if the query contained `SQL_CALC_FOUND_ROWS`!
///
/// # Panics
///
/// This function will panic if the database ever returns a negative number for `FOUND_ROWS()`.
pub async fn total_rows(transaction: &mut Transaction<'_, MySql>) -> sqlx::Result<u64> {
	let total = sqlx::query_scalar!("SELECT FOUND_ROWS() as total")
		.fetch_one(transaction.as_mut())
		.await?
		.try_into()
		.expect("positive row count");

	Ok(total)
}

/// Extension trait for [`sqlx::QueryBuilder`].
pub trait QueryBuilderExt {
	/// Push `LIMIT` and `OFFSET` clauses into the query.
	fn push_limits(&mut self, limit: Limit, offset: Offset) -> &mut Self;

	/// Push an `ORDER BY` clause into the query.
	fn order_by<C>(&mut self, order: SortingOrder, columns: C) -> &mut Self
	where
		C: Display;
}

impl QueryBuilderExt for QueryBuilder<'_, MySql> {
	fn push_limits(&mut self, limit: Limit, offset: Offset) -> &mut Self {
		self.push(" LIMIT ")
			.push_bind(limit)
			.push(" OFFSET ")
			.push_bind(offset)
	}

	fn order_by<C>(&mut self, order: SortingOrder, columns: C) -> &mut Self
	where
		C: Display,
	{
		self.push(" ORDER BY ").push(columns).push(order.sql())
	}
}

/// Wrapper around [`QueryBuilder`] with helper methods for pushing `WHERE` / `AND` clauses into
/// the query.
#[derive(Debug, Deref, DerefMut, From, Into)]
pub struct FilteredQuery<'args> {
	/// The wrapped [`QueryBuilder`].
	#[deref]
	#[deref_mut]
	#[from]
	#[into]
	#[debug(skip)]
	query_builder: QueryBuilder<'args, MySql>,

	/// The current filter state.
	current_filter: Filter,
}

/// The two keywords used in `WHERE` clauses.
#[derive(Debug, Default, Clone, Copy)]
pub enum Filter {
	/// SQL `WHERE` clause.
	#[default]
	Where,

	/// SQL `AND` clause.
	And,
}

impl Filter {
	/// The corresponding SQL code for the current state.
	const fn sql(&self) -> &'static str {
		match *self {
			Self::Where => " WHERE ",
			Self::And => " AND ",
		}
	}
}

impl<'args> FilteredQuery<'args> {
	/// Creates a new [`FilteredQuery`].
	pub fn new<S>(query: S) -> Self
	where
		S: Display,
	{
		Self {
			query_builder: QueryBuilder::new(format!("UPDATE {query} ")),
			current_filter: Filter::default(),
		}
	}

	/// Returns back the wrapped [`QueryBuilder`].
	pub fn into_inner(self) -> QueryBuilder<'args, MySql> {
		self.query_builder
	}

	/// Inserts a filter for `$column $value`.
	///
	/// # Example
	///
	/// ```rust,ignore
	/// let mut query = FilteredQuery::new("SELECT * FROM table");
	///
	/// if let Some(thing) = maybe_thing {
	///     query.filter("thing = ", thing);
	/// }
	/// ```
	pub fn filter<C, V>(&mut self, column: C, value: V) -> &mut Self
	where
		C: Display,
		V: sqlx::Type<MySql> + sqlx::Encode<'args, MySql> + Send + 'args,
	{
		self.query_builder
			.push(self.current_filter.sql())
			.push(column)
			.push_bind(value);

		self.current_filter = Filter::And;
		self
	}
}

/// Wrapper around [`QueryBuilder`] with helper methods for pushing `SET` clauses in an `UPDATE`
/// query.
#[derive(Debug, Deref, DerefMut, From, Into)]
pub struct UpdateQuery<'args> {
	/// The wrapped [`QueryBuilder`].
	#[deref]
	#[deref_mut]
	#[from]
	#[into]
	#[debug(skip)]
	query_builder: QueryBuilder<'args, MySql>,

	/// The current filter state.
	current_delimiter: UpdateDelimiter,
}

/// The two delimiters used in an `UPDATE` query.
#[derive(Debug, Default, Clone, Copy)]
pub enum UpdateDelimiter {
	/// SQL `SET` clause.
	#[default]
	Set,

	/// `, `
	Comma,
}

impl UpdateDelimiter {
	/// The corresponding SQL code for the current state.
	const fn sql(&self) -> &'static str {
		match *self {
			UpdateDelimiter::Set => " SET ",
			UpdateDelimiter::Comma => ", ",
		}
	}
}

impl<'args> UpdateQuery<'args> {
	/// Creates a new [`UpdateQuery`].
	pub fn new<S>(query: S) -> Self
	where
		S: Into<String>,
	{
		Self {
			query_builder: QueryBuilder::new(query),
			current_delimiter: UpdateDelimiter::default(),
		}
	}

	/// Returns back the wrapped [`QueryBuilder`].
	pub fn into_inner(self) -> QueryBuilder<'args, MySql> {
		self.query_builder
	}

	/// Sets the specified `column` to `value`.
	///
	/// # Example
	///
	/// ```rust,ignore
	/// let mut query = UpdateQuery::new("table");
	///
	/// if let Some(thing) = maybe_thing {
	///     query.set("thing", thing);
	/// }
	/// ```
	pub fn set<C, V>(&mut self, column: C, value: V) -> &mut Self
	where
		C: Display,
		V: sqlx::Type<MySql> + sqlx::Encode<'args, MySql> + Send + 'args,
	{
		self.query_builder
			.push(self.current_delimiter.sql())
			.push(column)
			.push(" = ")
			.push_bind(value);

		self.current_delimiter = UpdateDelimiter::Comma;
		self
	}
}
