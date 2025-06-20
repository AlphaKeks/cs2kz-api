//! Record point calculations
//!
//! This module contains the core types and functions used to calculate the
//! final points value for any given record. The [`PointsDaemon`] can be run in
//! a background task to continually re-calculate points for leaderboards.
//!
//! ## Constraints
//!
//! - The maximum number of points any record can have is [`Points::MAX`].
//! - The final points value is based on the following factors:
//!    - [`Tier`] - completing difficult courses is a feat of its own and should
//!      be rewarded
//!    - placing "objectively" high (top 100) on the leaderboard
//!    - placing high on the leaderboard compared to other players
//!
//! ## Implementation
//!
//! These 3 factors are represented by the [`TierPortion`], [`RankPortion`], and
//! [`LeaderboardPortion`] types, respectively.
//!
//! ### Tier Portion
//!
//! The value of [`TierPortion`] is a baseline value between 0 and 9550 that is
//! based on both the [`Tier`] of the course as well as the [`Leaderboard`] the
//! record will be placed on. This is because completing a course is generally
//! more difficult without using checkpoints, and as such should be rewarded
//! more.
//!
//! See the implementation of [`TierPortion::new()`] for details.
//!
//! ### Rank Portion
//!
//! The value of [`RankPortion`] is a modifier that is applied during the final
//! calculation and is based solely on the [`Rank`] of the run on its designated
//! leaderboard.
//!
//! See the implementation of [`RankPortion::new()`] for details.
//!
//! ### Leaderboard Portion
//!
//! The value of [`LeaderboardPortion`] is a modifier that is applied during the
//! final calculation and is based on the relative performance of the run
//! compared to all others. To make this as accurate as possible, [small]
//! leaderboards get an estimation based on
//! [`LeaderboardPortion::for_small_leaderboard()`], whereas everything else is
//! calculated using a [Normal Inverse Gaussian Distribution][norminvgauss]. The
//! parameters for the distribution are represented by the [`Distribution`] type
//! and passed into any functions that require it. For how these parameters are
//! determined, see [Points Daemon].
//!
//! ## Points Daemon
//!
//! The [`PointsDaemon`] type represents a background task that continually
//! re-calculates both the [`Distribution`] and points for all leaderboards.
//!
//! [`Rank`]: crate::records::Rank
//! [small]: SMALL_LEADERBOARD_THRESHOLD
//! [Points Daemon]: self#points-daemon
//! [norminvgauss]: https://en.wikipedia.org/wiki/Normal-inverse_Gaussian_distribution

pub use self::{
	daemon::{PointsDaemon, PointsDaemonError, PointsDaemonHandle},
	distribution::{Distribution, DistributionError},
};
use {
	crate::{
		maps::Tier,
		python::{self, PyState, PythonError},
		records::{self, Leaderboard, Points},
	},
	pyo3::{
		PyResult,
		types::{PyAnyMethods, PyTuple},
	},
	std::error::Error,
};

mod daemon;
mod distribution;

/// Threshold for what constitutes a "small" leaderboard.
pub const SMALL_LEADERBOARD_THRESHOLD: usize = 50;

/// The base line points for completing a course of a given [`Tier`]
///
/// See the [module-level documentation] for more information.
///
/// [module-level documentation]: crate::points#tier-portion
#[derive(Debug, Default, Clone, Copy)]
#[debug("{_0:?}")]
pub struct TierPortion(f64);

/// A modifier based on a record's [`Rank`]
///
/// See the [module-level documentation] for more information.
///
/// [`Rank`]: crate::records::Rank
/// [module-level documentation]: crate::points#rank-portion
#[derive(Debug, Default, Clone, Copy)]
#[debug("{_0:?}")]
pub struct RankPortion(f64);

/// A modifier based on a record's time relative to the rest of the leaderboard
///
/// See the [module-level documentation] for more information.
///
/// [module-level documentation]: crate::points#leaderboard-portion
#[derive(Debug, Default, Clone, Copy, sqlx::Type)]
#[debug("{_0:?}")]
#[sqlx(transparent)]
pub struct LeaderboardPortion(f64);

#[derive(Debug, Display, Error, From)]
#[display("failed to calculate leaderboard portion for new record: {_variant}")]
pub enum CalculateLeaderboardPortionForNewRecordError
{
	#[display("survival function returned NaN")]
	NaN,

	#[display("{_0}")]
	PythonError(PythonError),

	#[display("{_0}")]
	CalculateDistribution(DistributionError),
}

/// Calculates the [`Points`] a record should be awarded.
#[track_caller]
pub fn calculate(
	TierPortion(tier_portion): TierPortion,
	RankPortion(rank_portion): RankPortion,
	LeaderboardPortion(leaderboard_portion): LeaderboardPortion,
) -> Points
{
	let remaining = Points::MAX.as_f64() - tier_portion;
	let rank_portion = 0.125_f64 * remaining * rank_portion;
	let leaderboard_portion = 0.875_f64 * remaining * leaderboard_portion;
	let result = tier_portion + rank_portion + leaderboard_portion;

	Points::try_from(result).unwrap_or_else(|err| {
		error!(error = &err as &dyn Error, result, tier_portion, rank_portion, leaderboard_portion);
		panic!("constructed invalid points value");
	})
}

impl TierPortion
{
	/// Returns the minimum number of points to award for a completion on
	/// a filter with the given `tier`.
	///
	/// # Panics
	///
	/// This function will panic if <code>tier > [Tier::Death]</code>.
	#[track_caller]
	pub fn new(tier: Tier, leaderboard: Leaderboard) -> Self
	{
		Self(match (tier, leaderboard) {
			(Tier::VeryEasy, Leaderboard::NUB) => 0.0,
			(Tier::VeryEasy, Leaderboard::PRO) => 1000.0,
			(Tier::Easy, Leaderboard::NUB) => 500.0,
			(Tier::Easy, Leaderboard::PRO) => 1450.0,
			(Tier::Medium, Leaderboard::NUB) => 2000.0,
			(Tier::Medium, Leaderboard::PRO) => 2800.0,
			(Tier::Advanced, Leaderboard::NUB) => 3500.0,
			(Tier::Advanced, Leaderboard::PRO) => 4150.0,
			(Tier::Hard, Leaderboard::NUB) => 5000.0,
			(Tier::Hard, Leaderboard::PRO) => 5500.0,
			(Tier::VeryHard, Leaderboard::NUB) => 6500.0,
			(Tier::VeryHard, Leaderboard::PRO) => 6850.0,
			(Tier::Extreme, Leaderboard::NUB) => 8000.0,
			(Tier::Extreme, Leaderboard::PRO) => 8200.0,
			(Tier::Death, Leaderboard::NUB) => 9500.0,
			(Tier::Death, Leaderboard::PRO) => 9550.0,
			(Tier::Unfeasible, Leaderboard::NUB)
			| (Tier::Unfeasible, Leaderboard::PRO)
			| (Tier::Impossible, Leaderboard::NUB)
			| (Tier::Impossible, Leaderboard::PRO) => {
				panic!("passed invalid tier {tier:?} to `Points::for_tier()`");
			},
		})
	}
}

impl RankPortion
{
	/// Returns a modifier used to calculate [`Points`] based on a record's
	/// leaderboard placement.
	pub fn new(records::Rank(rank): records::Rank) -> Self
	{
		let mut value = 0.0_f64;

		#[allow(clippy::cast_precision_loss, reason = "`100 - rank` will always fit into f64")]
		if rank < 100 {
			value += ((100 - rank) as f64) * 0.004_f64;
		}

		#[allow(clippy::cast_precision_loss, reason = "`20 - rank` will always fit into f64")]
		if rank < 20 {
			value += ((20 - rank) as f64) * 0.02_f64;
		}

		if let Some(&extra) = [0.2_f64, 0.12_f64, 0.09_f64, 0.06_f64, 0.02_f64].get(rank) {
			value += extra;
		}

		Self(value)
	}
}

#[bon::bon]
impl LeaderboardPortion
{
	pub const fn as_f64(self) -> f64
	{
		self.0
	}

	/// Calculates the amount of points to award for perfoming relative to
	/// everyone else on the leaderboard.
	///
	/// ## Invariants
	///
	/// - This function should only be called if the leaderboard has more than
	///   [`SMALL_LEADERBOARD_THRESHOLD`] entries.
	#[instrument(level = "debug", ret(level = "debug"), err)]
	pub async fn from_distribution(
		distribution: Distribution,
		time: records::Time,
	) -> Result<Self, CalculateLeaderboardPortionForNewRecordError>
	{
		let span = tracing::Span::current();

		python::execute(move |py_state| {
			let _guard = span.enter();
			let sf = distribution.sf(py_state, time.as_f64())?;

			if sf.is_nan() {
				#[expect(clippy::manual_assert)]
				if cfg!(debug_assertions) {
					panic!("sf returned NaN (time={time}, distribution={distribution:?})");
				}

				warn!(?distribution, %time, "sf returned NaN");
				return Err(CalculateLeaderboardPortionForNewRecordError::NaN);
			}

			Ok(Self(sf / distribution.top_scale.as_f64()))
		})
		.await?
	}

	/// Returns a modifier used to calculate [`Points`] based on a record's
	/// leaderboard placement.
	///
	/// ## Invariants
	///
	/// - This function should only be called if the leaderboard has at most
	///   [`SMALL_LEADERBOARD_THRESHOLD`] entries.
	#[track_caller]
	pub fn for_small_leaderboard(tier: Tier, top_time: records::Time, time: records::Time) -> Self
	{
		debug_assert!(tier.is_humanly_possible());
		debug_assert!(top_time <= time);

		// no idea what any of this means; consult zer0.k

		let x = 2.1_f64 - 0.25_f64 * f64::from(tier as u8);
		let y = 1.0_f64 + (x * -0.5_f64).exp();
		let z = 1.0_f64 + (x * (time.as_f64() / top_time.as_f64() - 1.5_f64)).exp();

		Self(y / z)
	}

	#[instrument(level = "trace", skip(py_state), ret(level = "trace"), err)]
	#[builder(finish_fn = calculate)]
	fn incremental(
		#[builder(start_fn)] distribution: &Distribution,
		#[builder(finish_fn)] py_state: &PyState<'_>,
		results_so_far: &[Self],
		scaled_times: &[f64],
		rank: records::Rank,
	) -> PyResult<Self>
	{
		debug_assert_eq!(rank.0, results_so_far.len());

		let Some(previous_time) = rank.0.checked_sub(1).map(|idx| scaled_times[idx]) else {
			// we already calculated this
			return Ok(distribution.top_scale);
		};

		let current_time = scaled_times[rank.0];

		// `rank` and `rank - 1` are tied, so just award the same points
		if current_time == previous_time {
			return Ok(results_so_far[rank.0 - 1]);
		}

		let quad = py_state
			.python()
			.import("scipy")?
			.getattr("integrate")
			.inspect_err(|error| error!(%error, "failed to import scipy.integrate"))?
			.getattr("quad")
			.inspect_err(|error| error!(%error, "failed to import quad"))?;

		let pdf = py_state
			.python()
			.import("scipy.stats")
			.inspect_err(|error| error!(%error, "failed to import scipy.stats"))?
			.getattr("norminvgauss")
			.inspect_err(|error| error!(%error, "failed to import norminvgauss"))?
			.getattr("_pdf")
			.inspect_err(|error| error!(%error, "failed to get pdf"))?;

		let (difference, _) = quad
			.call1((pdf, previous_time, current_time, (distribution.a, distribution.b)))
			.inspect_err(|error| {
				error!(
					%error,
					?previous_time,
					?current_time,
					?distribution,
					"failed to call pdf",
				)
			})?
			.downcast_into::<PyTuple>()
			.inspect_err(|error| error!(%error, "pdf result is not a tuple"))?
			.extract::<(f64, f64)>()
			.inspect_err(|error| error!(%error, "pdf result is not a tuple of 2 floats"))?;

		Ok(Self(results_so_far[rank.0 - 1].0 - difference))
	}
}
