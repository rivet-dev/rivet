CREATE TABLE uploads (
	upload_id UUID PRIMARY KEY NOT NULL,
	bucket STRING NOT NULL,
	content_length INT NOT NULL,
	create_ts INT NOT NULL,
	complete_ts INT,
	user_id UUID,
	deleted_ts INT,
	INDEX (user_id, create_ts DESC)
);

CREATE TABLE upload_files (
	upload_id UUID NOT NULL REFERENCES uploads,
	path STRING NOT NULL,
	mime STRING,
	content_length INT NOT NULL,
	nsfw_score_threshold REAL,
	PRIMARY KEY (upload_id, path)
);

