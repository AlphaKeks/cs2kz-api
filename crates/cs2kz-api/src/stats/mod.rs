//! This modules contains types related to statistics.

mod bhop_stats;
pub use bhop_stats::BhopStats;

mod game_sessions;
pub use game_sessions::{
	CourseSession,
	CourseSessionData,
	CourseSessionID,
	GameSession,
	GameSessionID,
};
