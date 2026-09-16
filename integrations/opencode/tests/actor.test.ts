import { describe, expect, it, vi } from "vitest";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
// Exercise the published bundle, including upstream import/asset resolution.
import { opencode } from "../dist/index.js";
import { sqliteFixture } from "./sqlite-fixture.js";

describe("OpenCode actor", () => {
	it("loads the published package in plain Node ESM", async () => {
		const { stdout } = await promisify(execFile)(process.execPath, [
			"--input-type=module",
			"-e",
			`import(${JSON.stringify(new URL("../dist/index.js", import.meta.url).href)}).then(m => console.log(typeof m.opencode))`,
		]);
		expect(stdout.trim()).toBe("function");
	});
	it("preserves custom actions, events, state and options", () => {
		const definition = opencode({
			state: { count: 0 },
			actions: { increment: (c: any) => ++c.state.count },
			options: { sleepTimeout: 1234 },
		});
		expect(definition.config.actions).toHaveProperty("session.prompt");
		expect(definition.config.actions).toHaveProperty("permission.reply");
		expect(definition.config.actions).toHaveProperty("increment");
		expect(definition.config.options?.sleepTimeout).toBe(1234);
		expect(() => opencode({ actions: { prompt: () => "override" } })).toThrow(
			"reserved",
		);
	});
	it("migrates real SQLite, exposes native sessions, and recovers after sleep", async () => {
		const { db } = sqliteFixture();
		const definition = opencode({ opencode: { models: { fetch: false } } });
		const config = definition.config as any;
		const context: any = {
			actorId: "opencode-test",
			key: ["test"],
			db,
			client: () => ({}),
			keepAwake: (p: Promise<unknown>) => p,
			broadcast: vi.fn(),
			log: { error: vi.fn() },
		};
		await db.execute(
			"CREATE TABLE _rivet_opencode (id INTEGER PRIMARY KEY, binding TEXT, cwd TEXT NOT NULL)",
		);
		try {
			await config.onWake(context);
			const session = await config.actions.getSession(context);
			expect(session.id).toMatch(/^ses_/);
			await config.actions.session.rename(context, {
				sessionID: session.id,
				title: "Persistent session",
			});
			expect((await config.actions.getSession(context)).title).toBe(
				"Persistent session",
			);
			expect(
				(await config.actions.session.get(context, { sessionID: session.id }))
					.title,
			).toBe("Persistent session");
			await config.onSleep(context);
			await config.onWake(context);
			const recovered = await config.actions.getSession(context);
			expect(recovered.id).toBe(session.id);
			expect(recovered.title).toBe("Persistent session");
			expect(await config.actions.getMessages(context)).toBeDefined();
			expect(
				await config.actions.readEvents(context, {
					sessionID: session.id,
					limit: 10,
				}),
			).toBeInstanceOf(Array);
			await config.actions.session.remove(context, { sessionID: session.id });
			expect((await config.actions.getSession(context)).id).toBe(session.id);
		} finally {
			await config.onDestroy(context);
			await db.close();
		}
	}, 60_000);
});
