-- Modules
CREATE TABLE modules (
	module_id UUID PRIMARY KEY,
	name_id STRING NOT NULL,
	team_id UUID NOT NULL,  -- References db-team.teams
	create_ts INT NOT NULL,
	publicity INT NOT NULL,
	UNIQUE INDEX (team_id, name_id ASC)
);

-- Version
CREATE TABLE versions (
	module_version_id UUID PRIMARY KEY,
	module_id UUID NOT NULL REFERENCES modules (module_id),
	create_ts INT NOT NULL,

	major INT NOT NULL,
	minor INT NOT NULL,
	patch INT NOT NULL,

	INDEX (module_id, create_ts ASC),
	INDEX (module_id, major ASC, minor ASC, patch ASC)
);

CREATE TABLE versions_image_docker (
	module_version_id UUID PRIMARY KEY REFERENCES versions (module_version_id),
	image_tag STRING NOT NULL
);

-- Function
CREATE TABLE functions (
	module_version_id UUID NOT NULL REFERENCES versions (module_version_id),
	name STRING NOT NULL,
	parameter_schema STRING NOT NULL,
	response_schema STRING NOT NULL,
	PRIMARY KEY (module_version_id, name)
);

CREATE TABLE functions_callable (
	module_version_id UUID NOT NULL REFERENCES versions (module_version_id),
	name STRING NOT NULL,
	PRIMARY KEY (module_version_id, name),
	FOREIGN KEY (module_version_id, name) REFERENCES functions (module_version_id, name)
);

-- Game version
CREATE TABLE game_versions (
	version_id UUID PRIMARY KEY  -- References db-game.versions
);

CREATE TABLE game_version_modules (
	version_id UUID NOT NULL REFERENCES game_versions (version_id),
	module_version_id UUID NOT NULL REFERENCES versions (module_version_id),
	PRIMARY KEY (version_id, module_version_id)
);

CREATE TABLE game_versions_runtime_fly (
	version_id UUID NOT NULL,
	module_version_id UUID NOT NULL,
	vm_size STRING NOT NULL,
	PRIMARY KEY (version_id, module_version_id),
	FOREIGN KEY (version_id, module_version_id) REFERENCES game_version_modules (version_id, module_version_id)
);

