CREATE TABLE game_namespaces (
    namespace_id UUID PRIMARY KEY,  -- References db-game.versions
    enable_domain_public_auth BOOLEAN DEFAULT TRUE
);
