CREATE TABLE game_version_name_reservations (
	game_id UUID NOT NULL,
	version_display_name TEXT NOT NULL,
	create_ts INT NOT NULL,
	PRIMARY KEY (game_id, version_display_name),
	INDEX (game_id, version_display_name, create_ts DESC)
);

