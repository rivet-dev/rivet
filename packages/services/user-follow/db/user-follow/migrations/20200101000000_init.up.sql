CREATE TABLE user_follows (
    follower_user_id UUID NOT NULL,
    following_user_id UUID NOT NULL,
    create_ts INT NOT NULL,
    PRIMARY KEY (follower_user_id, following_user_id)
);

