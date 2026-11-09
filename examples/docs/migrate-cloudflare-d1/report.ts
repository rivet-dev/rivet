import { createClient } from "rivetkit/client";
import type { registry } from "./directory";

const client = createClient<typeof registry>("http://localhost:6420");

// SELECT COUNT(*) FROM users GROUP BY tenant_id, as a fan-out over actors
const tenants = await client.tenantDirectory
	.getOrCreate(["main"])
	.listTenants();

// Query a few tenants at a time. `get` only reaches existing actors, so a
// report never recreates a tenant that was deleted.
const CONCURRENCY = 10;
const counts: { tenantId: string; users: number }[] = [];
for (let i = 0; i < tenants.length; i += CONCURRENCY) {
	const chunk = tenants.slice(i, i + CONCURRENCY);
	const results = await Promise.all(
		chunk.map(async (tenantId) => ({
			tenantId,
			users: await client.tenantDb.get([tenantId]).countUsers(),
		})),
	);
	counts.push(...results);
}
console.log(counts);
