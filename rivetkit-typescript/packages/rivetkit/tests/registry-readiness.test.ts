import { afterEach, describe, expect, test, vi } from "vitest";
import { actor } from "@/actor/mod";
import { type RegistryDeps, Registry } from "@/registry";
import type {
	CoreRuntime,
	RegistryHandle,
	RuntimeServeConfig,
} from "@/registry/runtime";

const testActor = actor({ state: {}, actions: {} });

function deferred<T = void>() {
	let resolve!: (value: T | PromiseLike<T>) => void;
	let reject!: (reason?: unknown) => void;
	const promise = new Promise<T>((res, rej) => {
		resolve = res;
		reject = rej;
	});
	return { promise, resolve, reject };
}

function createRegistry() {
	const ready = deferred();
	const serve = deferred();
	const calls = { build: 0, serve: 0, ready: 0 };
	const runtime = {
		kind: "napi",
		serveRegistry: () => {
			calls.serve += 1;
			return serve.promise;
		},
		waitRegistryReady: () => {
			calls.ready += 1;
			return ready.promise;
		},
		shutdownRegistry: async () => {},
	} as unknown as CoreRuntime;
	const handle = {} as RegistryHandle;
	const buildConfiguredRegistry = async () => {
		calls.build += 1;
		return {
			runtime,
			registry: handle,
			serveConfig: {} as RuntimeServeConfig,
		};
	};
	const registry = new Registry(
		{
			use: { test: testActor },
			startEngine: false,
			noWelcome: true,
		},
		{
			buildConfiguredRegistry:
				buildConfiguredRegistry as RegistryDeps["buildConfiguredRegistry"],
		},
	);
	return { registry, ready, serve, calls };
}

describe("Registry.startAndWait", () => {
	afterEach(() => {
		vi.useRealTimers();
		vi.restoreAllMocks();
	});

	test("shares one cold startup and readiness promise", async () => {
		const { registry, ready, calls } = createRegistry();
		const first = registry.startAndWait();
		const second = registry.startAndWait();

		expect(second).toBe(first);
		await vi.waitFor(() => expect(calls.ready).toBe(1));
		expect(calls).toEqual({ build: 1, serve: 1, ready: 1 });

		ready.resolve();
		await Promise.all([first, second, registry.startAndWait()]);
		expect(calls).toEqual({ build: 1, serve: 1, ready: 1 });
	});

	test("surfaces and retains a startup failure", async () => {
		const { registry, serve, calls } = createRegistry();
		const first = registry.startAndWait();
		await vi.waitFor(() => expect(calls.serve).toBe(1));
		serve.reject(new Error("engine unavailable"));

		await expect(first).rejects.toThrow("engine unavailable");
		const second = registry.startAndWait();
		expect(second).toBe(first);
		await expect(second).rejects.toThrow("engine unavailable");
		expect(calls.build).toBe(1);
	});

	test("rejects startup after shutdown has begun", async () => {
		const { registry, calls } = createRegistry();
		await registry.shutdown();

		await expect(registry.startAndWait()).rejects.toThrow(
			"cannot run after shutdown has begun",
		);
		expect(calls).toEqual({ build: 0, serve: 0, ready: 0 });
	});

	test("rejects the WebAssembly runtime before building it", async () => {
		const buildConfiguredRegistry = vi.fn();
		const registry = new Registry(
			{
				use: { test: testActor },
				runtime: "wasm",
				noWelcome: true,
			},
			{
				buildConfiguredRegistry:
					buildConfiguredRegistry as RegistryDeps["buildConfiguredRegistry"],
			},
		);

		await expect(registry.startAndWait()).rejects.toThrow(
			"requires the native runtime",
		);
		expect(buildConfiguredRegistry).not.toHaveBeenCalled();
	});

	test("bounds registry construction and readiness with one timeout", async () => {
		vi.useFakeTimers();
		const registry = new Registry(
			{ use: { test: testActor }, startEngine: false, noWelcome: true },
			{
				buildConfiguredRegistry: (() => new Promise(() => {})) as RegistryDeps["buildConfiguredRegistry"],
			},
		);

		const readiness = registry.startAndWait();
		const assertion = expect(readiness).rejects.toThrow(
			"did not register with the Engine within 30000ms",
		);
		await vi.advanceTimersByTimeAsync(30_000);
		await assertion;
	});
});
