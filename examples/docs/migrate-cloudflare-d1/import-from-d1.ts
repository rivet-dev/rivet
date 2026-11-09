import { readFileSync } from "node:fs";
import { createClient } from "rivetkit/client";
import type { registry, User } from "./tenant";

// Export the rows from D1 first:
//
//   npx wrangler d1 execute my-database --remote --json \
//     --command "SELECT id, tenant_id, email, name FROM users" > users.json
interface D1User {
	id: number;
	tenant_id: string;
	email: string;
	name: string;
}

const [{ results }] = JSON.parse(readFileSync("users.json", "utf8")) as {
	results: D1User[];
}[];

// Group the shared table by tenant, the same key the actors use
const byTenant = new Map<string, D1User[]>();
for (const row of results) {
	const rows = byTenant.get(row.tenant_id) ?? [];
	rows.push(row);
	byTenant.set(row.tenant_id, rows);
}

const client = createClient<typeof registry>("http://localhost:6420");

// An action call's arguments must fit in maxIncomingMessageSize (64 KiB by
// default), so split each tenant's rows into batches by encoded size.
const MAX_BATCH_BYTES = 48 * 1024;

function batchBySize(users: User[]): User[][] {
	const batches: User[][] = [];
	let batch: User[] = [];
	let bytes = 0;
	for (const user of users) {
		const size = JSON.stringify(user).length;
		if (batch.length > 0 && bytes + size > MAX_BATCH_BYTES) {
			batches.push(batch);
			batch = [];
			bytes = 0;
		}
		batch.push(user);
		bytes += size;
	}
	if (batch.length > 0) batches.push(batch);
	return batches;
}

for (const [tenantId, rows] of byTenant) {
	const tenant = client.tenantDb.getOrCreate([tenantId]);
	const users = rows.map(({ id, email, name }) => ({ id, email, name }));
	for (const batch of batchBySize(users)) {
		await tenant.importUsers(batch);
	}
}
