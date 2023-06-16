ALTER TABLE dev_teams
	DROP COLUMN last_invoice_ts;

DROP TABLE dev_team_billing_schedules;
DROP TABLE invoice_history;

ALTER TABLE dev_teams
	ADD COLUMN create_ts INT NOT NULL DEFAULT 0,

	ADD COLUMN setup_complete_ts INT,
	ADD COLUMN payment_failed_ts INT,
	ADD COLUMN spending_limit_reached_ts INT,
	
	ALTER COLUMN customer_id DROP NOT NULL,
	RENAME COLUMN customer_id TO stripe_customer_id,
	
	ADD COLUMN lago_id UUID,
	ADD COLUMN lago_wallet_id UUID,
	ADD COLUMN plan_code STRING,
	ADD COLUMN subscription_id UUID;

CREATE INDEX ON dev_teams (stripe_customer_id);
