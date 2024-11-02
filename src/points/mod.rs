//! Leaderboard point calculations.

#![allow(dead_code)] // FIXME

use std::cmp;

use cs2kz::Tier;
use pyo3::{PyResult, Python};

#[cfg(test)]
mod tests;

mod record;
pub use record::Record;

mod distribution;
pub use distribution::Distribution;

mod daemon;
#[allow(unused_imports)] // FIXME
pub use daemon::run_daemon;

/// Results of a round of calculating points.
pub struct Results
{
	/// The distribution data for the input.
	pub distribution: Distribution,

	pub sf_values: Vec<f64>,

	/// The points calculated for each record.
	///
	/// This is guaranteed to have the same length as the input slice, and each
	/// element maps to the record with the same index.
	pub points: Vec<u16>,
}

/// Extra state for calculating points for each record.
struct State<'a>
{
	/// The distribution data calculated for the entire input set.
	distribution: &'a Distribution,

	sf_values: &'a mut Vec<f64>,

	/// The current record.
	record: &'a Record,

	/// The rank of the current record on the leaderboard.
	rank: usize,
}

/// Calculates points for the NUB leaderboard.
pub fn calculate_nub(tier: Tier, records: &[Record]) -> PyResult<Results>
{
	Python::with_gil(|py| {
		calculate(py, records, |State { distribution, sf_values, record, rank }| {
			distribution.calculate_points(py, record, rank, tier, sf_values, false, true)
		})
	})
}

/// Calculates points for the PRO leaderboard.
pub fn calculate_pro(
	tier: Tier,
	pro_records: &[Record],
	nub_records: &[Record],
	nub_results: &mut Results,
) -> PyResult<Results>
{
	Python::with_gil(|py| {
		calculate(py, pro_records, |State { distribution, sf_values, record, rank }| {
			let pro_points =
				distribution.calculate_points(py, record, rank, tier, sf_values, true, true)?;

			// figure out which rank this record would be, if it was in the NUB leaderboard
			// instead
			let (Ok(rank) | Err(rank)) =
				nub_records.binary_search_by(|record| record.time.total_cmp(&record.time));

			let nub_points = nub_results.distribution.calculate_points(
				py,
				record,
				rank,
				tier,
				&mut nub_results.sf_values,
				false,
				false,
			)?;

			Ok(cmp::max(nub_points, pro_points))
		})
	})
}

/// Shared logic for `calculate_nub` and `calculate_pro`.
fn calculate<F>(py: Python<'_>, records: &[Record], mut calc_points: F) -> PyResult<Results>
where
	F: FnMut(State<'_>) -> PyResult<u16>,
{
	let distribution = Distribution::new(py, records)?;
	let mut sf_values = Vec::with_capacity(records.len());
	let points = records
		.iter()
		.enumerate()
		.map(|(rank, record)| {
			calc_points(State {
				distribution: &distribution,
				sf_values: &mut sf_values,
				record,
				rank,
			})
		})
		.collect::<PyResult<Vec<u16>>>()?;

	Ok(Results { distribution, sf_values, points })
}
