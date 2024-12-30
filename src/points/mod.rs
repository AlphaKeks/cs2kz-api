//! Leaderboard Points calculations.

use cs2kz::Tier;
use pyo3::types::{PyAnyMethods, PyTuple};
use pyo3::{PyResult, Python};

#[cfg(test)]
mod tests;

mod record;
pub use record::Record;

mod distribution;
pub use distribution::Distribution;

mod worker;
use worker::Worker;

pub mod daemon;

/// The maximum points that can be awarded for a run.
pub const MAX_POINTS: f64 = 10_000.0;

/// The threshold for leaderboards with "low completion" (few entries).
pub const LOW_COMPLETION_THRESHOLD: usize = 50;

#[derive(Debug, Clone, Copy)]
pub struct Points
{
	minimum: f64,
	rank: f64,
	dist: f64,
}

impl Points
{
	pub fn total(self) -> f64
	{
		self.minimum + self.rank + self.dist
	}
}

/// Calculates the minimum amount of points rewarded for completing a filter of
/// a given tier.
///
/// [`Unfeasible`] and [`Impossible`] should never appear in ranked filters, and
/// will result in [`None`].
///
/// [`Unfeasible`]: Tier::Unfeasible
/// [`Impossible`]: Tier::Impossible
#[cfg_attr(not(test), expect(dead_code, reason = "will be used later"))]
pub fn minimum_points(tier: Tier, is_pro_leaderboard: bool) -> Option<f64>
{
	let mut points = match tier {
		Tier::VeryEasy => 0.0,
		Tier::Easy => 500.0,
		Tier::Medium => 2000.0,
		Tier::Advanced => 3500.0,
		Tier::Hard => 5000.0,
		Tier::VeryHard => 6500.0,
		Tier::Extreme => 8000.0,
		Tier::Death => 9500.0,
		Tier::Unfeasible | Tier::Impossible => {
			return None;
		}
	};

	if is_pro_leaderboard {
		points += (MAX_POINTS - points) * 0.1;
	}

	Some(points)
}

/// Calculates extra points awarded for placing a specific rank on a
/// leaderboard.
#[allow(dead_code, reason = "will be used later")]
pub fn points_for_rank(leaderboard_size: usize, rank: usize) -> f64
{
	let mut points = 0.5 * (1.0 - rank as f64 / leaderboard_size as f64);

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

/// Calculates points for a completion on a filter with few total completions.
///
/// "few total completions" is determined by [`LOW_COMPLETION_THRESHOLD`].
pub fn points_for_low_completion(tier: Tier, wr_time: f64, time: f64) -> f64
{
	let x = 2.1 - 0.25 * (u8::from(tier) as f64);
	let y = 1.0 + (x * -0.5).exp();
	let z = 1.0 + (x * (time / wr_time - 1.5)).exp();

	y / z
}

/// Calculates points for a record based on the overall distribution.
///
/// # Parameters
///
/// * `distribution` - the distribution for the leaderboard
/// * `scaled_times` - time values for the leaderboard scaled according to
///   `distribution`
/// * `dist_points_so_far` - list of previous results returned by this function
///
/// As this function is supposed to be called subsequently, the value returned
/// by it should be appended to `dist_points_so_far` before the next call.
pub fn calculate_dist_points(
	py: Python<'_>,
	distribution: &Distribution,
	scaled_times: &[f64],
	dist_points_so_far: &[f64],
	rank: usize,
) -> PyResult<f64>
{
	if rank == 0 {
		return Ok(distribution.top_scale);
	}

	// Time is the same as the previous rank, so they also get the same amount of
	// points.
	if scaled_times[rank] == scaled_times[rank - 1] {
		return Ok(dist_points_so_far[rank - 1]);
	}

	let pdf = py
		.import_bound("scipy.stats")?
		.getattr("norminvgauss")?
		.getattr("_pdf")?;

	let (diff, _) = py
		.import_bound("scipy")?
		.getattr("integrate")?
		.getattr("quad")?
		.call1((pdf, scaled_times[rank - 1], scaled_times[rank], (distribution.a, distribution.b)))?
		.downcast_into::<PyTuple>()?
		.extract::<(f64, f64)>()?;

	Ok(dist_points_so_far[rank - 1] - diff)
}

pub fn points_for_record(
	py: Python<'_>,
	distribution: Option<&Distribution>,
	leaderboard_size: usize,
	wr_time: f64,
	tier: Tier,
	time: f64,
	rank: usize,
	is_pro_leaderboard: bool,
) -> PyResult<Points>
{
	let minimum = minimum_points(tier, is_pro_leaderboard).expect("`tier` should be <= 8");
	let remaining = MAX_POINTS - minimum;
	let rank = 0.25 * remaining * points_for_rank(leaderboard_size, rank);
	let dist = 0.75
		* remaining
		* (match (distribution, leaderboard_size) {
			(None, _) | (Some(_), ..=LOW_COMPLETION_THRESHOLD) => {
				points_for_low_completion(tier, wr_time, time)
			}
			(Some(distribution), _) => distribution.sf(py, time)? / distribution.top_scale,
		});

	Ok(Points { minimum, rank, dist })
}
