use pyo3::types::{PyAnyMethods, PyList, PyTuple};

use crate::{points::LeaderboardPortion, python::PyState};

#[derive(Debug, Clone, Copy)]
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

impl Distribution
{
	/// Calculates the distribution parameters by fitting the given input
	/// `data`.
	///
	/// # Panics
	///
	/// This function will panic if `data` is empty.
	#[track_caller]
	#[tracing::instrument(skip(py_state, data, as_f64), fields(data.size = data.len()), ret(level = "debug"), err)]
	pub fn fit<T, F>(
		py_state: &PyState<'_>,
		data: &[T],
		mut as_f64: F,
	) -> Result<Self, DistributionError>
	where
		F: FnMut(&T) -> f64,
	{
		let norminvgauss = py_state
			.python()
			.import("scipy.stats")
			.inspect_err(|error| tracing::error!(%error, "failed to import scipy.stats"))?
			.getattr("norminvgauss")
			.inspect_err(|error| tracing::error!(%error, "failed to import norminvgauss"))?;

		let fit = norminvgauss
			.getattr("fit")
			.inspect_err(|error| tracing::error!(%error, "failed to import fit"))?;

		let top_value = data.first().map(&mut as_f64).unwrap_or_else(|| {
			panic!("`data` passed to `Distribution::fit()` is empty");
		});

		let data =
			PyList::new(py_state.python(), data.iter().map(as_f64)).inspect_err(|error| {
				tracing::error!(%error, "failed to create PyList from input data");
			})?;

		let (a, b, loc, scale) = fit
			.call1((data,))
			.inspect_err(|error| tracing::error!(%error, "failed to call fit"))?
			.downcast_into::<PyTuple>()
			.inspect_err(|error| tracing::error!(%error, "fit result is not a tuple"))?
			.extract::<(f64, f64, f64, f64)>()
			.inspect_err(|error| {
				tracing::error!(%error, "fit result is not a tuple of 4 floats");
			})?;

		let top_scale = norminvgauss
			.call1((a, b, loc, scale))
			.inspect_err(|error| {
				tracing::error!(%error, a, b, loc, scale, "failed to call norminvgauss");
			})?
			.getattr("sf")
			.inspect_err(|error| tracing::error!(%error, "failed to get sf"))?
			.call1((top_value,))
			.inspect_err(|error| tracing::error!(%error, input = top_value, "failed to call sf"))?
			.extract::<f64>()
			.map(LeaderboardPortion)
			.inspect_err(|error| {
				tracing::error!(%error, input = top_value, "sf result is not a float");
			})?;

		Ok(Self { a, b, loc, scale, top_scale })
	}

	/// Calls the distribution's survival function with the given `value` as the input.
	#[tracing::instrument(level = "trace", skip(py_state), ret(level = "trace"), err)]
	pub fn sf(&self, py_state: &PyState<'_>, value: f64) -> Result<f64, DistributionError>
	{
		let Distribution { a, b, loc, scale, .. } = *self;

		py_state
			.python()
			.import("scipy.stats")
			.inspect_err(|error| tracing::error!(%error, "failed to import scipy.stats"))?
			.getattr("norminvgauss")
			.inspect_err(|error| tracing::error!(%error, "failed to import norminvgauss"))?
			.call1((a, b, loc, scale))
			.inspect_err(|error| {
				tracing::error!(%error, a, b, loc, scale, "failed to call norminvgauss");
			})?
			.getattr("sf")
			.inspect_err(|error| tracing::error!(%error, "failed to get sf"))?
			.call1((value,))
			.inspect_err(|error| tracing::error!(%error, input = value, "failed to call sf"))?
			.extract::<f64>()
			.inspect_err(|error| {
				tracing::error!(%error, input = value, "sf result is not a float");
			})
			.map_err(DistributionError::from)
	}

	/// Scales the given `values` according to the distribution parameters.
	pub fn scale(&self, values: impl IntoIterator<Item = f64>) -> impl Iterator<Item = f64>
	{
		values
			.into_iter()
			.inspect(|&value| tracing::trace!(value, "before scale"))
			.map(|value| (value - self.loc) / self.scale)
			.inspect(|&value| tracing::trace!(value, "after scale"))
	}
}
