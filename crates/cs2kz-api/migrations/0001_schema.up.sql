-- Users are Steam accounts who have authenticated themselves with the API
-- via OpenID, e.g. on the website. These are separate from "players".
CREATE TABLE Users (
  -- The user's SteamID.
  id INT8 UNSIGNED NOT NULL PRIMARY KEY,
  -- Bitflags describing which privileges the user has.
  -- These are used for authorization.
  permissions INT8 UNSIGNED NOT NULL DEFAULT 0,
  -- The user's email address. An empty string is used instead of NULL to
  -- signal that we don't know a user's email. We don't require users to tell
  -- us their email unless they're an admin or server owner.
  email VARCHAR(255) NOT NULL UNIQUE DEFAULT "",
  -- When the account was created.
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  -- When the account was last authenticated.
  last_seen_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Sessions for user logins.
CREATE TABLE UserSessions (
  -- The session ID.
  --
  -- This is a randomly generated UUID we give to the user so they can
  -- authenticate themselves.
  id BINARY(16) NOT NULL PRIMARY KEY,
  -- The user's ID.
  user_id INT8 UNSIGNED NOT NULL REFERENCES Users(id),
  -- When the session was created.
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  -- When the session will expire.
  expires_at TIMESTAMP NOT NULL
);

-- Opaque credentials used for internal authentication, e.g. GitHub CI.
CREATE TABLE Credentials (
  -- The name of the credentials.
  --
  -- This is an arbitrary value only used for identifying which credentials
  -- are used for which purpose.
  name VARCHAR(255) NOT NULL PRIMARY KEY,
  -- A randomly generated UUID included in request headers to authenticate the
  -- request.
  access_key BINARY(16) NOT NULL,
  -- When the credentials were created.
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  -- When the credentials will expire.
  expires_at TIMESTAMP NOT NULL
);

-- cs2kz-metamod versions.
--
-- This table holds a list of all official plugin releases.
CREATE TABLE PluginVersions (
  -- The version's ID.
  --
  -- While `name` and `git_revision` are already unique, they are slower for
  -- JOINs and sorting than integers.
  id INT2 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  -- The name of the version.
  --
  -- This is a SemVer compatible version identifier, e.g. "0.14.5"
  name VARCHAR(14) NOT NULL UNIQUE CHECK(name != ""),
  -- The git revision associated with this release.
  git_revision BINARY(40) NOT NULL UNIQUE,
  -- When this release was published.
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Players are Steam accounts who have joined approved CS2 servers running the
-- cs2kz-metamod plugin.
CREATE TABLE Players (
  -- The player's SteamID.
  id INT8 UNSIGNED NOT NULL PRIMARY KEY,
  -- The player's name.
  name VARCHAR(255) NOT NULL,
  -- The player's IP address.
  --
  -- CS2 always reports player IPs as IPv4.
  ip_address INET4 NOT NULL,
  -- The player's in-game preferences.
  --
  -- This is an arbitrary JSON object we store only so CS2 servers can retreive
  -- and override it. _We_ don't actually care what's in there.
  preferences JSON NOT NULL DEFAULT "{}",
  -- When the player first joined an approved CS2 server.
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  -- When the player last joined an approved CS2 server.
  last_seen_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Officially approved CS2 servers.
--
-- These servers are running the cs2kz-metamod plugin and may register players,
-- submit jumpstats and records, among other things.
CREATE TABLE Servers (
  -- The server's ID.
  id INT2 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  -- The server's name.
  name VARCHAR(255) NOT NULL UNIQUE,
  -- The server's host.
  --
  -- This may be an IPv4/IPv6 address, or a domain name.
  host VARCHAR(255) NOT NULL,
  -- The server's port.
  port INT2 UNSIGNED NOT NULL,
  -- The SteamID of the server's owner.
  owner_id INT8 UNSIGNED NOT NULL REFERENCES Users(id),
  -- An access key the server uses to authenticate itself with the API.
  --
  -- This is a randomly generated UUID that may be reset by both the owner and
  -- admins at any point in time.
  --
  -- The special value of all zeros is used to represent revoked access keys.
  -- Servers with this special key will not pass authentication and can only
  -- have their key reset by an admin, not the owner.
  access_key BINARY(16) NOT NULL,
  -- When this server was approved.
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  -- When this server last authenticated itself.
  last_seen_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT UC_host_port UNIQUE (host, port)
);

-- Player bans.
--
-- Players may be banned for a variety of reasons, most of which fall under the
-- category of "cheating".
CREATE TABLE Bans (
  -- The ban's ID.
  id INT8 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  -- The SteamID of the player who got banned.
  player_id INT8 UNSIGNED NOT NULL REFERENCES Players(id),
  -- The player's IP address at the time of the ban.
  player_ip INET4 NOT NULL,
  -- Either the SteamID of the admin who banned the player, or the ID of the
  -- server on which the player was auto-banned by the Anti-Cheat system.
  --
  -- Server IDs are never high enough to be a valid SteamID, and SteamIDs are
  -- never low enough to be a valid Server ID, so we can identify which one is
  -- actually used by checking which range the value falls into.
  banned_by INT8 UNSIGNED NOT NULL,
  -- The reason for the ban.
  reason VARCHAR(255) NOT NULL,
  -- The ID of the plugin version that was in use when the player was banned.
  --
  -- If the player was banned by Anti-Cheat, this will be the version used on
  -- the server the ban was issued by. If the player was banned by an admin,
  -- this will simply be the latest version at the time of the ban.
  plugin_version_id INT2 UNSIGNED NOT NULL REFERENCES PluginVersions(id),
  -- When this ban was issued.
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  -- When this ban will expire.
  expires_at TIMESTAMP NOT NULL
);

-- This table holds records of explicit unbans.
--
-- Implicit unbans, i.e. expired bans, are not recorded.
CREATE TABLE Unbans (
  -- The unban's ID.
  id INT8 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  -- The ID of the ban this unban corresponds to.
  ban_id INT8 UNSIGNED NOT NULL UNIQUE REFERENCES Bans(id),
  -- The SteamID of the admin who issued this unban.
  unbanned_by INT8 UNSIGNED NOT NULL REFERENCES Players(id),
  -- The reason for the unban.
  reason VARCHAR(255) NOT NULL,
  -- When this unban was issued.
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Officially approved maps.
CREATE TABLE Maps (
  -- The map's ID.
  id INT2 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  -- The map's Steam Workshop ID.
  --
  -- Every official map must be uploaded to the workshop.
  workshop_id INT4 UNSIGNED NOT NULL,
  -- The map's name.
  --
  -- This is not `UNIQUE`, because there may be multiple versions of each map.
  -- A new map version is created whenever the gameplay of the map changes.
  name VARCHAR(255) NOT NULL,
  -- A description of the map.
  description TEXT NOT NULL DEFAULT "",
  -- The approval state the map is currently in.
  --
  -- For more details, see the `MapApprovalStatus` type in the Rust code.
  approval_status INT1 NOT NULL DEFAULT -1,
  -- The MD5 hash digest of the map's `.vpk` file.
  hash BINARY(16) NOT NULL,
  -- When this map was approved.
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- A join table documenting which players contributed to the creation of which
-- map.
--
-- Every map must have at least one mapper associated with it.
CREATE TABLE Mappers (
  -- The ID of the map.
  map_id INT2 UNSIGNED NOT NULL REFERENCES Maps(id),
  -- The ID of the mapper.
  player_id INT8 UNSIGNED NOT NULL REFERENCES Players(id),
  PRIMARY KEY (map_id, player_id)
);

-- A course associated with a map.
--
-- Every map has one or more courses.
CREATE TABLE Courses (
  -- The course's ID.
  id INT2 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  -- The ID of the map this course belongs to.
  map_id INT2 UNSIGNED NOT NULL REFERENCES Maps(id),
  -- The course's name.
  name VARCHAR(255) NOT NULL,
  -- A description of the course.
  description TEXT NOT NULL DEFAULT "",
  UNIQUE (map_id, name)
);

-- A join table documenting which players contributed to the creation of which
-- map course.
--
-- Every course must have at least one mapper associated with it.
CREATE TABLE CourseMappers (
  -- The ID of the course.
  course_id INT2 UNSIGNED NOT NULL REFERENCES Courses(id),
  -- The ID of the mapper.
  player_id INT8 UNSIGNED NOT NULL REFERENCES Players(id),
  PRIMARY KEY (course_id, player_id)
);

-- "Filters" are used to attach attributes to courses, but per mode+runtype.
--
-- These attributes may differ between modes, and whether teleports can be
-- used or not. Because there are two modes, every course always has 4 filters.
CREATE TABLE CourseFilters (
  -- The filter's ID.
  id INT2 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  -- The ID of the course this filter belongs to.
  course_id INT2 UNSIGNED NOT NULL REFERENCES Courses(id),
  -- The mode for this filter.
  `mode` INT1 UNSIGNED NOT NULL CHECK(`mode` BETWEEN 1 AND 2),
  -- Whether this filter is for runs with teleports.
  teleports BOOLEAN NOT NULL,
  -- The tier associated with this filter.
  --
  -- This rates the filter's difficulty.
  tier INT1 UNSIGNED NOT NULL CHECK(tier BETWEEN 1 AND 10),
  -- The ranked status of this filter.
  --
  -- This determines whether players can gain points by submitting records on
  -- this filter.
  ranked_status INT1 NOT NULL CHECK(
    (ranked_status BETWEEN -1 AND 1)
    AND (
      tier <= 8
      OR ranked_status = -1
    )
  ),
  -- Any additional notes, e.g. justifications for the tier given to this
  -- filter.
  notes TEXT NOT NULL DEFAULT "",
  UNIQUE (course_id, `mode`, teleports)
);

-- Records, or "runs", submitted by players, on approved servers.
CREATE TABLE Records (
  -- The record's ID.
  id INT8 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  -- The SteamID of the player who performed this run.
  player_id INT8 UNSIGNED NOT NULL REFERENCES Players(id),
  -- The ID of the server which submitted this run.
  server_id INT2 UNSIGNED NOT NULL REFERENCES Servers(id),
  -- The ID of the filter this run was performed on.
  filter_id INT2 UNSIGNED NOT NULL REFERENCES CourseFilters(id),
  -- Styles the player used during the run.
  --
  -- These are bitflags so we can store multiple styles in a single integer.
  -- For more details, see the `Style` and `Styles` types in the Rust code.
  styles INT4 UNSIGNED NOT NULL,
  -- How many teleports were used during the run.
  teleports INT4 UNSIGNED NOT NULL,
  -- The time it took to complete the run, in seconds.
  time FLOAT8 NOT NULL,
  -- The total amount of bhops.
  bhop_count INT4 UNSIGNED NOT NULL,
  -- The amount of bhops that are considered a "perf" by the mode.
  perf_count INT4 UNSIGNED NOT NULL,
  -- The amount of tick-perfect bhops.
  perfect_perfs_count INT4 UNSIGNED NOT NULL,
  -- The ID of the plugin version that was in use when the record was performed.
  plugin_version_id INT2 UNSIGNED NOT NULL REFERENCES PluginVersions(id),
  -- When this record was submitted.
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Records that are suspected to be cheated.
--
-- These will be investigated by admins and eventually moved either into
-- `Records` or `Records_Cheated`.
CREATE TABLE Records_Suspicious LIKE Records;

-- Records that are considered cheated.
--
-- These may be investigated by admins and moved back into `Records` or
-- `Records_Suspicious`, if the case is not blatantly obvious.
CREATE TABLE Records_Cheated LIKE Records;

-- Records that are bugged or were performed using exploits.
--
-- These aren't really "cheated", but should nevertheless not appear on
-- leaderboards.
CREATE TABLE Records_Hidden LIKE Records;

-- Jumpstats submitted by players on approved servers.
CREATE TABLE Jumpstats (
  -- The jumpstat's ID.
  id INT8 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  -- The SteamID of the player who performed this jump.
  player_id INT8 UNSIGNED NOT NULL REFERENCES Players(id),
  -- The ID of the server which submitted this jumpstat.
  server_id INT2 UNSIGNED NOT NULL REFERENCES Servers(id),
  -- The mode the jump was performed in.
  `mode` INT1 UNSIGNED NOT NULL CHECK(`mode` BETWEEN 1 AND 2),
  -- The type of jump that was performed.
  --
  -- For more details, see the `JumpType` type in the Rust code.
  jump_type INT1 UNSIGNED NOT NULL CHECK(jump_type BETWEEN 1 AND 7),
  -- How many strafes were used in this jump.
  strafes INT1 UNSIGNED NOT NULL,
  -- The distance cleared by the jump.
  distance FLOAT4 NOT NULL,
  -- The amount of time, in seconds, during which the player was gaining speed.
  sync FLOAT4 NOT NULL,
  -- The amount of speed the player was moving at when leaving the ground.
  pre FLOAT4 NOT NULL,
  -- The peak speed during the jump.
  max FLOAT4 NOT NULL,
  -- The amount of time, in seconds, during which the player pressed both of
  -- their strafe keys at the same time.
  overlap FLOAT4 NOT NULL,
  -- The amount of time, in seconds, during which the player pressed a key, but
  -- not both of their strafe keys, and neither gained nor lost speed.
  --
  -- For example, facing straight forwards and holding `W` mid-air will add to
  -- `bad_angles`.
  bad_angles FLOAT4 NOT NULL,
  -- The amount of time, in seconds, during which the player pressed no keys.
  dead_air FLOAT4 NOT NULL,
  -- The peak height during the jump.
  height FLOAT4 NOT NULL,
  -- How optimal the path taken through the air was.
  airpath FLOAT4 NOT NULL,
  -- How far the landing position deviated from the origin position.
  deviation FLOAT4 NOT NULL,
  -- The average strafe width, in degrees.
  average_width FLOAT4 NOT NULL,
  -- How much time, in seconds, the player spent in the air.
  airtime FLOAT4 NOT NULL,
  -- The ID of the plugin version that was in use when the jump was performed.
  plugin_version_id INT2 UNSIGNED NOT NULL REFERENCES PluginVersions(id),
  -- When this jump was submitted.
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Same purpose as `Records_Suspicious`, but for jumpstats.
CREATE TABLE Jumpstats_Suspicious LIKE Jumpstats;

-- Same purpose as `Records_Cheated`, but for jumpstats.
CREATE TABLE Jumpstats_Cheated LIKE Jumpstats;

-- Same purpose as `Records_Hidden`, but for jumpstats.
CREATE TABLE Jumpstats_Hidden LIKE Jumpstats;

-- Sessions to keep track of player statistics.
--
-- A session begins when a player connects to a server and ends when they
-- disconnect.
CREATE TABLE GameSessions (
  -- The ID of the session.
  id INT8 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  -- The SteamID of the player this session is associated with.
  player_id INT8 UNSIGNED NOT NULL REFERENCES Players(id),
  -- The ID of the server which submitted this session.
  server_id INT2 UNSIGNED NOT NULL REFERENCES Servers(id),
  -- The amount of time, in seconds, the player was actively doing something.
  time_active FLOAT4 NOT NULL,
  -- The amount of time, in seconds, the player was spectating other players.
  time_spectating FLOAT4 NOT NULL,
  -- The amount of time, in seconds, the player was not doing anything.
  time_afk FLOAT4 NOT NULL,
  -- The total amount of bhops.
  bhop_count INT4 UNSIGNED NOT NULL,
  -- The amount of bhops that are considered a "perf" by the mode.
  perf_count INT4 UNSIGNED NOT NULL,
  -- The amount of tick-perfect bhops.
  perfect_perfs_count INT4 UNSIGNED NOT NULL,
  -- When this session was submitted.
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Course sessions are similar to game sessions, but are per course+mode.
CREATE TABLE CourseSessions (
  -- The ID of the session.
  id INT8 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  -- The ID of the game session this course session is part of.
  game_session_id INT8 UNSIGNED NOT NULL REFERENCES GameSessions(id),
  -- The ID of the course this session is associated with.
  course_id INT2 UNSIGNED NOT NULL REFERENCES Courses(id),
  -- The mode this session is associated with.
  `mode` INT1 UNSIGNED NOT NULL CHECK(`mode` BETWEEN 1 AND 2),
  -- The amount of time, in seconds, the player spent on this course with a
  -- running timer.
  playtime FLOAT4 NOT NULL,
  -- The total amount of bhops.
  bhop_count INT4 UNSIGNED NOT NULL,
  -- The amount of bhops that are considered a "perf" by the mode.
  perf_count INT4 UNSIGNED NOT NULL,
  -- The amount of tick-perfect bhops.
  perfect_perfs_count INT4 UNSIGNED NOT NULL,
  -- The amount of times the player left the start zone of the course.
  started_runs INT2 UNSIGNED NOT NULL,
  -- The amount of times the player entered the end zone of the course.
  finished_runs INT2 UNSIGNED NOT NULL
);
