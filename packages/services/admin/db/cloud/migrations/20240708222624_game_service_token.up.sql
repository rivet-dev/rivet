CREATE TABLE service_cloud_tokens (
    game_id UUID NOT NULL REFERENCES game_configs (game_id),
    token_session_id UUID NOT NULL,  -- References db-tokens.sessions
    PRIMARY KEY (game_id, token_session_id)
)