/// Tracing configuration.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TracingConfig {
	/// Whether to enable tracing.
	#[serde(default = "default_enable")]
	pub enable: bool,

	/// Whether to include HTTP headers in request lifecycle traces.
	#[serde(default)]
	pub include_http_headers: bool,
}

impl Default for TracingConfig {
	fn default() -> Self {
		Self {
			enable: true,
			include_http_headers: false,
		}
	}
}

fn default_enable() -> bool {
	true
}
