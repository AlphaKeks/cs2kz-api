//! Different ways of identifying map courses.

crate::identifier::identifier! {
	/// Different ways of identifying a map course.
	enum CourseIdentifier {
		/// An ID.
		ID(u16),

		/// A name.
		Name(String),
	}

	ParseError: ParseCourseIdentifierError
}
