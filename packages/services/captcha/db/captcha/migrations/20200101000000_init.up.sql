CREATE TABLE captcha_verifications (
	verification_id UUID PRIMARY KEY,
	topic JSONB NOT NULL,
	topic_str STRING NOT NULL,
	remote_address INET NOT NULL,
	complete_ts INT NOT NULL,
	expire_ts TIMESTAMPTZ NOT NULL,
	provider INT NOT NULL,  -- rivet.backend.captcha.CaptchaProvider
	success BOOLEAN NOT NULL DEFAULT false,
	user_id UUID,
	namespace_id UUID,
	INDEX (topic_str, remote_address, complete_ts DESC) WHERE success = true
) WITH (ttl = 'on', ttl_expiration_expression = 'expire_ts', ttl_job_cron = '@hourly');

CREATE TABLE captcha_requests (
	request_id UUID NOT NULL PRIMARY KEY,
	topic JSONB NOT NULL,
	topic_str STRING NOT NULL,
	remote_address INET NOT NULL,
	create_ts INT NOT NULL,
	expire_ts TIMESTAMPTZ NOT NULL,
	user_id UUID,
	namespace_id UUID,
	INDEX (topic_str, remote_address, create_ts DESC)
) WITH (ttl = 'on', ttl_expiration_expression = 'expire_ts', ttl_job_cron = '@hourly');

