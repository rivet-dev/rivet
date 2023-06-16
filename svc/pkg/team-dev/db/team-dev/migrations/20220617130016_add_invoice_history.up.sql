CREATE TABLE invoice_history (
    team_id UUID PRIMARY KEY,
    period_start_ts INT NOT NULL,
    period_end_ts INT NOT NULL,
    csv_upload_id UUID NOT NULL,
    pdf_upload_id UUID NOT NULL,
    UNIQUE (team_id, period_start_ts, period_end_ts)
);
