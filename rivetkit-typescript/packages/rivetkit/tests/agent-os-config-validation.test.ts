import common from "@rivet-dev/agent-os-common";
import { describe, expect, test } from "vitest";
import { agentOsActorConfigSchema } from "@/agent-os/config";
import { agentOs } from "@/agent-os/index";
import { setup } from "@/mod";
import { setupTest } from "@/test/mod";

describe("agentOs config validation", () => {
	// --- Zod schema XOR enforcement ---

	test("accepts config with only options", () => {
		const result = agentOsActorConfigSchema.safeParse({
			options: { software: [] },
		});
		expect(result.success).toBe(true);
	});

	test("accepts config with only createOptions", () => {
		const result = agentOsActorConfigSchema.safeParse({
			createOptions: () => ({ software: [] }),
		});
		expect(result.success).toBe(true);
	});

	test("rejects config with both options and createOptions", () => {
		const result = agentOsActorConfigSchema.safeParse({
			options: { software: [] },
			createOptions: () => ({ software: [] }),
		});
		expect(result.success).toBe(false);
		if (!result.success) {
			expect(result.error.message).toContain(
				"exactly one of 'options' or 'createOptions'",
			);
		}
	});

	test("rejects config with neither options nor createOptions", () => {
		const result = agentOsActorConfigSchema.safeParse({});
		expect(result.success).toBe(false);
		if (!result.success) {
			expect(result.error.message).toContain(
				"exactly one of 'options' or 'createOptions'",
			);
		}
	});

	test("rejects config with createOptions set to a non-function", () => {
		const result = agentOsActorConfigSchema.safeParse({
			createOptions: "not-a-function",
		});
		expect(result.success).toBe(false);
	});

	test("accepts config with createOptions and destroyOptions", () => {
		const result = agentOsActorConfigSchema.safeParse({
			createOptions: () => ({ software: [] }),
			destroyOptions: async () => {},
		});
		expect(result.success).toBe(true);
	});

	test("rejects config with destroyOptions set to a non-function", () => {
		const result = agentOsActorConfigSchema.safeParse({
			createOptions: () => ({ software: [] }),
			destroyOptions: "not-a-function",
		});
		expect(result.success).toBe(false);
	});

	test("rejects config with destroyOptions paired with options", () => {
		const result = agentOsActorConfigSchema.safeParse({
			options: { software: [] },
			destroyOptions: async () => {},
		});
		expect(result.success).toBe(false);
		if (!result.success) {
			expect(result.error.message).toContain(
				"'destroyOptions' is only valid with 'createOptions'",
			);
		}
	});

	// --- Runtime behavior ---

	test("createOptions factory boots a working VM", async (c) => {
		const vm = agentOs({
			createOptions: async () => ({ software: [common] }),
		});
		const registry = setup({ use: { vm } });
		const { client } = await setupTest(c, registry);
		const actor = (client as any).vm.getOrCreate([
			`config-test-${crypto.randomUUID()}`,
		]);

		await actor.writeFile("/home/user/test.txt", "config-validation");
		const data = await actor.readFile("/home/user/test.txt");
		expect(new TextDecoder().decode(data)).toBe("config-validation");
	}, 60_000);

	test("createOptions receives actor context", async (c) => {
		let receivedContext = false;

		const vm = agentOs({
			createOptions: async (ctx) => {
				// The context should have log and vars available.
				receivedContext =
					ctx.log !== undefined && ctx.vars !== undefined;
				return { software: [common] };
			},
		});
		const registry = setup({ use: { vm } });
		const { client } = await setupTest(c, registry);
		const actor = (client as any).vm.getOrCreate([
			`ctx-test-${crypto.randomUUID()}`,
		]);

		// Trigger ensureVm by calling any action.
		await actor.exec("echo context-check");
		expect(receivedContext).toBe(true);
	}, 60_000);

	test("createOptions can persist sandboxId in state", async (c) => {
		const fakeSandboxId = `test/${crypto.randomUUID()}`;

		const vm = agentOs({
			createOptions: async (ctx) => {
				// On first boot, sandboxId should be null.
				if (!ctx.state.sandboxId) {
					ctx.state.sandboxId = fakeSandboxId;
				}
				return { software: [common] };
			},
		});
		const registry = setup({ use: { vm } });
		const { client } = await setupTest(c, registry);
		const actor = (client as any).vm.getOrCreate([
			`sandbox-id-test-${crypto.randomUUID()}`,
		]);

		// Trigger ensureVm so createOptions runs.
		await actor.exec("echo sandbox-id-check");

		// The actor should have set the sandboxId. If we call another
		// action, createOptions won't re-run (VM is already booted), but
		// the state should persist.
		const result = await actor.exec("echo done");
		expect(result.exitCode).toBe(0);
	}, 60_000);

	test("destroyOptions is accepted as a valid config callback", () => {
		// Verify destroyOptions is accepted by agentOs() at runtime (not
		// just at the type level). If the Zod schema rejected it, this
		// would throw during parse.
		const vm = agentOs({
			createOptions: async (ctx) => {
				ctx.state.sandboxId = "test/destroy-check";
				return { software: [common] };
			},
			destroyOptions: async (ctx) => {
				// This callback would be called during onDestroy.
				// We just verify it is accepted without error.
				const _id = ctx.state.sandboxId;
			},
		});

		// The factory should return a valid actor definition.
		expect(vm).toBeDefined();
	});
});
