//! Normal Inverse Gaussian distribution calculation.

use pyo3::types::{PyAnyMethods, PyList, PyTuple};
use pyo3::{PyResult, Python};

use crate::points::Record;

/// A Normal Inverse Gaussian distribution.
#[derive(Debug, Default, Clone, Copy)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct Distribution
{
	pub a: f64,
	pub b: f64,
	pub loc: f64,
	pub scale: f64,
	pub top_scale: f64,
}

impl Distribution
{
	/// Calculates the distribution for the given leaderboard.
	pub fn calculate(py: Python<'_>, records: &[Record]) -> PyResult<Self>
	{
		assert!(records.is_sorted(), "cannot calculate distribution data for unsorted input");

		if records.is_empty() {
			return Ok(Self::default());
		}

		let norminvgauss = py.import_bound("scipy.stats")?.getattr("norminvgauss")?;
		let (a, b, loc, scale) = norminvgauss
			.getattr("fit")?
			.call1((PyList::new_bound(py, records.iter().map(|record| record.time)),))?
			.downcast_into::<PyTuple>()?
			.extract::<(f64, f64, f64, f64)>()?;

		let sf = norminvgauss.call1((a, b, loc, scale))?.getattr("sf")?;
		let top_scale = sf.call1((records[0].time,))?.extract::<f64>()?;

		Ok(Self { a, b, loc, scale, top_scale })
	}

	/// Scales the given `values` according to this distribution.
	pub fn scale<I>(&self, values: I) -> impl Iterator<Item = f64> + use<'_, I>
	where
		I: IntoIterator<Item = f64>,
	{
		values
			.into_iter()
			.map(|value| (value - self.loc) / self.scale)
	}

	pub fn sf(&self, py: Python<'_>, value: f64) -> PyResult<f64>
	{
		let Self { a, b, loc, scale, .. } = self;

		py.import_bound("scipy.stats")?
			.getattr("norminvgauss")?
			.call1((a, b, loc, scale))?
			.getattr("sf")?
			.call1((value,))?
			.extract::<f64>()
	}
}
