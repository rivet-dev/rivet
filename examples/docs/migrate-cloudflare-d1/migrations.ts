import { actor } from "rivetkit";
import { db } from "rivetkit/db";
import { migrations } from "rivetkit/unstable/migrations";

// Version 1 is the current D1 schema without the tenant_id column. Later
// schema changes are added as new versions. The applied version is recorded
// in a table inside each actor's database.
const migrateTenant = migrations({
	migrations: [
		{
			// The current D1 schema, minus tenant_id
			version: 1,
			sql: `
				CREATE TABLE users (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					email TEXT NOT NULL UNIQUE,
					name TEXT NOT NULL
				);
				CREATE INDEX idx_users_name ON users (name);
			`,
		},
		{
			// A schema change made after the move
			version: 2,
			sql: `
				ALTER TABLE users
				ADD COLUMN order_count INTEGER NOT NULL DEFAULT 0
			`,
		},
	],
});

export const tenantDb = actor({
	// Runs on every start and applies only the versions this actor is missing
	db: db({ onMigrate: migrateTenant }),
	actions: {},
});
