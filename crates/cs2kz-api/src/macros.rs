macro_rules! with_database_error {
	(
		$(#[$meta:meta])*
		$vis:vis enum $name:ident {
			$($variants:tt)*
		}
	) => {
		$(#[$meta])*
		$vis enum $name {
			/// The database returned an error.
			#[cfg_attr(feature = "production", error("something went wrong; please report this incident"))]
			#[cfg_attr(not(feature = "production"), error("database error: {0}"))]
			Database(#[from] $crate::database::DatabaseError),

			$($variants)*
		}
	};
}
