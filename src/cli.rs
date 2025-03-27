use std::{net::IpAddr, path::Path};

pub(crate) fn args() -> Args
{
	<Args as clap::Parser>::parse()
}

#[derive(Debug, clap::Parser)]
pub(crate) enum Args
{
	/// Run the HTTP server
	#[clap(name = "serve")]
	Serve
	{
		/// Path to the API's configuration file
		#[arg(long = "config", default_value = "/etc/cs2kz-api.toml")]
		config_path: Box<Path>,

		/// The environment the API is running in
		#[arg(value_enum, long = "env")]
		environment: Option<crate::runtime::Environment>,

		/// Path to the `DepotDownloader` executable
		#[arg(long)]
		depot_downloader_path: Option<Box<Path>>,

		/// The IP address the server should listen on
		#[arg(long = "ip")]
		ip_addr: Option<IpAddr>,

		/// The port the server should listen on
		#[arg(long)]
		port: Option<u16>,
	},

	/// Print the API's OpenAPI schema
	#[clap(name = "generate-openapi-schema")]
	GenerateOpenApiSchema,
}
