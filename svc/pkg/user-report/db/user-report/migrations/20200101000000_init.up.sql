CREATE TABLE user_reports (
    reporter_user_id UUID NOT NULL,
    subject_user_id UUID NOT NULL,
    namespace_id UUID,
    create_ts INT NOT NULL,
    reason STRING
);
