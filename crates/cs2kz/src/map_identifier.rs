//! Different ways of identifying maps.

crate::identifier::identifier! {
	/// Different ways of identifying a map.
	enum MapIdentifier {
		/// An ID.
		ID(u16),

		/// A name.
		Name(String),
	}

	ParseError: ParseMapIdentifierError
}
