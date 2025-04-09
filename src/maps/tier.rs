use {
	serde::{Deserialize, Serialize},
	utoipa::ToSchema,
};

#[repr(u8)]
#[derive(
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Serialize,
	Deserialize,
	sqlx::Type,
	ToSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum Tier
{
	/// An average CS2 player who has never touched KZ before is able to
	/// complete the course
	VeryEasy = 1,

	/// A new KZ player who has just learned movement basics like air strafing
	/// and bunny hopping is able to complete the course
	Easy = 2,

	/// More difficult than T2 but without introducing new concepts
	Medium = 3,

	/// More advanced concepts like surf and ladders but still on a basic level
	Advanced = 4,

	/// Even more advanced concepts and more niche mechanics like box-tech,
	/// tightly timed sections, wall strafing, prekeep, etc.
	Hard = 5,

	/// More difficult than T5 but without introducing new concepts
	VeryHard = 6,

	/// Features the most advanced mechanics
	Extreme = 7,

	/// T7 but harder
	Death = 8,

	/// Special tier for courses that are *technically* possible, but unlikely
	/// to be done by humans without TAS (Tool-Assisted Speedrun) tools
	Unfeasible = 9,

	/// Special tier for courses that are *literally* impossible; even with
	/// perfect inputs (generally reserved for VNL filters on courses intended
	/// for CKZ)
	Impossible = 10,
}

impl Tier
{
	pub const fn is_humanly_possible(&self) -> bool
	{
		(*self as u8) <= (Self::Death as u8)
	}
}

impl_rand!(Tier => |rng| match rng.random_range(1..=10) {
	1 => Tier::VeryEasy,
	2 => Tier::Easy,
	3 => Tier::Medium,
	4 => Tier::Advanced,
	5 => Tier::Hard,
	6 => Tier::VeryHard,
	7 => Tier::Extreme,
	8 => Tier::Death,
	9 => Tier::Unfeasible,
	10 => Tier::Impossible,
	_ => unreachable!(),
});
