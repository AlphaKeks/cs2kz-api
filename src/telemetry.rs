use tracing_subscriber::{EnvFilter, fmt::time::UtcTime};

pub(crate) fn init()
{
	tracing_subscriber::fmt()
		.pretty()
		.with_file(true)
		.with_line_number(true)
		.with_timer(UtcTime::rfc_3339())
		.with_env_filter(EnvFilter::from_default_env())
		.init();
}
