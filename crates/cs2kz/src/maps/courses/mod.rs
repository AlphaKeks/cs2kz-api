use std::num::NonZero;

pub mod filters;

define_id_type! {
	/// A unique identifier for CS2KZ map courses.
	#[derive(sqlx::Type)]
	#[sqlx(transparent)]
	pub struct CourseId(NonZero<u16>);
}

#[derive(Debug, serde::Serialize)]
pub struct CourseInfo {
	pub id: CourseId,
	pub name: String,
}
