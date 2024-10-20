use std::net::IpAddr;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

pub fn args() -> Args {
	Args::parse()
}

#[derive(Debug, Parser)]
pub struct Args {
	#[command(subcommand)]
	pub action: Action,
}

#[derive(Debug, Subcommand)]
pub enum Action {
	Serve {
		#[arg(long)]
		config: Option<PathBuf>,

		#[arg(long = "ip")]
		ip_addr: Option<IpAddr>,

		#[arg(long)]
		port: Option<u16>,
	},
}
