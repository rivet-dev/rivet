CREATE TABLE sessions (
	session_id UUID PRIMARY KEY,
	entitlements BYTES[] NOT NULL,  -- rivet.claims.Entitlement[]
	entitlement_tags INT[] NOT NULL,  -- rivet.claims.Entitlement.kind[]

	-- Expiration mirrored from token with longest expiration
	exp INT NOT NULL,
	ttl_expire_ts TIMESTAMPTZ AS (to_timestamp(exp::float / 1000)) STORED
) WITH (ttl = 'on', ttl_expiration_expression = 'ttl_expire_ts', ttl_job_cron = '@hourly');

CREATE TABLE tokens (
	-- Core JWT data. Entitlements can be acquired from the session.
	jti UUID PRIMARY KEY,
	exp INT NOT NULL,
	iat INT NOT NULL,

	-- Metadata
	refresh_jti UUID,
	session_id UUID NOT NULL,
	issuer STRING NOT NULL,
	user_agent STRING,
	remote_address STRING,
	revoke_ts INT,
	ttl_expire_ts TIMESTAMPTZ AS (to_timestamp(exp::float / 1000)) STORED,

	INDEX (session_id, iat DESC)
) WITH (ttl = 'on', ttl_expiration_expression = 'ttl_expire_ts', ttl_job_cron = '@hourly');

