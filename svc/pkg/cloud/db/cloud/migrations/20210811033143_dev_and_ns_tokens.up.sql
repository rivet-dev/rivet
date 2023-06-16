CREATE TABLE game_namespace_public_tokens (
    namespace_id UUID NOT NULL REFERENCES game_namespaces (namespace_id),
    token_session_id UUID NOT NULL,  -- References db-tokens.sessions
    PRIMARY KEY (namespace_id, token_session_id)
);

CREATE TABLE game_namespace_development_tokens (
    namespace_id UUID NOT NULL REFERENCES game_namespaces (namespace_id),
    token_session_id UUID NOT NULL,  -- References db-tokens.sessions
    PRIMARY KEY (namespace_id, token_session_id)
);
