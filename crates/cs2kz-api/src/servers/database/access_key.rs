use super::{get_by_access_key, Server};
use crate::database;
use crate::servers::{AccessKey, ServerID};

/// Invalidates a server's access key.
///
/// This function will return `false` if there is no server with the provided `server_id`, and
/// therefore no access key has actually been reset.
#[instrument(skip(conn), ret(level = "debug"), err(level = "debug"))]
pub async fn invalidate_access_key(
	conn: &mut database::Connection,
	server_id: ServerID,
) -> database::Result<bool> {
	sqlx::query!(
		"UPDATE Servers
		 SET access_key = ?
		 WHERE id = ?",
		AccessKey::invalid(),
		server_id,
	)
	.execute(conn)
	.await
	.map(|result| match result.rows_affected() {
		0 => false,
		1 => true,
		n => panic!("updated more than 1 server ({n})"),
	})
}

/// Resets a server's access key.
///
/// This function will return the newly generated access key.
#[instrument(skip(conn), ret(level = "debug"), err(level = "debug"))]
pub async fn reset_access_key(
	conn: &mut database::Connection,
	server_id: ServerID,
) -> database::Result<AccessKey> {
	let access_key = AccessKey::new();

	sqlx::query!(
		"UPDATE Servers
		 SET access_key = ?
		 WHERE id = ?",
		access_key,
		server_id,
	)
	.execute(conn)
	.await?;

	Ok(access_key)
}
