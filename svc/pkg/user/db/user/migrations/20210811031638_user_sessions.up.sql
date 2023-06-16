CREATE TABLE user_tokens (
    user_id UUID NOT NULL REFERENCES users (user_id),
    token_session_id UUID NOT NULL,  -- References db-tokens.sessions
    PRIMARY KEY (user_id, token_session_id)
);