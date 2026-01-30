#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ReplayStorageConfig {
    name: String,
    #[serde(deserialize_with = "deserialize_region")]
    region: s3::Region,
    access_key: String,
    secret_key: String,
    security_token: String,
}

impl ReplayStorageConfig {
    pub fn bucket(&self) -> s3::Bucket {
        let credentials = s3::creds::Credentials {
            access_key: Some(self.access_key.clone()),
            secret_key: Some(self.secret_key.clone()),
            security_token: Some(self.security_token.clone()),
            session_token: None,
            expiration: None,
        };

        *s3::Bucket::new(&self.name, self.region.clone(), credentials)
            .expect("failed to initialize replay storage bucket")
    }
}

fn deserialize_region<'de, D>(deserializer: D) -> Result<s3::Region, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let region = <String as serde::Deserialize<'de>>::deserialize(deserializer)?;
    let region = region
        .parse::<s3::Region>()
        .map_err(serde::de::Error::custom)?;

    Ok(region)
}
