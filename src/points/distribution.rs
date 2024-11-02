//! Point distribution.

use cs2kz::Tier;
use pyo3::types::{PyAny, PyAnyMethods, PyList, PyTuple};
use pyo3::{Py, PyResult, Python};

use super::Record;

/// The threshold for leaderboards with "low completion" (few entries).
const LOW_COMPLETION_THRESHOLD: usize = 50;

/// A Normal Inverse Gaussian distribution.
pub struct Distribution
{
	/// The size of the input.
	input_size: usize,

	/// The top value of the input.
	top_time: f64,

	/// The result of calling the survival function with `top_time` as the
	/// input.
	top_sf: f64,

	/// `time`s scaled to the survival function's parameters.
	scaled_times: Vec<f64>,

	/// Parameters for the distribution.
	dist_params: [f64; 4],

	/// The survival function.
	sf: Py<PyAny>,
}

impl Distribution
{
	/// Calculates distribution parameters for the given `records`.
	pub fn new(py: Python<'_>, records: &[Record]) -> PyResult<Self>
	{
		debug_assert!(records.is_sorted(), "records should be sorted");

		let input_size = records.len();
		let top_time = records[0].time;
		let norminvgauss = py.import_bound("scipy.stats")?.getattr("norminvgauss")?;
		let dist_params = norminvgauss
			.getattr("fit")?
			.call1((PyList::new_bound(py, records.iter().map(|record| record.time)),))?
			.downcast_into::<PyTuple>()?
			.extract::<(f64, f64, f64, f64)>()?;

		let sf = norminvgauss.call1(dist_params)?.getattr("sf")?;
		let top_sf = sf.call1((top_time,))?.extract::<f64>()?;
		let scaled_times = records
			.iter()
			.map(|record| (record.time - dist_params.2) / dist_params.3)
			.collect();

		Ok(Self {
			input_size,
			top_time,
			top_sf,
			scaled_times,
			dist_params: dist_params.into(),
			sf: sf.unbind(),
		})
	}

	/// Calculates points for the given `record`.
	pub fn calculate_points(
		&self,
		py: Python<'_>,
		record: &Record,
		rank: usize,
		tier: Tier,
		sf_values: &mut Vec<f64>,
		is_pro_leaderboard: bool,
		should_push: bool,
	) -> PyResult<u16>
	{
		let mut minimum_points = match tier {
			Tier::VeryEasy => 0.0,
			Tier::Easy => 500.0,
			Tier::Medium => 2000.0,
			Tier::Advanced => 3500.0,
			Tier::Hard => 5000.0,
			Tier::VeryHard => 6500.0,
			Tier::Extreme => 8000.0,
			Tier::Death => 9500.0,
			Tier::Unfeasible | Tier::Impossible => unreachable!(),
		};

		if is_pro_leaderboard {
			minimum_points += (10_000.0 - minimum_points) * 0.1;
		}

		let remaining_points = 10_000.0 - minimum_points;
		let rank_points = 0.25 * remaining_points * points_for_rank(self.input_size, rank);
		let dist_points = 0.75
			* remaining_points
			* (if self.input_size < LOW_COMPLETION_THRESHOLD {
				points_for_low_completion(self.top_time, tier, record.time)
			} else {
				let next_sf = calculate_next_sf_value(
					py,
					&self.scaled_times,
					&self.dist_params,
					self.top_sf,
					sf_values,
					rank,
					should_push,
				)?;

				(next_sf / self.top_sf).min(1.0)
			});

		Ok((minimum_points + dist_points + rank_points) as u16)
	}

	/// Calls the survival function with the given `input`.
	fn sf(&self, py: Python<'_>, input: f64) -> PyResult<f64>
	{
		self.sf.call1(py, (input,))?.extract::<f64>(py)
	}
}

/// Calculates points for a specific rank on the leaderboard.
fn points_for_rank(input_size: usize, rank: usize) -> f64
{
	let mut points = 0.5 * (1.0 - rank as f64 / input_size as f64);

	if rank < 100 {
		points += (100 - rank) as f64 * 0.002;
	}

	if rank < 20 {
		points += (20 - rank) as f64 * 0.01;
	}

	if let Some(extra) = [0.1, 0.06, 0.045, 0.03, 0.01].get(rank) {
		points += *extra;
	}

	points
}

/// Calculates points for a time on a leaderboard with few entries.
fn points_for_low_completion(top_time: f64, tier: Tier, time: f64) -> f64
{
	let tier = u8::from(tier) as f64;
	let x = 2.1 - 0.25 * tier;
	let y = 1.0 + (x * -0.5).exp();
	let z = 1.0 + (x * (time / top_time - 1.5)).exp();

	y / z
}

fn calculate_next_sf_value(
	py: Python<'_>,
	scaled_times: &[f64],
	dist_params: &[f64; 4],
	top_sf: f64,
	sf_values: &mut Vec<f64>,
	rank: usize,
	should_push: bool,
) -> PyResult<f64>
{
	if rank == 0 {
		if should_push {
			sf_values.push(top_sf);
		}

		return Ok(top_sf);
	}

	let mut next = sf_values.get(rank - 1).copied().unwrap();

	if scaled_times[rank - 1] != scaled_times[rank] {
		let integrate = py.import_bound("scipy")?.getattr("integrate")?;
		let norminvgauss = py.import_bound("scipy.stats")?.getattr("norminvgauss")?;
		let (thing, _) = integrate
			.getattr("quad")?
			.call1((
				norminvgauss.getattr("_pdf")?,
				scaled_times[rank - 1],
				scaled_times[rank],
				(dist_params[0], dist_params[1]),
			))?
			.downcast_into::<PyTuple>()?
			.extract::<(f64, f64)>()?;

		next -= thing;
	}

	if should_push {
		sf_values.push(next);
	}

	Ok(next)
}
