CREATE TABLE game_users (
	game_user_id UUID PRIMARY KEY,
	user_id UUID NOT NULL,
	token_session_id UUID NOT NULL,
	create_ts INT NOT NULL,
	namespace_id UUID NOT NULL,
	deleted_ts INT,
	INDEX (user_id, create_ts DESC),
	INDEX (namespace_id)
);

CREATE TABLE sessions (
	session_id UUID PRIMARY KEY,
	game_user_id UUID NOT NULL REFERENCES game_users,
	start_ts INT NOT NULL,
	refresh_jti UUID NOT NULL,
	INDEX (game_user_id, start_ts DESC)
);

CREATE TABLE links (
	link_id UUID PRIMARY KEY,
	namespace_id UUID NOT NULL,
	token_session_id UUID NOT NULL,
	current_game_user_id UUID NOT NULL REFERENCES game_users,
	new_game_user_id UUID REFERENCES game_users,
	new_game_user_token STRING,
	create_ts INT NOT NULL,
	complete_ts INT,
	cancelled_ts INT,
	INDEX (new_game_user_id)
);

