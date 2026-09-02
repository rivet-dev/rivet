import { DatabaseSync } from "node:sqlite";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
	DefaultResourceLoader,
	SettingsManager,
} from "@earendil-works/pi-coding-agent";
import type { RawAccess } from "rivetkit/db";
import { describe, expect, it } from "vitest";
import { pi } from "../src/index.js";
import { loadPiSession, migratePiActorTables } from "../src/storage.js";

function sqliteAccess(database: DatabaseSync): Pick<RawAccess, "execute"> {
	return {
		async execute<TRow extends Record<string, unknown>>(
			query: string,
			...args: unknown[]
		): Promise<TRow[]> {
			const statements = query
				.split(";")
				.map((statement) => statement.trim())
				.filter(Boolean);
			if (statements.length > 1) {
				database.exec(query);
				return [];
			}
			const statement = database.prepare(query);
			if (statement.columns().length > 0) {
				return statement.all(...(args as any[])) as TRow[];
			}
			statement.run(...(args as any[]));
			return [];
		},
	};
}

describe("Pi actor session lifecycle", () => {
	it("restores the same Pi session after actor sleep", async () => {
		const database = new DatabaseSync(":memory:");
		const access = sqliteAccess(database);
		await migratePiActorTables(access);
		const agentDir = await mkdtemp(join(tmpdir(), "rivet-pi-test-agent-"));
		const background: Promise<unknown>[] = [];
		const loggedErrors: unknown[] = [];
		let receivedShutdown = false;
		const context = {
			actorId: "pi-session-test",
			key: ["test"],
			db: access,
			keepAwake: <T>(promise: Promise<T>) => promise,
			waitUntil: (promise: Promise<unknown>) => background.push(promise),
			broadcast: () => {},
			client: () => ({}),
			log: { error: (error: unknown) => loggedErrors.push(error) },
		} as any;
		const resourceSettings = SettingsManager.inMemory();
		const resourceLoader = new DefaultResourceLoader({
				cwd: "/tmp",
				agentDir,
				settingsManager: resourceSettings,
				extensionFactories: [
					(api) => {
						api.on("session_shutdown", async () => {
							receivedShutdown = true;
						});
					},
				],
				noSkills: true,
				noPromptTemplates: true,
				noThemes: true,
				noContextFiles: true,
			});
		await resourceLoader.reload();
		const definition = pi({
			cwd: "/tmp",
			resourceLoader,
			onSessionEvent: () => {
				throw new Error("hook failure");
			},
		});
		const actions = definition.config.actions as any;

		await actions.setSteeringMode(context, "all");
		await actions.sendCustomMessage(context, {
			customType: "durability-test",
			content: "saved before sleep",
			display: true,
		});
		const beforeSleep = await actions.getSession(context);
		await definition.config.onSleep?.(context);
		expect(receivedShutdown).toBe(true);
		const stored = await loadPiSession(access);
		expect(stored?.settings?.steeringMode).toBe("all");
		expect(stored?.transcript).toContain("saved before sleep");
		const afterWake = await actions.getSession(context);
		expect(afterWake.sessionId).toBe(beforeSleep.sessionId);
		expect(afterWake.steeringMode).toBe("all");
		expect(await actions.getMessages(context)).toEqual(
			expect.arrayContaining([
				expect.objectContaining({ customType: "durability-test" }),
			]),
		);

		await definition.config.onDestroy?.(context);
		await Promise.all(background);
		expect(loggedErrors).toEqual(
			expect.arrayContaining([
				expect.objectContaining({ msg: "Pi session event hook failed" }),
			]),
		);
		database.close();
		await rm(agentDir, { recursive: true, force: true });
	});
});
