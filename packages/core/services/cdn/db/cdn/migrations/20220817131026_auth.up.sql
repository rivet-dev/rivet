ALTER TABLE game_namespaces ADD COLUMN auth_type INTEGER DEFAULT 0;

-- The domain column should not be unique, this prevents multiple games from using the same domain and allows
-- for domain hoarding.
DROP INDEX game_namespace_domains_domain_key CASCADE;

CREATE TABLE game_namespace_auth_users (
    namespace_id UUID REFERENCES game_namespaces (namespace_id),
    user_name STRING NOT NULL,
    password STRING NOT NULL, 
    PRIMARY KEY (namespace_id, user_name)
);
