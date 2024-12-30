use std::fs::File;
use std::io::BufReader;
use std::iter;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{WrapErr, bail};
use cs2kz::Tier;
use pyo3::{PyResult, Python};
use serde::Deserialize;

use crate::points::{
	Distribution,
	LOW_COMPLETION_THRESHOLD,
	MAX_POINTS,
	Record,
	calculate_dist_points,
	minimum_points,
	points_for_low_completion,
	points_for_rank,
};

fn current_dir() -> PathBuf
{
	Path::new(file!())
		.parent()
		.map(ToOwned::to_owned)
		.expect("file should have parent directory")
}

fn load_data<T>(path: impl AsRef<Path>) -> color_eyre::Result<T>
where
	T: for<'de> Deserialize<'de>,
{
	File::open(path.as_ref())
		.map(BufReader::new)
		.map(serde_json::from_reader)
		.with_context(|| format!("open data file at `{:?}`", path.as_ref()))?
		.context("parse data")
}

fn calculate_nub(py: Python<'_>, records: &[Record], tier: Tier) -> PyResult<Vec<f64>>
{
	let distribution = Distribution::calculate(py, records)?;
	let scaled_times = distribution
		.scale(records.iter().map(|record| record.time))
		.collect::<Vec<_>>();
	let mut dist_points_so_far = Vec::with_capacity(records.len());
	let minimum_points = minimum_points(tier, false).unwrap();
	let remaining_points = MAX_POINTS - minimum_points;

	records
		.iter()
		.enumerate()
		.map(|(rank, record)| {
			let rank_points = 0.25 * remaining_points * points_for_rank(records.len(), rank);
			let dist_points = 0.75
				* remaining_points
				* (if records.len() <= LOW_COMPLETION_THRESHOLD {
					points_for_low_completion(tier, records[0].time, record.time)
				} else {
					calculate_dist_points(
						py,
						&distribution,
						&scaled_times,
						&dist_points_so_far,
						rank,
					)
					.inspect(|&points| dist_points_so_far.push(points))
					.map(|points| (points / distribution.top_scale).min(1.0))?
				});

			Ok(minimum_points + rank_points + dist_points)
		})
		.collect()
}

#[test]
fn nub() -> color_eyre::Result<()>
{
	let mut records =
		load_data::<Vec<Record>>(current_dir().join("test-data/dakow-nub-records.json"))
			.context("load nub records")?;

	records.sort_unstable();

	let expected_points =
		load_data::<Vec<f64>>(current_dir().join("test-data/dakow-nub-points.json"))
			.context("load nub points")?;

	let actual_points = Python::with_gil(|py| calculate_nub(py, &records, Tier::VeryEasy))?;
	let mut success = true;

	for (idx, (actual, expected)) in iter::zip(&actual_points, &expected_points)
		.map(|(&actual, &expected)| (actual.trunc(), expected))
		.enumerate()
	{
		if actual != expected {
			success = false;
			eprintln!("#{idx} - {actual} != {expected}");
		}
	}

	if !success {
		bail!("found differences");
	}

	Ok(())
}
