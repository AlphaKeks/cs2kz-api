-- cs2kz-metamod releases
CREATE TABLE PluginVersions (
  id INT2 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  -- SemVer version (e.g. "0.5.3")
  name VARCHAR(14) NOT NULL UNIQUE,
  -- Git revision of the release commit
  revision VARCHAR(40) NOT NULL UNIQUE,
  created_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT valid_name CHECK(name != ''),
  CONSTRAINT valid_revision CHECK(LENGTH(revision) = 40)
);

-- Opaque API keys for internal use
CREATE TABLE Credentials (
  name VARCHAR(255) NOT NULL,
  -- UUID
  value BINARY(16) NOT NULL PRIMARY KEY,
  created_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  expires_on TIMESTAMP NOT NULL DEFAULT '2038-01-19 03:14:07',
  CONSTRAINT valid_name CHECK(name != '')
);

CREATE TABLE Users (
  -- 64-bit SteamID
  id INT8 UNSIGNED NOT NULL PRIMARY KEY,
  -- Steam name
  name VARCHAR(255) NOT NULL,
  -- may be mapped IPv4
  ip_address INET6 NOT NULL,
  -- arbitrary data; read/written by CS2 servers
  game_preferences JSON NOT NULL DEFAULT '{}',
  -- bitflags, see `Permissions` type in Rust
  permissions INT8 UNSIGNED NOT NULL DEFAULT 0,
  created_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  last_seen_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT valid_name CHECK(name != '')
);

CREATE TABLE UserSessions (
  -- UUID
  id BINARY(16) NOT NULL PRIMARY KEY,
  user_id INT8 UNSIGNED NOT NULL REFERENCES Users(id),
  created_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  expires_on TIMESTAMP NOT NULL DEFAULT '2038-01-19 03:14:07'
);

CREATE TABLE Servers (
  id INT2 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  name VARCHAR(255) NOT NULL UNIQUE,
  -- IPv4/IPv6/domain
  host VARCHAR(255) NOT NULL,
  port INT2 UNSIGNED NOT NULL,
  owner_id INT8 UNSIGNED NOT NULL REFERENCES Users(id),
  -- UUID; all zeroes -> unauthorized
  access_key BINARY(16) NOT NULL,
  created_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  last_seen_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT valid_name CHECK(name != ''),
  CONSTRAINT valid_host CHECK(host != ''),
  CONSTRAINT unique_host_port UNIQUE (host, port)
);

CREATE TABLE Maps (
  id INT2 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  -- not `UNIQUE` because there might be multiple versions of any map
  name VARCHAR(255) NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  workshop_id INT4 UNSIGNED NOT NULL,
  -- see `MapState` enum in Rust
  state INT1 NOT NULL DEFAULT -1,
  -- MD5 hash of the map's `.vpk` file
  hash BINARY(16) NOT NULL,
  created_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT valid_name CHECK(name != ''),
  CONSTRAINT valid_state CHECK(state BETWEEN -1 AND 1)
);

CREATE TABLE Courses (
  id INT2 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  map_id INT2 UNSIGNED NOT NULL REFERENCES Maps(id),
  -- not `UNIQUE` because there might be multiple versions of any map (and therefore courses)
  name VARCHAR(255) NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  created_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT valid_name CHECK(name != ''),
  UNIQUE (map_id, name)
);

CREATE TABLE CourseFilters (
  id INT2 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  course_id INT2 UNSIGNED NOT NULL REFERENCES Courses(id),
  -- See `Mode` enum in Rust
  game_mode INT1 UNSIGNED NOT NULL,
  has_teleports BOOLEAN NOT NULL,
  tier INT1 UNSIGNED NOT NULL,
  -- See `RankedStatus` enum in Rust
  ranked_status INT1 NOT NULL DEFAULT -1,
  notes TEXT NOT NULL DEFAULT '',
  UNIQUE (course_id, game_mode, has_teleports),
  CONSTRAINT valid_mode CHECK(game_mode BETWEEN 1 AND 2),
  CONSTRAINT valid_tier CHECK(tier BETWEEN 1 AND 10),
  CONSTRAINT valid_ranked_status CHECK(
    (ranked_status BETWEEN -1 AND 1)
    AND (
      tier <= 8
      OR ranked_status = -1
    )
  )
);

CREATE TABLE Mappers (
  user_id INT8 UNSIGNED NOT NULL REFERENCES Users(id),
  map_id INT2 UNSIGNED NOT NULL REFERENCES Maps(id),
  PRIMARY KEY (user_id, map_id)
);

CREATE TABLE CourseMappers (
  user_id INT8 UNSIGNED NOT NULL REFERENCES Users(id),
  course_id INT2 UNSIGNED NOT NULL REFERENCES Courses(id),
  PRIMARY KEY (user_id, course_id)
);

CREATE TABLE Records (
  id INT8 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  user_id INT8 UNSIGNED NOT NULL REFERENCES Users(id),
  server_id INT2 UNSIGNED NOT NULL REFERENCES Servers(id),
  filter_id INT2 UNSIGNED NOT NULL REFERENCES CourseFilters(id),
  -- bitflags, see `Styles` type in Rust
  styles INT4 UNSIGNED NOT NULL,
  teleports INT4 UNSIGNED NOT NULL,
  -- seconds
  time FLOAT8 NOT NULL,
  -- total bhop count
  bhops INT4 UNSIGNED NOT NULL,
  -- "perf" count (according to the mode)
  perfs INT4 UNSIGNED NOT NULL,
  -- tick-perfect bhop count
  perfect_perfs INT4 UNSIGNED NOT NULL,
  plugin_version_id INT2 UNSIGNED NOT NULL REFERENCES PluginVersions(id),
  created_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- records that might be cheated and have to be investigated by a human
CREATE TABLE Records_Suspicious LIKE Records;

-- records that are cheated
CREATE TABLE Records_Cheated LIKE Records;

-- records that are hidden (e.g. records using exploits)
CREATE TABLE Records_Hidden LIKE Records;

CREATE TABLE Jumpstats (
  id INT8 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  user_id INT8 UNSIGNED NOT NULL REFERENCES Users(id),
  server_id INT2 UNSIGNED NOT NULL REFERENCES Servers(id),
  -- See `JumpType` enum in Rust
  jump_type INT1 UNSIGNED NOT NULL,
  -- See `Mode` enum in Rust
  game_mode INT1 UNSIGNED NOT NULL,
  strafe_count INT1 UNSIGNED NOT NULL,
  distance FLOAT4 NOT NULL,
  sync FLOAT4 NOT NULL,
  pre FLOAT4 NOT NULL,
  max FLOAT4 NOT NULL,
  overlap FLOAT4 NOT NULL,
  bad_angles FLOAT4 NOT NULL,
  dead_air FLOAT4 NOT NULL,
  height FLOAT4 NOT NULL,
  airpath FLOAT4 NOT NULL,
  deviation FLOAT4 NOT NULL,
  average_width FLOAT4 NOT NULL,
  airtime FLOAT4 NOT NULL,
  plugin_version_id INT2 UNSIGNED NOT NULL REFERENCES PluginVersions(id),
  created_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- jumpstats that might be cheated and have to be investigated by a human
CREATE TABLE Jumpstats_Suspicious LIKE Jumpstats;

-- jumpstats that are cheated
CREATE TABLE Jumpstats_Cheated LIKE Jumpstats;

CREATE TABLE Bans (
  id INT8 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  user_id INT8 UNSIGNED NOT NULL REFERENCES Users(id),
  user_ip INET6 NOT NULL,
  server_id INT2 UNSIGNED NOT NULL REFERENCES Servers(id),
  admin_id INT8 UNSIGNED NOT NULL REFERENCES Users(id),
  reason VARCHAR(255) NOT NULL,
  plugin_version_id INT2 UNSIGNED NOT NULL REFERENCES PluginVersions(id),
  created_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  expires_on TIMESTAMP NOT NULL DEFAULT '2038-01-19 03:14:07'
);

CREATE TABLE Unbans (
  id INT8 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  ban_id INT8 UNSIGNED NOT NULL UNIQUE REFERENCES Bans(id),
  admin_id INT8 UNSIGNED NOT NULL REFERENCES Users(id),
  reason VARCHAR(255) NOT NULL,
  created_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE GameSessions (
  id INT8 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  user_id INT8 UNSIGNED NOT NULL REFERENCES Users(id),
  server_id INT2 UNSIGNED NOT NULL REFERENCES Servers(id),
  -- seconds
  time_active INT2 UNSIGNED NOT NULL,
  -- seconds
  time_spectating INT2 UNSIGNED NOT NULL,
  -- seconds
  time_afk INT2 UNSIGNED NOT NULL,
  -- total bhop count
  bhops INT4 UNSIGNED NOT NULL,
  -- "perf" count (according to the mode)
  perfs INT4 UNSIGNED NOT NULL,
  created_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE CourseSessions (
  id INT8 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  game_session_id INT8 UNSIGNED NOT NULL REFERENCES GameSessions(id),
  course_id INT2 UNSIGNED NOT NULL REFERENCES Courses(id),
  -- See `Mode` enum in Rust
  game_mode INT1 UNSIGNED NOT NULL,
  -- seconds
  playtime INT2 UNSIGNED NOT NULL,
  -- total bhop count
  bhops INT4 UNSIGNED NOT NULL,
  -- "perf" count (according to the mode)
  perfs INT4 UNSIGNED NOT NULL,
  started_runs INT2 UNSIGNED NOT NULL,
  finished_runs INT2 UNSIGNED NOT NULL,
  created_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
