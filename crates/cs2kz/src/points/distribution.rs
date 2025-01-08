use pyo3::types::{PyAnyMethods, PyList, PyTuple};
use pyo3::{PyErr, Python};

use crate::num::AsF64;

/// [Normal-inverse Gaussian distribution][norminvgauss] parameters.
///
/// [norminvgauss]: https://en.wikipedia.org/wiki/Normal-inverse_Gaussian_distribution
#[derive(Debug, Default, Clone, Copy)]
pub struct Distribution {
	pub a: f64,
	pub b: f64,
	pub loc: f64,
	pub scale: f64,
	pub top_scale: f64,
}

impl Distribution {
	/// Calculates the distribution parameters using `times` as the input dataset.
	pub fn new(py: Python<'_>, times: &[impl AsF64]) -> Result<Self, PyErr> {
		let Some(top_time) = times.first().map(AsF64::as_f64) else {
			return Ok(Self::default());
		};

		let norminvgauss = py.import("scipy.stats")?.getattr("norminvgauss")?;

		let (a, b, loc, scale) = norminvgauss
			.getattr("fit")?
			.call1((PyList::new(py, times.iter().map(AsF64::as_f64))?,))?
			.downcast_into::<PyTuple>()?
			.extract::<(f64, f64, f64, f64)>()?;

		let top_scale = norminvgauss
			.call1((a, b, loc, scale))?
			.getattr("sf")?
			.call1((top_time,))?
			.extract::<f64>()?;

		Ok(Self { a, b, loc, scale, top_scale })
	}

	/// Scales the given `values` according to the distribution parameters.
	pub fn scale(&self, values: impl IntoIterator<Item: AsF64>) -> impl Iterator<Item = f64> {
		values
			.into_iter()
			.map(|value| (value.as_f64() - self.loc) / self.scale)
	}

	/// Calls the distribution's survival function with the given `value` as the input.
	pub fn sf(&self, py: Python<'_>, value: f64) -> Result<f64, PyErr> {
		let Distribution { a, b, loc, scale, .. } = *self;

		py.import("scipy.stats")?
			.getattr("norminvgauss")?
			.call1((a, b, loc, scale))?
			.getattr("sf")?
			.call1((value,))?
			.extract::<f64>()
	}
}
