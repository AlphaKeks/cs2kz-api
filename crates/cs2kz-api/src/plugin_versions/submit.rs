use crate::events::Event;
use crate::git::GitRevision;
use crate::plugin_versions::{PluginVersionID, PluginVersionName};
use crate::{database, events, plugin_versions};

#[instrument(skip(pool), fields(%name, %git_revision), ret(level = "debug"), err(level = "debug"))]
pub async fn submit(
	pool: &database::ConnectionPool,
	NewPluginVersion { name, git_revision }: NewPluginVersion,
) -> Result<PluginVersionID, SubmitPluginVersionError> {
	let mut txn = pool.begin().await?;
	let latest = plugin_versions::database::get_latest(&mut txn).await?;

	if let Some(latest) = latest.filter(|v| v.name > name).map(|v| v.name) {
		return Err(SubmitPluginVersionError::VersionOutdated { latest });
	}

	let id = plugin_versions::database::create(&mut txn, &name, &git_revision).await?;

	txn.commit().await?;
	events::dispatch(Event::PluginVersionSubmitted {
		id,
		name,
		git_revision,
	});

	Ok(id)
}

pub struct NewPluginVersion {
	pub name: PluginVersionName,
	pub git_revision: GitRevision,
}

#[derive(Debug, Error)]
pub enum SubmitPluginVersionError {
	#[error("this plugin version is older than the current latest version ({latest})")]
	VersionOutdated { latest: PluginVersionName },

	#[error("this plugin version has been submitted previously")]
	VersionAlreadyExists,

	#[error("database error: {0}")]
	Database(#[from] database::Error),
}

impl_error_from!(plugin_versions::database::CreatePluginVersionError => SubmitPluginVersionError => {
	E::DuplicateName | E::DuplicateGitRevision => Self::VersionAlreadyExists,
	E::Database(source) => Self::Database(source),
});
