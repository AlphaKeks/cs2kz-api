mod database;
pub use database::DatabaseConfig;

mod replay_storage;
pub use replay_storage::ReplayStorageConfig;

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub database: DatabaseConfig,

    #[serde(default)]
    pub replay_storage: Option<ReplayStorageConfig>,
}
