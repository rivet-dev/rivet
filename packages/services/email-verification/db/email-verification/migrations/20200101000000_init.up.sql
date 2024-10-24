CREATE TABLE verifications (
    verification_id UUID PRIMARY KEY,
    email STRING NOT NULL,
    code STRING NOT NULL,
    create_ts INT NOT NULL,
    expire_ts INT NOT NULL,
    complete_ts INT
);

CREATE TABLE verification_attempts (
	verification_id UUID NOT NULL REFERENCES verifications,
	attempt_id UUID NOT NULL,
	create_ts INT,
	PRIMARY KEY (verification_id, attempt_id)
);

