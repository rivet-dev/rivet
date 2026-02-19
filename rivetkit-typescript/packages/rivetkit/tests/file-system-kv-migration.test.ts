import {
	copyFileSync,
	mkdirSync,
	mkdtempSync,
	readFileSync,
	readdirSync,
	rmSync,
} from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { describe, expect, it } from "vitest";
import { importNodeDependencies } from "@/utils/node";
import { FileSystemGlobalState } from "@/drivers/file-system/global-state";
import { loadSqliteRuntime } from "@/drivers/file-system/sqlite-runtime";

const encoder = new TextEncoder();
const decoder = new TextDecoder();
const fixtureStateDir = join(__dirname, "fixtures", "legacy-kv", "state");

function makeStorageFromFixtures(): string {
	const storageRoot = mkdtempSync(join(tmpdir(), "rivetkit-kv-migration-"));

	const stateDir = join(storageRoot, "state");
	const dbDir = join(storageRoot, "databases");
	const alarmsDir = join(storageRoot, "alarms");
	mkdirSync(stateDir, { recursive: true });
	mkdirSync(dbDir, { recursive: true });
	mkdirSync(alarmsDir, { recursive: true });

	for (const fileName of readdirSync(fixtureStateDir)) {
		copyFileSync(join(fixtureStateDir, fileName), join(stateDir, fileName));
	}

	return storageRoot;
}

describe("file-system driver legacy KV startup migration", () => {
	it("migrates legacy actor kvStorage into sqlite databases on startup", async () => {
		importNodeDependencies();
		const storageRoot = makeStorageFromFixtures();
		try {
			const actorOneStatePath = join(storageRoot, "state", "legacy-actor-one");
			const actorOneStateBefore = readFileSync(actorOneStatePath);

			const state = new FileSystemGlobalState({
				persist: true,
				customPath: storageRoot,
				useNativeSqlite: true,
			});

			const alpha = await state.kvBatchGet("legacy-actor-one", [
				encoder.encode("alpha"),
			]);
			expect(alpha[0]).not.toBeNull();
			expect(decoder.decode(alpha[0] ?? new Uint8Array())).toBe("one");

			const prefixed = await state.kvListPrefix(
				"legacy-actor-one",
				encoder.encode("prefix:"),
			);
			expect(prefixed).toHaveLength(2);

			const sqliteRuntime = loadSqliteRuntime();
			const actorTwoDb = sqliteRuntime.open(
				join(storageRoot, "databases", "legacy-actor-two.db"),
			);
			const actorTwoRow = actorTwoDb.get<{ value: Uint8Array | ArrayBuffer }>(
				"SELECT value FROM kv WHERE key = ?",
				[encoder.encode("beta")],
			);
			expect(actorTwoRow).toBeDefined();
			expect(
				decoder.decode(
					(actorTwoRow?.value as Uint8Array | ArrayBuffer) ?? new Uint8Array(),
				),
			).toBe("two");
			actorTwoDb.close();

			// Migration must not mutate legacy state files.
			const actorOneStateAfter = readFileSync(actorOneStatePath);
			expect(Buffer.compare(actorOneStateBefore, actorOneStateAfter)).toBe(0);
		} finally {
			rmSync(storageRoot, { recursive: true, force: true });
		}
	});

	it("does not overwrite sqlite data when database is already populated", async () => {
		importNodeDependencies();
		const storageRoot = makeStorageFromFixtures();
		try {
			const sqliteRuntime = loadSqliteRuntime();
			const actorDbPath = join(storageRoot, "databases", "legacy-actor-one.db");
			const db = sqliteRuntime.open(actorDbPath);
			db.exec(`
				CREATE TABLE IF NOT EXISTS kv (
					key BLOB PRIMARY KEY NOT NULL,
					value BLOB NOT NULL
				)
			`);
			db.run("INSERT OR REPLACE INTO kv (key, value) VALUES (?, ?)", [
				encoder.encode("alpha"),
				encoder.encode("existing"),
			]);
			db.close();

			const state = new FileSystemGlobalState({
				persist: true,
				customPath: storageRoot,
				useNativeSqlite: true,
			});
			void state;
			const checkDb = sqliteRuntime.open(actorDbPath);
			const alpha = checkDb.get<{ value: Uint8Array | ArrayBuffer }>(
				"SELECT value FROM kv WHERE key = ?",
				[encoder.encode("alpha")],
			);
			expect(alpha).toBeDefined();
			expect(
				decoder.decode(
					(alpha?.value as Uint8Array | ArrayBuffer) ?? new Uint8Array(),
				),
			).toBe("existing");
			checkDb.close();
		} finally {
			rmSync(storageRoot, { recursive: true, force: true });
		}
	});
});
