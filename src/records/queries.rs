//! Shared SQL queries.

/// SQL query for `SELECT`ing records from the database.
pub static SELECT: &str = r#"
	SELECT SQL_CALC_FOUND_ROWS
	  r.id,
	  f.mode_id mode,
	  r.styles,
	  r.teleports,
	  r.time,
	  p.name player_name,
	  p.id player_id,
	  m.id map_id,
	  m.name map_name,
	  c.id course_id,
	  c.name course_name,
	  f.tier course_tier,
	  s.name server_name,
	  s.id server_id,
	  r.bhops,
	  r.perfs,
	  r.created_on
	FROM
	  Records r
	  JOIN CourseFilters f ON f.id = r.filter_id
	  JOIN Players p ON p.id = r.player_id
	  JOIN Courses c ON c.id = f.course_id
	  JOIN Maps m ON m.id = c.map_id
	  JOIN Servers s ON s.id = r.server_id
"#;
