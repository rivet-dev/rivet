CREATE TABLE game_namespace_version_history (
    namespace_id UUID NOT NULL REFERENCES game_namespaces (namespace_id),
    version_id UUID NOT NULL REFERENCES game_versions (version_Id),
    deploy_ts INT NOT NULL,
	PRIMARY KEY (namespace_id, version_id, deploy_ts),
    INDEX (namespace_id)
);
