CREATE TABLE threads (
	thread_id UUID PRIMARY KEY,
	create_ts INT NOT NULL,
	party_party_id UUID UNIQUE,
	direct_user_a_id UUID,
	direct_user_b_id UUID,
	team_team_id UUID UNIQUE,

	-- User
	UNIQUE (direct_user_a_id, direct_user_b_id),
	INDEX (direct_user_b_id)
);

CREATE TABLE messages (
	message_id UUID PRIMARY KEY,
    thread_id UUID NOT NULL REFERENCES threads,
    send_ts INT NOT NULL,
    body BYTES NOT NULL,  -- MessageBody bytes
	sender_user_id UUID,

	-- Optimized for fast reads by thread ID at the expense of slower reads
	INDEX (thread_id, send_ts DESC, message_id DESC) STORING (body),

	-- Optimized for infrequent reads by user ID and heavy writes
	INDEX (sender_user_id, send_ts DESC) USING HASH WHERE sender_user_id IS NOT NULL
);

CREATE TABLE thread_user_settings (
    user_id UUID NOT NULL,
	thread_id UUID NOT NULL REFERENCES threads,
	last_read_ts INT NOT NULL,
	PRIMARY KEY (user_id, thread_id)
);

