pub use self::{
	description::{CourseDescription, InvalidCourseDescription},
	id::{CourseId, ParseCourseIdError},
	local_id::{CourseLocalId, ParseCourseLocalIdError},
	name::{CourseName, InvalidCourseName},
};

mod description;
mod id;
mod local_id;
mod name;
