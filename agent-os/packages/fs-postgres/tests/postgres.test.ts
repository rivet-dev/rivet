import pg from "pg";
import { afterAll, beforeAll } from "vitest";
import { defineFsDriverTests } from "@rivet-dev/agent-os/test/file-system";
import type { PostgresContainerHandle } from "@rivet-dev/agent-os/test/docker";
import { startPostgresContainer } from "@rivet-dev/agent-os/test/docker";
import { createPostgresBackend } from "../src/index.js";

let postgres: PostgresContainerHandle;
let pool: pg.Pool;

beforeAll(async () => {
	postgres = await startPostgresContainer({ healthTimeout: 60_000 });
	pool = new pg.Pool({ connectionString: postgres.connectionString });
}, 90_000);

afterAll(async () => {
	if (pool) await pool.end();
	if (postgres) await postgres.stop();
});

defineFsDriverTests({
	name: "PostgresBackend",
	createFs: async () => {
		// Use a unique schema per test to avoid cross-test interference.
		const schema = `test_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
		return createPostgresBackend({ pool, schema });
	},
	capabilities: {
		symlinks: true,
		hardLinks: true,
		permissions: true,
		utimes: true,
		truncate: true,
		pread: true,
		mkdir: true,
		removeDir: true,
	},
});
