//! Different ways of identifying players.

use crate::SteamID;

crate::identifier::identifier! {
	/// Different ways of identifying a player.
	enum PlayerIdentifier {
		/// A [SteamID].
		SteamID(SteamID),

		/// A player name.
		Name(String),
	}

	ParseError: ParsePlayerIdentifierError
}
