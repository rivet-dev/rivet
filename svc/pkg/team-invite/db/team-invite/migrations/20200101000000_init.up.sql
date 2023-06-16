CREATE TABLE invitations (
	code STRING PRIMARY KEY,
	team_id UUID NOT NULL,
	create_ts INT NOT NULL,
	expire_ts INT,
	max_use_count INT,
	use_counter INT NOT NULL DEFAULT 0,
	revoke_ts INT,
	INDEX (team_id, create_ts DESC)
);

CREATE TABLE invitation_uses (
	code STRING NOT NULL REFERENCES invitations,
	user_id UUID NOT NULL,
	create_ts INT NOT NULL,
	PRIMARY KEY (code, create_ts, user_id)
);


