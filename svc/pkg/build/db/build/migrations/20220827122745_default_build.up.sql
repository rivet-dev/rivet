DROP INDEX builds_image_tag_key CASCADE;

CREATE TABLE default_builds (
	kind TEXT PRIMARY KEY,
	image_tag TEXT NOT NULL,
	upload_id UUID NOT NULL
);

