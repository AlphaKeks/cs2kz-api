use {
	crate::{
		database::{self, DatabaseError, DatabaseResult},
		maps::FilterId,
		points::LeaderboardPortion,
		python::PyState,
		records::Leaderboard,
	},
	futures_util::TryFutureExt,
	pyo3::types::{PyAnyMethods, PyList, PyTuple},
};

/// Parameters for a [Normal Inverse Gaussian distribution][norminvgauss]
///
/// [norminvgauss]: https://en.wikipedia.org/wiki/Normal-inverse_Gaussian_distribution
#[derive(Debug, Clone, Copy, sqlx::FromRow)]
pub struct Distribution
{
	pub a: f64,
	pub b: f64,
	pub loc: f64,
	pub scale: f64,
	pub top_scale: LeaderboardPortion,
}

#[derive(Debug, Display, Error)]
#[display("{message}")]
pub struct DistributionError
{
	#[error(ignore)]
	message: Box<str>,
}

impl From<pyo3::PyErr> for DistributionError
{
	fn from(err: pyo3::PyErr) -> Self
	{
		Self { message: err.to_string().into_boxed_str() }
	}
}

impl From<pyo3::DowncastIntoError<'_>> for DistributionError
{
	fn from(err: pyo3::DowncastIntoError<'_>) -> Self
	{
		Self { message: err.to_string().into_boxed_str() }
	}
}

#[bon::bon]
impl Distribution
{
	/// Returns the cached distribution parameters for the given filter and
	/// leaderboard.
	#[instrument(skip(db_conn), ret(level = "debug"), err)]
	#[builder(finish_fn = exec)]
	pub(crate) async fn get_cached(
		#[builder(start_fn)] filter_id: FilterId,
		#[builder(start_fn)] leaderboard: Leaderboard,
		#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	) -> DatabaseResult<Option<Self>>
	{
		let (conn, query) = db_conn.parts();
		query.reset();

		query.push(format_args! {
			"SELECT a, b, loc, scale, top_scale
			 FROM {leaderboard}
			 WHERE filter_id = ?",
			leaderboard = match leaderboard {
				Leaderboard::NUB => "DistributionParameters",
				Leaderboard::PRO => "ProDistributionParameters",
			},
		});

		query
			.build_query_as::<Self>()
			.bind(filter_id)
			.fetch_optional(conn)
			.map_err(DatabaseError::from)
			.await
	}

	/// Calculates the distribution parameters by fitting the given input `data`.
	///
	/// # Panics
	///
	/// This function will panic if `data` is empty.
	#[track_caller]
	#[instrument(skip_all, fields(data.size = data.len()), ret(level = "debug"), err)]
	pub fn fit<T, F>(
		py_state: &PyState<'_>,
		data: &[T],
		mut as_f64: F,
	) -> Result<Self, DistributionError>
	where
		F: FnMut(&T) -> f64,
	{
		assert!(!data.is_empty(), "`data` passed to `Distribution::fit()` is empty");

		let norminvgauss = py_state
			.python()
			.import("scipy.stats")
			.inspect_err(|error| error!(%error, "failed to import scipy.stats"))?
			.getattr("norminvgauss")
			.inspect_err(|error| error!(%error, "failed to import norminvgauss"))?;

		let fit = norminvgauss
			.getattr("fit")
			.inspect_err(|error| error!(%error, "failed to import fit"))?;

		let top_value = as_f64(&data[0]);

		if cfg!(debug_assertions) {
			data[1..].iter().map(&mut as_f64).for_each(|value| {
				assert!(value >= top_value, "{value} < {top_value}");
			});
		}

		let data =
			PyList::new(py_state.python(), data.iter().map(as_f64)).inspect_err(|error| {
				error!(%error, "failed to create PyList from input data");
			})?;

		let (a, b, loc, scale) = fit
			.call1((data,))
			.inspect_err(|error| error!(%error, "failed to call fit"))?
			.downcast_into::<PyTuple>()
			.inspect_err(|error| error!(%error, "fit result is not a tuple"))?
			.extract::<(f64, f64, f64, f64)>()
			.inspect_err(|error| {
				error!(%error, "fit result is not a tuple of 4 floats");
			})?;

		let top_scale = norminvgauss
			.call1((a, b, loc, scale))
			.inspect_err(|error| {
				error!(%error, a, b, loc, scale, "failed to call norminvgauss");
			})?
			.getattr("sf")
			.inspect_err(|error| error!(%error, "failed to get sf"))?
			.call1((top_value,))
			.inspect_err(|error| error!(%error, input = top_value, "failed to call sf"))?
			.extract::<f64>()
			.map(LeaderboardPortion)
			.inspect_err(|error| {
				error!(%error, input = top_value, "sf result is not a float");
			})?;

		Ok(Self { a, b, loc, scale, top_scale })
	}

	/// Calls the distribution's survival function with the given `value` as the input.
	#[instrument(level = "trace", skip(py_state), ret(level = "trace"), err)]
	pub fn sf(&self, py_state: &PyState<'_>, value: f64) -> Result<f64, DistributionError>
	{
		let Distribution { a, b, loc, scale, .. } = *self;

		py_state
			.python()
			.import("scipy.stats")
			.inspect_err(|error| error!(%error, "failed to import scipy.stats"))?
			.getattr("norminvgauss")
			.inspect_err(|error| error!(%error, "failed to import norminvgauss"))?
			.call1((a, b, loc, scale))
			.inspect_err(|error| {
				error!(%error, a, b, loc, scale, "failed to call norminvgauss");
			})?
			.getattr("sf")
			.inspect_err(|error| error!(%error, "failed to get sf"))?
			.call1((value,))
			.inspect_err(|error| error!(%error, input = value, "failed to call sf"))?
			.extract::<f64>()
			.inspect_err(|error| {
				error!(%error, input = value, "sf result is not a float");
			})
			.map_err(DistributionError::from)
	}

	/// Scales the given `values` according to the distribution parameters.
	pub fn scale(&self, values: impl IntoIterator<Item = f64>) -> impl Iterator<Item = f64>
	{
		values
			.into_iter()
			.inspect(|&value| trace!(value, "before scale"))
			.map(|value| (value - self.loc) / self.scale)
			.inspect(|&value| trace!(value, "after scale"))
	}
}
