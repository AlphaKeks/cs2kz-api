//! Reusable SQL queries for this module.

/// SQL query for fetching servers.
pub const SELECT: &str = r#"
	SELECT SQL_CALC_FOUND_ROWS
	  s.id,
	  s.name,
	  s.ip_address,
	  s.port,
	  o.name player_name,
	  o.steam_id player_id,
	  s.created_on
	FROM
	  Servers s
	  JOIN Players o ON o.id = s.owned_by
"#;
