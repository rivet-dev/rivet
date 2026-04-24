import { fileURLToPath } from "node:url";
import { describe, expect, test } from "vitest";
import { describeDriverMatrix } from "./shared-matrix";
import { importActorSnapshot, setupDriverTest } from "./shared-utils";

const SQLITE_SNAPSHOT_DIR = fileURLToPath(
	new URL(
		"../../fixtures/driver-test-suite/snapshots/sqlite-counter-v2_1_x",
		import.meta.url,
	),
);
const SQLITE_SNAPSHOT_KEY = "sqlite-import-snapshot-v2-1-x";

describeDriverMatrix(
	"Actor Import Snapshot Database",
	(driverTestConfig) => {
		describe("Actor Import Snapshot Database Tests", () => {
			test(
				"imports a v2.1.x sqlite snapshot and keeps the database readable",
				async (c) => {
					const { client, endpoint, namespace, token } =
						await setupDriverTest(c, driverTestConfig);

					const importResponse = await importActorSnapshot({
						endpoint,
						namespace,
						token,
						archivePath: SQLITE_SNAPSHOT_DIR,
					});
					expect(importResponse.imported_actors).toBe(1);
					expect(importResponse.skipped_actors).toBe(0);
					expect(importResponse.warnings).toEqual([]);

					const actor = client.sqliteCounter.get([SQLITE_SNAPSHOT_KEY]);
					expect(await actor.getCount()).toBe(5);
					expect(await actor.increment(1)).toBe(6);
					expect(await actor.getCount()).toBe(6);
				},
				60_000,
			);
		});
	},
	{ registryVariants: ["static"] },
);
