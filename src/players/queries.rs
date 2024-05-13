//! Reusable SQL queries for this module.

/// Base query for `SELECT`ing players from the database.
pub const SELECT: &str = r#"
	SELECT
	  SQL_CALC_FOUND_ROWS p.id player_id p.name player_name
	FROM
	  Players p
"#;
