//! Different ways of identifying servers.

crate::identifier::identifier! {
	/// Different ways of identifying a server.
	enum ServerIdentifier {
		/// An ID.
		ID(u16),

		/// A name.
		Name(String),
	}

	ParseError: ParseServerIdentifierError
}
