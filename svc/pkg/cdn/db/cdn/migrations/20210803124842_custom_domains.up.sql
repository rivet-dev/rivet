CREATE TABLE game_namespace_domains (
    namespace_id UUID REFERENCES game_namespaces (namespace_id),
    domain STRING NOT NULL UNIQUE,
    PRIMARY KEY (namespace_id, domain)
);

