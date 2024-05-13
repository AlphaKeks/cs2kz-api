CREATE TABLE IF NOT EXISTS `PluginVersions` (
  `id` INT2 UNSIGNED NOT NULL AUTO_INCREMENT,
  `semver` VARCHAR(32) NOT NULL,
  `git_revision` VARCHAR(255) NOT NULL,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE (`semver`),
  UNIQUE (`git_revision`)
);

CREATE TABLE IF NOT EXISTS `Credentials` (
  `id` INT2 UNSIGNED NOT NULL AUTO_INCREMENT,
  `name` VARCHAR(255) NOT NULL,
  `key` UUID NOT NULL,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `expires_on` TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE (`key`)
);

CREATE TABLE IF NOT EXISTS `Players` (
  `id` INT8 UNSIGNED NOT NULL,
  `name` VARCHAR(255) NOT NULL,
  `ip_address` INET4,
  `permissions` INT8 UNSIGNED NOT NULL DEFAULT 0,
  `game_preferences` JSON NOT NULL DEFAULT '{}',
  `joined_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `last_seen` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`)
);

CREATE TABLE IF NOT EXISTS `Maps` (
  `id` INT2 UNSIGNED NOT NULL AUTO_INCREMENT,
  `name` VARCHAR(64) NOT NULL,
  `description` TEXT,
  `global_status` ENUM('not_global', 'in_testing', 'global') NOT NULL DEFAULT 'not_global',
  `workshop_id` INT4 UNSIGNED NOT NULL,
  `checksum` INT4 UNSIGNED NOT NULL,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  CONSTRAINT `valid_global_status` CHECK(`global_status` BETWEEN -1 AND 1)
);

CREATE TABLE IF NOT EXISTS `Mappers` (
  `map_id` INT2 UNSIGNED NOT NULL,
  `player_id` INT8 UNSIGNED NOT NULL,
  PRIMARY KEY (`map_id`, `player_id`),
  FOREIGN KEY (`map_id`) REFERENCES `Maps` (`id`),
  FOREIGN KEY (`player_id`) REFERENCES `Players` (`id`)
);

CREATE TABLE IF NOT EXISTS `Courses` (
  `id` INT2 UNSIGNED NOT NULL AUTO_INCREMENT,
  `name` VARCHAR(64),
  `description` TEXT,
  `map_id` INT2 UNSIGNED NOT NULL,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`map_id`) REFERENCES `Maps` (`id`)
);

CREATE TABLE IF NOT EXISTS `CourseMappers` (
  `course_id` INT2 UNSIGNED NOT NULL,
  `player_id` INT8 UNSIGNED NOT NULL,
  PRIMARY KEY (`course_id`, `player_id`),
  FOREIGN KEY (`course_id`) REFERENCES `Courses` (`id`),
  FOREIGN KEY (`player_id`) REFERENCES `Players` (`id`)
);

CREATE TABLE IF NOT EXISTS `CourseFilters` (
  `id` INT2 UNSIGNED NOT NULL AUTO_INCREMENT,
  `course_id` INT2 UNSIGNED NOT NULL,
  `mode` ENUM('vanilla', 'classic') NOT NULL,
  `teleports` BOOLEAN NOT NULL,
  `tier` ENUM(
    'very_easy',
    'easy',
    'medium',
    'advanced',
    'hard',
    'very_hard',
    'extreme',
    'death',
    'unfeasible',
    'impossible'
  ) NOT NULL,
  `ranked_status` ENUM('never', 'unranked', 'ranked') NOT NULL,
  `notes` TEXT,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`course_id`) REFERENCES `Courses` (`id`),
  CONSTRAINT `valid_ranked_status` CHECK(
    `tier` <= 'death'
    OR `ranked_status` = 'never'
  ),
  UNIQUE (`course_id`, `mode`, `teleports`)
);

CREATE TABLE IF NOT EXISTS `Servers` (
  `id` INT2 UNSIGNED NOT NULL AUTO_INCREMENT,
  `name` VARCHAR(64) NOT NULL,
  `ip_address` INET4 NOT NULL,
  `port` INT2 UNSIGNED NOT NULL,
  `owned_by` INT8 UNSIGNED NOT NULL,
  `key` UUID,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `last_seen` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`owned_by`) REFERENCES `Players` (`id`),
  UNIQUE (`name`),
  UNIQUE (`ip_address`, `port`),
  UNIQUE (`key`)
);

CREATE TABLE IF NOT EXISTS `Jumpstats` (
  `id` INT8 UNSIGNED NOT NULL AUTO_INCREMENT,
  `jump_type` ENUM(
    'longjump',
    'bhop',
    'multi_bhop',
    'weird_jump',
    'ladder_jump',
    'ladder_hop',
    'jump_bug'
  ) NOT NULL,
  `mode` ENUM('vanilla', 'classic') NOT NULL,
  `strafes` INT1 UNSIGNED NOT NULL,
  `distance` FLOAT4 NOT NULL,
  `sync` FLOAT4 NOT NULL,
  `pre` FLOAT4 NOT NULL,
  `max` FLOAT4 NOT NULL,
  `overlap` FLOAT4 NOT NULL,
  `bad_angles` FLOAT4 NOT NULL,
  `dead_air` FLOAT4 NOT NULL,
  `height` FLOAT4 NOT NULL,
  `airpath` FLOAT4 NOT NULL,
  `deviation` FLOAT4 NOT NULL,
  `average_width` FLOAT4 NOT NULL,
  `airtime` FLOAT4 NOT NULL,
  `player_id` INT8 UNSIGNED NOT NULL,
  `server_id` INT2 UNSIGNED NOT NULL,
  `plugin_version_id` INT2 UNSIGNED NOT NULL,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`player_id`) REFERENCES `Players` (`id`),
  FOREIGN KEY (`server_id`) REFERENCES `Servers` (`id`),
  FOREIGN KEY (`plugin_version_id`) REFERENCES `PluginVersions` (`id`)
);

CREATE TABLE IF NOT EXISTS `SuspiciousJumpstats` LIKE `Jumpstats`;

CREATE TABLE IF NOT EXISTS `CheatedJumpstats` LIKE `Jumpstats`;

CREATE TABLE IF NOT EXISTS `Records` (
  `id` INT8 UNSIGNED NOT NULL AUTO_INCREMENT,
  `filter_id` INT2 UNSIGNED NOT NULL,
  `styles` INT4 UNSIGNED NOT NULL,
  `teleports` INT2 UNSIGNED NOT NULL,
  `time` FLOAT8 NOT NULL,
  `player_id` INT8 UNSIGNED NOT NULL,
  `server_id` INT2 UNSIGNED NOT NULL,
  `plugin_version_id` INT2 UNSIGNED NOT NULL,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`filter_id`) REFERENCES `CourseFilters` (`id`),
  FOREIGN KEY (`player_id`) REFERENCES `Players` (`id`),
  FOREIGN KEY (`server_id`) REFERENCES `Servers` (`id`),
  FOREIGN KEY (`plugin_version_id`) REFERENCES `PluginVersions` (`id`)
);

CREATE TABLE IF NOT EXISTS `SuspiciousRecords` LIKE `Records`;

CREATE TABLE IF NOT EXISTS `CheatedRecords` LIKE `Records`;

CREATE TABLE IF NOT EXISTS `WipedRecords` LIKE `Records`;

CREATE TABLE IF NOT EXISTS `Bans` (
  `id` INT8 UNSIGNED NOT NULL AUTO_INCREMENT,
  `player_id` INT8 UNSIGNED NOT NULL,
  `player_ip` INET4 NOT NULL,
  `server_id` INT2 UNSIGNED,
  `banned_by` INT8 UNSIGNED,
  `reason` TEXT NOT NULL,
  `plugin_version_id` INT2 UNSIGNED NOT NULL,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `expires_on` TIMESTAMP,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`player_id`) REFERENCES `Players` (`id`),
  FOREIGN KEY (`server_id`) REFERENCES `Servers` (`id`),
  FOREIGN KEY (`banned_by`) REFERENCES `Players` (`id`),
  FOREIGN KEY (`plugin_version_id`) REFERENCES `PluginVersions` (`id`)
);

CREATE TABLE IF NOT EXISTS `Unbans` (
  `id` INT8 UNSIGNED NOT NULL AUTO_INCREMENT,
  `ban_id` INT8 UNSIGNED NOT NULL,
  `unbanned_by` INT8 UNSIGNED,
  `reason` TEXT NOT NULL,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`ban_id`) REFERENCES `Bans` (`id`),
  FOREIGN KEY (`unbanned_by`) REFERENCES `Players` (`id`),
  UNIQUE (`ban_id`)
);

CREATE TABLE IF NOT EXISTS `GameSessions` (
  `id` INT8 UNSIGNED NOT NULL AUTO_INCREMENT,
  `player_id` INT8 UNSIGNED NOT NULL,
  `server_id` INT2 UNSIGNED NOT NULL,
  `time_active` INT2 NOT NULL,
  `time_spectating` INT2 NOT NULL,
  `time_afk` INT2 NOT NULL,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`player_id`) REFERENCES `Players` (`id`),
  FOREIGN KEY (`server_id`) REFERENCES `Servers` (`id`)
);

CREATE TABLE IF NOT EXISTS `CourseSessions` (
  `id` INT8 UNSIGNED NOT NULL AUTO_INCREMENT,
  `player_id` INT8 UNSIGNED NOT NULL,
  `course_id` INT2 UNSIGNED NOT NULL,
  `server_id` INT2 UNSIGNED NOT NULL,
  `playtime` INT2 NOT NULL,
  `started_runs` INT2 UNSIGNED NOT NULL,
  `finished_runs` INT2 UNSIGNED NOT NULL,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`player_id`) REFERENCES `Players` (`id`),
  FOREIGN KEY (`course_id`) REFERENCES `Courses` (`id`),
  FOREIGN KEY (`server_id`) REFERENCES `Servers` (`id`)
);

CREATE TABLE IF NOT EXISTS `LoginSessions` (
  `id` UUID NOT NULL,
  `user_id` INT8 UNSIGNED NOT NULL,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `expires_on` TIMESTAMP NOT NULL,
  PRIMARY KEY (`id`),
  FOREIGN KEY (`user_id`) REFERENCES `Players` (`id`)
);
