CREATE TABLE IF NOT EXISTS PluginVersions (
  id INT2 UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  major INT8 UNSIGNED NOT NULL,
  minor INT8 UNSIGNED NOT NULL,
  patch INT8 UNSIGNED NOT NULL,
  pre VARCHAR(255) NOT NULL,
  build VARCHAR(255) NOT NULL,
  game ENUM('cs2', 'csgo') NOT NULL,
  git_revision BINARY(20) NOT NULL,
  linux_checksum BINARY(16) NOT NULL,
  windows_checksum BINARY(16) NOT NULL,
  is_cutoff BOOLEAN NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT NOW(),
  UNIQUE (game, git_revision),
  UNIQUE (game, major, minor, patch, pre, build)
);

CREATE TABLE IF NOT EXISTS ModeChecksums (
  mode ENUM(
    'vanilla',
    'classic',
    'kztimer',
    'simplekz',
    'vanilla-csgo'
  ) NOT NULL,
  plugin_version_id INT2 UNSIGNED REFERENCES PluginVersions(id),
  linux_checksum BINARY(16) NOT NULL,
  windows_checksum BINARY(16) NOT NULL,
  PRIMARY KEY (mode, plugin_version_id)
);

CREATE TABLE IF NOT EXISTS StyleChecksums (
  style ENUM('auto-bhop') NOT NULL,
  plugin_version_id INT2 UNSIGNED REFERENCES PluginVersions(id),
  linux_checksum BINARY(16) NOT NULL,
  windows_checksum BINARY(16) NOT NULL,
  PRIMARY KEY (style, plugin_version_id)
);

CREATE TABLE IF NOT EXISTS AccessKeys (
  name VARCHAR(255) PRIMARY KEY CHECK(name != ''),
  value BINARY(16) NOT NULL UNIQUE,
  expires_at TIMESTAMP NOT NULL
);

CREATE TABLE IF NOT EXISTS Users (
  -- SteamID64
  id INT8 UNSIGNED PRIMARY KEY,
  discord_id INT8 UNSIGNED CHECK(discord_id != 0),
  name VARCHAR(255) NOT NULL CHECK(name != ''),
  email_address VARCHAR(255) UNIQUE CHECK(email_address != ''),
  permissions INT8 UNSIGNED NOT NULL DEFAULT 0,
  server_budget INT1 UNSIGNED NOT NULL DEFAULT 0,
  created_at TIMESTAMP NOT NULL DEFAULT NOW(),
  FULLTEXT (name)
);

CREATE TABLE IF NOT EXISTS UserSessions (
  id BINARY(16) PRIMARY KEY,
  user_id INT8 UNSIGNED NOT NULL REFERENCES Users(id) ON DELETE CASCADE,
  created_at TIMESTAMP NOT NULL DEFAULT NOW(),
  expires_at TIMESTAMP NOT NULL
);

CREATE TABLE IF NOT EXISTS Servers (
  id INT2 UNSIGNED PRIMARY KEY AUTO_INCREMENT,
  name VARCHAR(255) NOT NULL UNIQUE CHECK(name != ''),
  host VARCHAR(255) NOT NULL CHECK (host != ''),
  port INT2 UNSIGNED NOT NULL,
  game ENUM('cs2', 'csgo') NOT NULL,
  owner_id INT8 UNSIGNED NOT NULL REFERENCES Users(id),
  access_key BINARY(16) UNIQUE,
  created_at TIMESTAMP NOT NULL DEFAULT NOW(),
  last_seen_at TIMESTAMP,
  UNIQUE (host, port),
  FULLTEXT (name)
);

CREATE TABLE IF NOT EXISTS ServerSessions (
  id INT8 UNSIGNED PRIMARY KEY AUTO_INCREMENT,
  server_id INT2 UNSIGNED NOT NULL REFERENCES Servers(id) ON DELETE CASCADE,
  plugin_version_id INT2 UNSIGNED REFERENCES PluginVersions(id),
  created_at TIMESTAMP NOT NULL,
  ended_at TIMESTAMP
);

CREATE TABLE IF NOT EXISTS Maps (
  id INT2 UNSIGNED PRIMARY KEY AUTO_INCREMENT,
  workshop_id INT4 UNSIGNED NOT NULL,
  name VARCHAR(255) NOT NULL CHECK(name != ''),
  description TEXT CHECK(description != ''),
  state ENUM('graveyard', 'wip', 'pending', 'approved', 'completed') NOT NULL,
  checksum BINARY(16) NOT NULL UNIQUE,
  created_by INT8 UNSIGNED NOT NULL REFERENCES Users(id),
  created_at TIMESTAMP NOT NULL DEFAULT NOW(),
  FULLTEXT (name)
);

CREATE TABLE IF NOT EXISTS Courses (
  id INT2 UNSIGNED PRIMARY KEY AUTO_INCREMENT,
  map_id INT2 UNSIGNED NOT NULL REFERENCES Maps(id) ON DELETE CASCADE,
  local_id INT2 UNSIGNED NOT NULL,
  name VARCHAR(255) NOT NULL CHECK(name != ''),
  description TEXT CHECK(description != ''),
  UNIQUE (map_id, local_id),
  UNIQUE (map_id, name),
  FULLTEXT (name)
);

CREATE TABLE IF NOT EXISTS CourseMappers (
  course_id INT2 UNSIGNED NOT NULL REFERENCES Courses(id) ON DELETE CASCADE,
  user_id INT8 UNSIGNED NOT NULL REFERENCES Users(id),
  PRIMARY KEY (course_id, user_id)
);

CREATE TABLE IF NOT EXISTS Filters (
  id INT2 UNSIGNED PRIMARY KEY AUTO_INCREMENT,
  course_id INT2 UNSIGNED NOT NULL REFERENCES Courses(id) ON DELETE CASCADE,
  mode ENUM(
    'vanilla',
    'classic',
    'kztimer',
    'simplekz',
    'vanilla-csgo'
  ) NOT NULL,
  nub_tier ENUM(
    'very-easy',
    'easy',
    'medium',
    'advanced',
    'hard',
    'very-hard',
    'extreme',
    'death',
    'unfeasible',
    'impossible'
  ) NOT NULL,
  pro_tier ENUM(
    'very-easy',
    'easy',
    'medium',
    'advanced',
    'hard',
    'very-hard',
    'extreme',
    'death',
    'unfeasible',
    'impossible'
  ) NOT NULL,
  ranked BOOLEAN NOT NULL,
  notes TEXT CHECK(notes != ''),
  UNIQUE (course_id, mode)
);

CREATE TABLE IF NOT EXISTS Players (
  -- SteamID64
  id INT8 UNSIGNED PRIMARY KEY,
  name VARCHAR(255) NOT NULL CHECK(name != ''),
  ip_address INET4 NOT NULL,
  cs2_preferences JSON NOT NULL DEFAULT '{}',
  csgo_preferences JSON NOT NULL DEFAULT '{}',
  rating FLOAT8 NOT NULL DEFAULT 0,
  created_at TIMESTAMP NOT NULL DEFAULT NOW(),
  FULLTEXT (name)
);

CREATE TABLE IF NOT EXISTS Bans (
  id INT4 UNSIGNED PRIMARY KEY,
  player_id INT8 UNSIGNED NOT NULL REFERENCES Players(id) ON DELETE CASCADE,
  player_ip INET4 NOT NULL,
  reason VARCHAR(255) NOT NULL CHECK(reason != ''),
  banned_by INT8 UNSIGNED NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT NOW(),
  expires_at TIMESTAMP NOT NULL CHECK(expires_at > created_at)
);

CREATE TABLE IF NOT EXISTS Unbans (
  id INT4 UNSIGNED PRIMARY KEY REFERENCES Bans(id),
  reason VARCHAR(255) NOT NULL CHECK(reason != ''),
  unbanned_by INT8 UNSIGNED NOT NULL REFERENCES Users(id),
  created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS Jumps (
  id INT4 UNSIGNED PRIMARY KEY AUTO_INCREMENT,
  player_id INT8 UNSIGNED NOT NULL REFERENCES Players(id),
  session_id INT8 UNSIGNED NOT NULL REFERENCES ServerSessions(id),
  mode ENUM(
    'vanilla',
    'classic',
    'kztimer',
    'simplekz',
    'vanilla-csgo'
  ) NOT NULL,
  styles INT4 UNSIGNED NOT NULL,
  type ENUM(
    'longjump',
    'bhop',
    'multibhop',
    'weirdjump',
    'ladderjump',
    'ladderhop',
    'jumpbug'
  ) NOT NULL,
  time FLOAT8 NOT NULL CHECK (time > 0),
  strafes INT1 UNSIGNED NOT NULL,
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
  created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS Records (
  id INT4 UNSIGNED PRIMARY KEY AUTO_INCREMENT,
  filter_id INT2 UNSIGNED NOT NULL REFERENCES Filters(id),
  player_id INT8 UNSIGNED NOT NULL REFERENCES Players(id),
  session_id INT8 UNSIGNED NOT NULL REFERENCES ServerSessions(id),
  time FLOAT8 NOT NULL,
  teleports INT4 UNSIGNED NOT NULL,
  styles INT4 UNSIGNED NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS BestRecords (
  filter_id INT2 UNSIGNED NOT NULL REFERENCES Filters(id),
  player_id INT8 UNSIGNED NOT NULL REFERENCES Players(id),
  record_id INT4 UNSIGNED NOT NULL REFERENCES Records(id),
  points FLOAT8 NOT NULL CHECK(points <= 10000),
  PRIMARY KEY (filter_id, player_id)
);

CREATE TABLE IF NOT EXISTS BestProRecords (
  filter_id INT2 UNSIGNED NOT NULL REFERENCES Filters(id),
  player_id INT8 UNSIGNED NOT NULL REFERENCES Players(id),
  record_id INT4 UNSIGNED NOT NULL REFERENCES Records(id),
  points FLOAT8 NOT NULL CHECK(points <= 10000),
  PRIMARY KEY (filter_id, player_id)
);

CREATE TABLE IF NOT EXISTS DistributionParameters (
  filter_id INT2 UNSIGNED PRIMARY KEY REFERENCES Filters(id) ON DELETE CASCADE,
  a FLOAT8 NOT NULL,
  b FLOAT8 NOT NULL,
  loc FLOAT8 NOT NULL,
  scale FLOAT8 NOT NULL,
  top_scale FLOAT8 NOT NULL
);

CREATE TABLE IF NOT EXISTS ProDistributionParameters (
  filter_id INT2 UNSIGNED PRIMARY KEY REFERENCES Filters(id) ON DELETE CASCADE,
  a FLOAT8 NOT NULL,
  b FLOAT8 NOT NULL,
  loc FLOAT8 NOT NULL,
  scale FLOAT8 NOT NULL,
  top_scale FLOAT8 NOT NULL
);

CREATE TABLE IF NOT EXISTS FiltersToRecalculate (
  filter_id INT2 UNSIGNED PRIMARY KEY REFERENCES Filters(id) ON DELETE CASCADE,
  priority INT2 UNSIGNED NOT NULL DEFAULT 0,
  INDEX idx_priority (priority DESC)
);

CREATE TABLE IF NOT EXISTS PlayersToRecalculate (
  player_id INT8 UNSIGNED PRIMARY KEY REFERENCES Players(id) ON DELETE CASCADE,
  priority INT2 UNSIGNED NOT NULL DEFAULT 0,
  INDEX idx_priority (priority DESC)
);
