//! Worker-Thread management for CPU-intensive calculations.

use std::ops::ControlFlow;
use std::sync::mpsc;
use std::time::Duration;
use std::{cmp, thread};

use cs2kz::Tier;
use pyo3::{PyResult, Python};
use tokio::sync::oneshot;
use tokio::sync::oneshot::error::TryRecvError as OneshotRecvError;

use crate::points::{
	Distribution,
	LOW_COMPLETION_THRESHOLD,
	Record,
	calculate_dist_points,
	points_for_low_completion,
};

/// Timeout for waiting for a new job before checking the shutdown signal.
const JOB_WAIT_TIMEOUT: Duration = Duration::from_millis(500);

/// A handle to a worker thread.
pub struct Worker
{
	/// Job Queue.
	job_tx: mpsc::Sender<Job>,

	/// Shutdown signal to kill the worker thread.
	///
	/// This is an `Option` because we need to take ownership in `Drop::drop()`.
	///
	/// The outer `oneshot::Sender` is for telling the worker thread to
	/// shutdown, the inner `oneshot::Sender` is an ACK signal sent back by the
	/// worker thread right before shutting down. We need this so we can wait
	/// for the thread to actually exit, in case it's still currently
	/// processing a job.
	shutdown_tx: Option<oneshot::Sender<oneshot::Sender<()>>>,
}

/// A job queued for execution.
enum Job
{
	/// Calculate a distribution for the given `records`.
	Distribution
	{
		/// Input data for calculating the distribution.
		records: Vec<Record>,

		/// Output distribution values and the original input.
		response_tx: oneshot::Sender<PyResult<(Vec<Record>, Distribution)>>,
	},

	/// Calculate distribution-portion of the points for a set of NUB & PRO
	/// filters.
	DistPoints
	{
		/// NUB leaderboard to calculate points for.
		nub_records: Vec<Record>,

		/// The distribution for `nub_records`.
		nub_distribution: Distribution,

		/// The tier of the NUB filter.
		nub_tier: Tier,

		/// PRO leaderboard to calculate points for.
		pro_records: Vec<Record>,

		/// The distribution for `pro_records`.
		pro_distribution: Distribution,

		/// The tier of the PRO filter.
		pro_tier: Tier,

		/// The results - `(nub_records, nub_points, pro_records, pro_points)`.
		#[allow(clippy::type_complexity)]
		response_tx: oneshot::Sender<PyResult<(Vec<Record>, Vec<f64>, Vec<Record>, Vec<f64>)>>,
	},
}

impl Worker
{
	/// Spawns a new worker thread and returns a handle to it.
	pub fn spawn() -> Self
	{
		let (job_tx, mut job_rx) = mpsc::channel::<Job>();
		let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<oneshot::Sender<()>>();

		thread::spawn(move || {
			Python::with_gil(|py| {
				loop {
					match run(py, &mut job_rx, &mut shutdown_rx) {
						ControlFlow::Break(()) => break,
						ControlFlow::Continue(()) => continue,
					}
				}
			})
		});

		Self { job_tx, shutdown_tx: Some(shutdown_tx) }
	}

	/// Calculates the distribution data for the given `records`.
	pub async fn calculate_distribution(
		&mut self,
		records: Vec<Record>,
	) -> PyResult<(Vec<Record>, Distribution)>
	{
		let (response_tx, response_rx) = oneshot::channel();
		let job = Job::Distribution { records, response_tx };

		if self.job_tx.send(job).is_err() {
			unreachable!("worker thread has shutdown prematurely");
		}

		match response_rx.await {
			Ok(Ok((records, distribution))) => Ok((records, distribution)),
			Ok(Err(error)) => Err(error),
			Err(_) => unreachable!("worker thread dropped job response_tx"),
		}
	}

	/// Calculate the distribution-portion of points for a set of NUB & PRO
	/// leaderboards.
	pub async fn calculate_dist_points(
		&mut self,
		nub_records: Vec<Record>,
		nub_distribution: Distribution,
		nub_tier: Tier,
		pro_records: Vec<Record>,
		pro_distribution: Distribution,
		pro_tier: Tier,
	) -> PyResult<(Vec<Record>, Vec<f64>, Vec<Record>, Vec<f64>)>
	{
		let (response_tx, response_rx) = oneshot::channel();
		let job = Job::DistPoints {
			nub_records,
			nub_distribution,
			nub_tier,
			pro_records,
			pro_distribution,
			pro_tier,
			response_tx,
		};

		if self.job_tx.send(job).is_err() {
			unreachable!("worker thread has shutdown prematurely");
		}

		match response_rx.await {
			Ok(Ok(results)) => Ok(results),
			Ok(Err(error)) => Err(error),
			Err(_) => unreachable!("worker thread dropped job response_tx"),
		}
	}

	/// Shuts down the worker thread, waiting for it to exit.
	pub async fn shutdown(mut self)
	{
		let Some(shutdown_tx) = self.shutdown_tx.take() else {
			// We only ever `.take()` the `shutdown_tx` in this method and in
			// `Drop::drop()`, which can't be called manually.
			unreachable!();
		};

		let (ack_tx, ack_rx) = oneshot::channel::<()>();

		if shutdown_tx.send(ack_tx).is_err() {
			unreachable!("worker thread dropped the shutdown signal");
		}

		if ack_rx.await.is_err() {
			unreachable!("worker thread dropped the shutdown ack_tx");
		}
	}
}

impl Drop for Worker
{
	/// Shuts down the worker thread without waiting for it to exit.
	fn drop(&mut self)
	{
		if let Some(shutdown_tx) = self.shutdown_tx.take() {
			let (ack_tx, _ack_rx) = oneshot::channel::<()>();
			drop(shutdown_tx.send(ack_tx));
		}
	}
}

/// The worker thread's main function.
fn run(
	py: Python<'_>,
	job_rx: &mut mpsc::Receiver<Job>,
	shutdown_rx: &mut oneshot::Receiver<oneshot::Sender<()>>,
) -> ControlFlow<()>
{
	// wait for a new job for a bit
	match job_rx.recv_timeout(JOB_WAIT_TIMEOUT) {
		// if we got something, run it!
		Ok(Job::Distribution { records, response_tx }) => {
			let distribution = Distribution::calculate(py, &records);

			if response_tx
				.send(distribution.map(|distribution| (records, distribution)))
				.is_err()
			{
				unreachable!("job response_rx is never dropped");
			}

			ControlFlow::Continue(())
		}
		Ok(Job::DistPoints {
			nub_records,
			nub_distribution,
			nub_tier,
			pro_records,
			pro_distribution,
			pro_tier,
			response_tx,
		}) => {
			let mut nub_dist_points_so_far = Vec::with_capacity(nub_records.len());
			let scaled_nub_times = nub_distribution
				.scale(nub_records.iter().map(|record| record.time))
				.collect::<Vec<_>>();

			let nub_points = nub_records
				.iter()
				.enumerate()
				.map(|(rank, record)| {
					if nub_records.len() <= LOW_COMPLETION_THRESHOLD {
						Ok(points_for_low_completion(nub_tier, nub_records[0].time, record.time))
					} else {
						calculate_dist_points(
							py,
							&nub_distribution,
							&scaled_nub_times,
							&nub_dist_points_so_far,
							rank,
						)
						.inspect(|&points| nub_dist_points_so_far.push(points))
						.map(|points| (points / nub_distribution.top_scale).min(1.0))
					}
				})
				.collect::<PyResult<Vec<_>>>();

			let mut pro_dist_points_so_far = Vec::with_capacity(pro_records.len());
			let scaled_pro_times = pro_distribution
				.scale(pro_records.iter().map(|record| record.time))
				.collect::<Vec<_>>();

			let pro_points = pro_records
				.iter()
				.enumerate()
				.map(|(rank, record)| {
					let pro_points = if pro_records.len() <= LOW_COMPLETION_THRESHOLD {
						Ok(points_for_low_completion(pro_tier, pro_records[0].time, record.time))
					} else {
						calculate_dist_points(
							py,
							&pro_distribution,
							&scaled_pro_times,
							&pro_dist_points_so_far,
							rank,
						)
						.inspect(|&points| pro_dist_points_so_far.push(points))
						.map(|points| (points / pro_distribution.top_scale).min(1.0))
					}?;

					// figure out which rank this record would be, if it was in
					// the NUB leaderboard instead
					let (Ok(rank) | Err(rank)) =
						nub_records.binary_search_by(|record| record.time.total_cmp(&record.time));

					let nub_points = calculate_dist_points(
						py,
						&nub_distribution,
						&scaled_pro_times,
						&nub_dist_points_so_far,
						rank,
					)?;

					Ok(cmp::max_by(nub_points, pro_points, f64::total_cmp))
				})
				.collect::<PyResult<Vec<_>>>();

			let results = match (nub_points, pro_points) {
				(Err(error), _) | (_, Err(error)) => Err(error),
				(Ok(nub_points), Ok(pro_points)) => {
					Ok((nub_records, nub_points, pro_records, pro_points))
				}
			};

			if response_tx.send(results).is_err() {
				unreachable!("job response_rx is never dropped");
			}

			ControlFlow::Continue(())
		}

		// otherwise, check if we were asked to shutdown
		Err(mpsc::RecvTimeoutError::Timeout) => match shutdown_rx.try_recv() {
			// yep -> shutdown
			Ok(ack_tx) => {
				let _ = ack_tx.send(());
				ControlFlow::Break(())
			}

			// nope -> wait for a new job again
			Err(OneshotRecvError::Empty) => ControlFlow::Continue(()),

			// won't happen in practice
			Err(OneshotRecvError::Closed) => {
				unreachable!("shutdown signals are never dropped");
			}
		},

		// won't happen in practice
		Err(mpsc::RecvTimeoutError::Disconnected) => {
			unreachable!("tx is never dropped");
		}
	}
}
