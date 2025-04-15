use {
	cs2kz_api::{
		game::Game,
		maps::{FilterId, MapState},
		players::PlayerId,
		servers::ServerId,
		users::{Permission, UserId},
	},
	std::num::NonZero,
};

pub(crate) fn args() -> Args
{
	<Args as clap::Parser>::parse()
}

#[derive(Debug, clap::Parser)]
pub(crate) enum Args
{
	#[command(subcommand)]
	PluginVersions(PluginVersions),

	#[command(subcommand)]
	Users(Users),

	#[command(subcommand)]
	Servers(Servers),

	#[command(subcommand)]
	Maps(Maps),

	#[command(subcommand)]
	Players(Players),

	#[command(subcommand)]
	Records(Records),

	Permissions
	{
		user_id: UserId, permissions: Vec<Permission>
	},
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum PluginVersions
{
	Create
	{
		/// The game the plugin is for
		#[arg(value_enum, long)]
		game: Option<Game>,

		/// How many to create
		count: u64,
	},

	Delete
	{
		/// How many to delete
		count: u64,
	},
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Users
{
	Create
	{
		/// How many to create
		count: u64,
	},

	Delete
	{
		/// How many to delete
		count: u64,
	},
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Servers
{
	Create
	{
		/// The user owning the server
		#[arg(long)]
		owner: Option<UserId>,

		/// The game the server is running
		#[arg(value_enum, long)]
		game: Option<Game>,

		/// How many to create
		count: u64,
	},

	Delete
	{
		/// The user owning the servers to delete
		#[arg(long)]
		owner: Option<UserId>,

		/// How many to delete
		count: u64,
	},
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Maps
{
	Create
	{
		/// The user who created the map
		#[arg(long)]
		mapper: Option<UserId>,

		/// The state the map should be in
		#[arg(value_enum, long)]
		state: Option<MapState>,

		/// The number of courses to generate
		#[arg(long)]
		courses: Option<NonZero<u16>>,

		/// How many to create
		count: u64,
	},

	Delete
	{
		/// The user who created the maps to delete
		#[arg(long)]
		mapper: Option<UserId>,

		/// How many to delete
		count: u64,
	},
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Players
{
	Create
	{
		/// How many to create
		count: u64,
	},

	Delete
	{
		/// How many to delete
		count: u64,
	},
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Records
{
	Create
	{
		/// Which filter the record was set on
		#[arg(long)]
		filter: Option<FilterId>,

		/// Which player set the record
		#[arg(long)]
		player: Option<PlayerId>,

		/// Which server the record was set on
		#[arg(long)]
		server: Option<ServerId>,

		/// How many to create
		count: u64,
	},

	Delete
	{
		/// How many to delete
		count: u64,
	},
}
