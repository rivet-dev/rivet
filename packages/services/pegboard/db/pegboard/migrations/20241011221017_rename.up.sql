ALTER TABLE containers
	RENAME TO actors;
ALTER TABLE actors
	RENAME COLUMN container_id TO actor_id;
