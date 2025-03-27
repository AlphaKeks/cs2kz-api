#![feature(let_chains)]
#![feature(non_exhaustive_omitted_patterns_lint)]
#![feature(unqualified_local_imports)]

mod cli;

use std::{
	cmp,
	collections::hash_map::{self, HashMap},
	env,
	net::{IpAddr, Ipv4Addr},
	num::NonZero,
	time::Duration,
};

use color_eyre::{
	Section,
	eyre::{self, OptionExt, WrapErr, eyre},
};
use cs2kz_api::{
	database::{self, Database, DatabaseConnection},
	game::Game,
	maps::{
		self,
		CourseDescription,
		CourseLocalId,
		CourseName,
		FilterId,
		FilterNotes,
		MapDescription,
		MapName,
		MapState,
		NewCS2Filters,
		NewCSGOFilters,
		NewCourse,
		NewFilter,
		NewFilters,
		Tier,
	},
	mode::Mode,
	players::{self, PlayerId, PlayerName},
	plugin::{self, PluginVersion},
	records::{self, CreatedRecord, Teleports, Time},
	servers::{self, CreatedServer, ServerHost, ServerId, ServerName, ServerPort, ServerSessionId},
	stream::TryStreamExt as _,
	styles::{Style, Styles},
	time::{DurationExt, Seconds},
	users::{self, UserId, Username},
};
use fake::Fake;
use futures_util::{TryFutureExt, TryStreamExt as _};
use rand::{Rng, rngs::ThreadRng, seq::IndexedRandom};
use tracing_subscriber::EnvFilter;
use url::Url;

#[tokio::main]
async fn main() -> eyre::Result<()>
{
	color_eyre::install()?;
	tracing_subscriber::fmt()
		.compact()
		.with_env_filter(EnvFilter::from_default_env())
		.init();

	let args = cli::args();

	let database_url = env::var("DATABASE_URL").map_err(|err| match err {
		env::VarError::NotPresent => eyre!("`DATABASE_URL` is not set").suggestion({
			"set the `DATABASE_URL` environment variable to match the following schema: \
			 `mysql://<user>:<password>@<host>:<port>/<database>`"
		}),
		env::VarError::NotUnicode(raw_string) => {
			eyre!("{raw_string:?} is not a valid `DATABASE_URL`")
		},
	})?;

	let database_url = database_url.parse::<Url>().wrap_err("failed to parse `DATABASE_URL`")?;
	let database =
		Database::connect(database::ConnectOptions::builder().url(&database_url).build())
			.await
			.wrap_err("failed to connect to database")?;

	let mut rng = ThreadRng::default();

	database
		.in_transaction(async |conn| match args {
			cli::Args::PluginVersions(cli::PluginVersions::Create { game, count }) => {
				create_plugin_versions(conn, &mut rng, game, count).await
			},
			cli::Args::PluginVersions(cli::PluginVersions::Delete { count }) => {
				delete_plugin_versions(conn, count).await
			},
			cli::Args::Users(cli::Users::Create { count }) => {
				create_users(conn, &mut rng, count).await
			},
			cli::Args::Users(cli::Users::Delete { count }) => delete_users(conn, count).await,
			cli::Args::Servers(cli::Servers::Create { owner, game, count }) => {
				create_servers(conn, &mut rng, owner, game, count).await
			},
			cli::Args::Servers(cli::Servers::Delete { owner, count }) => {
				delete_servers(conn, owner, count).await
			},
			cli::Args::Maps(cli::Maps::Create { mapper, state, courses, count }) => {
				create_maps(conn, &mut rng, mapper, state, courses, count).await
			},
			cli::Args::Maps(cli::Maps::Delete { mapper, count }) => {
				delete_maps(conn, mapper, count).await
			},
			cli::Args::Players(cli::Players::Create { count }) => {
				create_players(conn, &mut rng, count).await
			},
			cli::Args::Players(cli::Players::Delete { count }) => delete_players(conn, count).await,
			cli::Args::Records(cli::Records::Create { filter, player, server, count }) => {
				create_records(conn, &mut rng, filter, player, server, count).await
			},
			cli::Args::Records(cli::Records::Delete { count }) => delete_records(conn, count).await,
		})
		.await
}

async fn create_plugin_versions(
	conn: &mut DatabaseConnection<'_, '_>,
	rng: &mut ThreadRng,
	game: Option<Game>,
	count: u64,
) -> eyre::Result<()>
{
	let latest_cs2 = plugin::get_latest_version(Game::CS2)
		.exec(&mut *conn)
		.await
		.wrap_err("failed to get latest plugin version for cs2")?;

	let latest_csgo = plugin::get_latest_version(Game::CSGO)
		.exec(&mut *conn)
		.await
		.wrap_err("failed to get latest plugin version for csgo")?;

	let mut latest = match (latest_cs2, latest_csgo) {
		(None, None) => PluginVersion::ZERO,
		(None, Some(csgo)) => csgo,
		(Some(cs2), None) => cs2,
		(Some(cs2), Some(csgo)) => cmp::max(cs2, csgo),
	};

	for _ in 0..count {
		latest = PluginVersion::from_parts(
			latest.major() + 1,
			latest.minor(),
			latest.patch(),
			latest.pre(),
			latest.build(),
		);

		let game = game.unwrap_or_else(|| rng.random());

		let modes = match game {
			Game::CS2 => &[Mode::Vanilla, Mode::Classic][..],
			Game::CSGO => &[Mode::KZTimer, Mode::SimpleKZ, Mode::VanillaCSGO][..],
		}
		.iter()
		.map(|&mode| (mode, rng.random::<plugin::Checksums>()))
		.collect::<Vec<_>>();

		let styles = match game {
			Game::CS2 => &[Style::AutoBhop][..],
			Game::CSGO => &[][..],
		}
		.iter()
		.map(|&style| (style, rng.random::<plugin::Checksums>()))
		.collect::<Vec<_>>();

		plugin::create_version(latest.clone(), game)
			.git_revision(rng.random())
			.linux_checksum(rng.random())
			.windows_checksum(rng.random())
			.is_cutoff(rng.random_range(0..100) < 10_u8)
			.modes(modes)
			.styles(styles)
			.exec(&mut *conn)
			.await
			.wrap_err("failed to create plugin version")?;

		tracing::info!(version = %latest, "created plugin version");
	}

	Ok(())
}

async fn delete_plugin_versions(
	conn: &mut DatabaseConnection<'_, '_>,
	count: u64,
) -> eyre::Result<()>
{
	plugin::delete_versions(count)
		.exec(conn)
		.map_ok(|amount| tracing::info!(amount, "deleted plugin versions"))
		.await
		.wrap_err("failed to delete plugin versions")
}

async fn create_users(
	conn: &mut DatabaseConnection<'_, '_>,
	rng: &mut ThreadRng,
	count: u64,
) -> eyre::Result<()>
{
	for _ in 0..count {
		let id = rng.random::<UserId>();
		let name = fake::faker::internet::en::Username()
			.fake_with_rng::<String, _>(rng)
			.parse::<Username>()
			.wrap_err("randomly generated an invalid username")?;

		users::create(id)
			.name(name.clone())
			.exec(&mut *conn)
			.await
			.wrap_err("failed to create user")?;

		tracing::info!(%id, %name, "created user");
	}

	Ok(())
}

async fn delete_users(conn: &mut DatabaseConnection<'_, '_>, count: u64) -> eyre::Result<()>
{
	users::delete(count)
		.exec(conn)
		.map_ok(|amount| tracing::info!(amount, "deleted users"))
		.await
		.wrap_err("failed to delete users")
}

async fn create_servers(
	conn: &mut DatabaseConnection<'_, '_>,
	rng: &mut ThreadRng,
	owner: Option<UserId>,
	game: Option<Game>,
	count: u64,
) -> eyre::Result<()>
{
	let potential_owners = users::get()
		.limit(100)
		.exec(&mut *conn)
		.try_collect::<Vec<_>>()
		.await
		.wrap_err("failed to fetch potential server owners")?;

	for _ in 0..count {
		let name = format!(
			"{}'s {}",
			fake::faker::internet::en::Username().fake_with_rng::<String, _>(rng),
			fake::faker::company::en::Bs().fake_with_rng::<String, _>(rng)
		)
		.parse::<ServerName>()
		.wrap_err("randomly generated an invalid server name")?;

		let host = match fake::faker::internet::en::IP().fake_with_rng::<IpAddr, _>(rng) {
			IpAddr::V4(ipv4_addr) => ServerHost::Ipv4(ipv4_addr),
			IpAddr::V6(ipv6_addr) => ServerHost::Ipv6(ipv6_addr),
		};

		let port = rng.random::<ServerPort>();
		let game = game.unwrap_or_else(|| rng.random::<Game>());
		let owner = owner
			.or_else(|| potential_owners.choose(rng).map(|user| user.id))
			.ok_or_eyre("no owner specified and no users in database")
			.suggestion("generate some users")?;

		let CreatedServer { id, .. } = servers::create()
			.name(name.clone())
			.host(host)
			.port(port)
			.game(game)
			.owned_by(owner)
			.exec(&mut *conn)
			.await
			.wrap_err("failed to create server")?;

		tracing::info!(%id, %name, "created server");
	}

	Ok(())
}

async fn delete_servers(
	conn: &mut DatabaseConnection<'_, '_>,
	owner: Option<UserId>,
	count: u64,
) -> eyre::Result<()>
{
	servers::delete(count)
		.maybe_owned_by(owner)
		.exec(conn)
		.map_ok(|amount| tracing::info!(amount, "deleted servers"))
		.await
		.wrap_err("failed to delete servers")
}

async fn create_maps(
	conn: &mut DatabaseConnection<'_, '_>,
	rng: &mut ThreadRng,
	mapper: Option<UserId>,
	state: Option<MapState>,
	courses: Option<NonZero<u16>>,
	count: u64,
) -> eyre::Result<()>
{
	let potential_mappers = users::get()
		.limit(100)
		.exec(&mut *conn)
		.try_collect::<Vec<_>>()
		.await
		.wrap_err("failed to fetch potential mappers")?;

	for _ in 0..count {
		let name = loop {
			let len = rng.random_range(2..=24);
			let name = (0_usize..len).map(|_| match b"_abcdefghijklmnopqrstuvwxyz".choose(rng) {
				Some(&ch) => ch as char,
				None => unreachable!(),
			});

			if let Ok(name) = format!("kz_{}", name.collect::<String>()).parse::<MapName>() {
				break name;
			}
		};

		let description = (rng.random_range(0..100) > 10_u8)
			.then(|| {
				fake::faker::lorem::en::Paragraph(1..11)
					.fake_with_rng::<String, _>(rng)
					.parse::<MapDescription>()
			})
			.transpose()
			.wrap_err("randomly generated an invalid map description")?;

		let mapper = mapper
			.or_else(|| potential_mappers.choose(rng).map(|user| user.id))
			.ok_or_eyre("no mapper specified and no users in database")
			.suggestion("generate some users")?;

		let courses = (1_u16..=courses.map_or_else(|| rng.random_range(1..=3), NonZero::get))
			.map(|raw| {
				NonZero::new(raw)
					.map(CourseLocalId::from)
					.unwrap_or_else(|| unreachable!())
			})
			.map(|local_id| {
				let name = fake::faker::company::en::Bs()
					.fake_with_rng::<String, _>(rng)
					.parse::<CourseName>()
					.wrap_err("randomly generated an invalid course name")?;

				let description = (rng.random_range(0..100) > 10_u8)
					.then(|| {
						fake::faker::lorem::en::Paragraph(1..11)
							.fake_with_rng::<String, _>(rng)
							.parse::<CourseDescription>()
					})
					.transpose()
					.wrap_err("randomly generated an invalid course description")?;

				let mapper_count = rng.random_range(1..=3);
				let mappers =
					potential_mappers.choose_multiple(rng, mapper_count).map(|user| user.id);

				let random_filter = |rng: &mut ThreadRng| -> eyre::Result<_> {
					let mut tiers = rng.random::<[Tier; 2]>();
					tiers.sort();

					let notes = (rng.random_range(0..100) > 10_u8)
						.then(|| {
							fake::faker::lorem::en::Paragraph(1..11)
								.fake_with_rng::<String, _>(rng)
								.parse::<FilterNotes>()
						})
						.transpose()
						.wrap_err("randomly generated an invalid course description")?;

					Ok(NewFilter::builder()
						.nub_tier(tiers[0])
						.pro_tier(tiers[1])
						.ranked({
							tiers.iter().any(|tier| tier.is_humanly_possible())
								&& rng.random_range(0..100) > 10_u8
						})
						.maybe_notes(notes)
						.build())
				};

				let (cs2_filters, csgo_filters) = if rng.random::<bool>() {
					let cs2_filters = NewCS2Filters::builder()
						.vnl(random_filter(rng)?)
						.ckz(random_filter(rng)?)
						.build();

					let csgo_filters = (rng.random_range(0..100) < 10_u8)
						.then(|| -> eyre::Result<_> {
							Ok(NewCSGOFilters::builder()
								.kzt(random_filter(rng)?)
								.skz(random_filter(rng)?)
								.vnl(random_filter(rng)?)
								.build())
						})
						.transpose()?;

					(Some(cs2_filters), csgo_filters)
				} else {
					let cs2_filters = (rng.random_range(0..100) < 10_u8)
						.then(|| -> eyre::Result<_> {
							Ok(NewCS2Filters::builder()
								.vnl(random_filter(rng)?)
								.ckz(random_filter(rng)?)
								.build())
						})
						.transpose()?;

					let csgo_filters = NewCSGOFilters::builder()
						.kzt(random_filter(rng)?)
						.skz(random_filter(rng)?)
						.vnl(random_filter(rng)?)
						.build();

					(cs2_filters, Some(csgo_filters))
				};

				let filters = NewFilters::builder()
					.maybe_cs2(cs2_filters)
					.maybe_csgo(csgo_filters)
					.build();

				Ok(NewCourse::builder(local_id)
					.name(name)
					.maybe_description(description)
					.mappers(mappers)
					.filters(filters)
					.build())
			})
			.collect::<eyre::Result<Vec<_>>>()?;

		let id = maps::create(rng.random())
			.name(name.clone())
			.maybe_description(description)
			.state(state.unwrap_or(MapState::Approved))
			.checksum(rng.random())
			.created_by(mapper)
			.courses(courses)
			.exec(&mut *conn)
			.await
			.wrap_err("failed to create map")?;

		tracing::info!(%id, %name, %mapper, "created map");
	}

	Ok(())
}

async fn delete_maps(
	conn: &mut DatabaseConnection<'_, '_>,
	mapper: Option<UserId>,
	count: u64,
) -> eyre::Result<()>
{
	maps::delete(count)
		.maybe_created_by(mapper)
		.exec(conn)
		.map_ok(|amount| tracing::info!(amount, "deleted maps"))
		.await
		.wrap_err("failed to delete maps")
}

async fn create_players(
	conn: &mut DatabaseConnection<'_, '_>,
	rng: &mut ThreadRng,
	count: u64,
) -> eyre::Result<()>
{
	for _ in 0..count {
		let id = rng.random::<PlayerId>();
		let name = fake::faker::internet::en::Username()
			.fake_with_rng::<String, _>(rng)
			.parse::<PlayerName>()
			.wrap_err("randomly generated an invalid player name")?;

		players::create(id)
			.name(name.clone())
			.ip_address(fake::faker::internet::en::IPv4().fake_with_rng::<Ipv4Addr, _>(rng))
			.exec(&mut *conn)
			.await
			.wrap_err("failed to create player")?;

		tracing::info!(%id, %name, "created player");
	}

	Ok(())
}

async fn delete_players(conn: &mut DatabaseConnection<'_, '_>, count: u64) -> eyre::Result<()>
{
	players::delete(count)
		.exec(conn)
		.map_ok(|amount| tracing::info!(amount, "deleted players"))
		.await
		.wrap_err("failed to delete players")
}

async fn create_records(
	conn: &mut DatabaseConnection<'_, '_>,
	rng: &mut ThreadRng,
	filter: Option<FilterId>,
	player: Option<PlayerId>,
	server: Option<ServerId>,
	count: u64,
) -> eyre::Result<()>
{
	let mode = if let Some(filter_id) = filter {
		maps::get_mode_by_filter_id(filter_id)
			.exec(&mut *conn)
			.await
			.wrap_err("failed to fetch filter information")?
	} else {
		None
	};

	let potential_filters = maps::get_filters()
		.limit(u64::MAX)
		.exec(&mut *conn)
		.try_collect::<Vec<_>>()
		.await
		.wrap_err("failed to fetch potential filters")?;

	let potential_players = players::get()
		.limit(10000)
		.exec(&mut *conn)
		.try_collect::<Vec<_>>()
		.await
		.wrap_err("failed to fetch potential players")?;

	let mut potential_servers = servers::get(Game::CS2)
		.limit(1000)
		.exec(&mut *conn)
		.try_collect::<Vec<_>>()
		.await
		.wrap_err("failed to fetch potential servers")?;

	servers::get(Game::CSGO)
		.limit(1000)
		.exec(&mut *conn)
		.try_collect_into(&mut potential_servers)
		.await?;

	let latest_cs2_version = plugin::get_latest_version_id(Game::CS2).exec(&mut *conn).await?;
	let latest_csgo_version = plugin::get_latest_version_id(Game::CSGO).exec(&mut *conn).await?;

	let mut sessions = HashMap::<ServerId, ServerSessionId>::new();
	let mut modes = HashMap::<FilterId, Mode>::new();

	for _ in 0..count {
		let filter_id = filter
			.or_else(|| potential_filters.choose(rng).map(|filter| filter.id))
			.ok_or_eyre("no filters found")
			.suggestion("generate some maps")?;

		let player_id = player
			.or_else(|| potential_players.choose(rng).map(|player| player.id))
			.ok_or_eyre("no players found")
			.suggestion("generate some players")?;

		let session_id = {
			let server_id = server
				.or_else(|| potential_servers.choose(rng).map(|server| server.id))
				.ok_or_eyre("no servers found")
				.suggestion("generate some servers")?;

			match sessions.entry(server_id) {
				hash_map::Entry::Occupied(entry) => *entry.get(),
				hash_map::Entry::Vacant(entry) => {
					let mode_to_game = |mode| match mode {
						Mode::Vanilla | Mode::Classic => Game::CS2,
						Mode::KZTimer | Mode::SimpleKZ | Mode::VanillaCSGO => Game::CSGO,
					};

					let game = mode_to_game(match mode {
						Some(mode) => mode,
						None => match modes.entry(filter_id) {
							hash_map::Entry::Occupied(entry) => *entry.get(),
							hash_map::Entry::Vacant(entry) => {
								let mode = maps::get_mode_by_filter_id(filter_id)
									.exec(&mut *conn)
									.await
									.wrap_err("failed to fetch filter information")?
									.ok_or_else(|| {
										eyre!("failed to fetch mode for filter {filter_id}")
									})?;

								*entry.insert(mode)
							},
						},
					});

					let plugin_version_id = match game {
						Game::CS2 => latest_cs2_version,
						Game::CSGO => latest_csgo_version,
					}
					.ok_or_else(|| eyre!("no plugin version for {game:?} found"))?;

					let session_id = servers::create_session(server_id)
						.plugin_version_id(plugin_version_id)
						.exec(&mut *conn)
						.await
						.wrap_err("failed to create fake server session")?;

					*entry.insert(session_id)
				},
			}
		};

		#[allow(clippy::cast_precision_loss)]
		let time = {
			let min = Duration::from_secs(15);
			let max = Duration::HOUR * 12;

			Time::from(Seconds::from(rng.random_range(min.as_secs()..=max.as_secs()) as f64))
		};

		let teleports = Teleports::from(if rng.random_range(0..100) > 35_u8 {
			rng.random_range(1..=1000)
		} else {
			0_u32
		});

		let CreatedRecord { id, .. } = records::create(filter_id, player_id)
			.session_id(session_id)
			.time(time)
			.teleports(teleports)
			.styles(Styles::default())
			.exec(&mut *conn)
			.await
			.wrap_err("failed to create record")?;

		tracing::info!(%id, "created record");
	}

	Ok(())
}

async fn delete_records(conn: &mut DatabaseConnection<'_, '_>, count: u64) -> eyre::Result<()>
{
	records::delete(count)
		.exec(conn)
		.map_ok(|amount| tracing::info!(amount, "deleted records"))
		.await
		.wrap_err("failed to delete records")
}
