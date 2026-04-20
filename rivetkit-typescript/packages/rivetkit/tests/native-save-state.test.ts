import * as cbor from "cbor-x";
import { afterEach, describe, expect, test, vi } from "vitest";
import type {
	ActorContext as NativeActorContext,
	ConnHandle as NativeConnHandle,
} from "@rivetkit/rivetkit-napi";
import {
	buildNativeFactory,
	NativeActorContextAdapter,
	resetNativePersistStateForTest,
} from "@/registry/native";
import { actor, setup } from "@/mod";

function createMockNativeContext(
	actorId: string,
	options?: {
		conns?: NativeConnHandle[];
		saveState?: () => Promise<void>;
		queueHibernationRemoval?: (connId: string) => void;
		hasPendingHibernationChanges?: () => boolean;
		takePendingHibernationChanges?: () => string[];
	},
) {
	return {
		actorId: vi.fn(() => actorId),
		state: vi.fn(() => Buffer.from(cbor.encode(undefined))),
		requestSave: vi.fn(),
		requestSaveWithin: vi.fn(),
		conns: vi.fn(() => options?.conns ?? []),
		queueHibernationRemoval: vi.fn((connId: string) =>
			options?.queueHibernationRemoval?.(connId),
		),
		hasPendingHibernationChanges: vi.fn(
			() => options?.hasPendingHibernationChanges?.() ?? false,
		),
		takePendingHibernationChanges: vi.fn(
			() => options?.takePendingHibernationChanges?.() ?? [],
		),
		saveState: vi.fn(() => options?.saveState?.() ?? Promise.resolve()),
		setVars: vi.fn(),
		setInOnStateChangeCallback: vi.fn(),
	} as unknown as NativeActorContext & {
		state: ReturnType<typeof vi.fn>;
		requestSave: ReturnType<typeof vi.fn>;
		requestSaveWithin: ReturnType<typeof vi.fn>;
		conns: ReturnType<typeof vi.fn>;
		queueHibernationRemoval: ReturnType<typeof vi.fn>;
		hasPendingHibernationChanges: ReturnType<typeof vi.fn>;
		takePendingHibernationChanges: ReturnType<typeof vi.fn>;
		saveState: ReturnType<typeof vi.fn>;
	};
}

function createMockNativeConn(
	connId: string,
	options?: {
		isHibernatable?: boolean;
	},
) {
	return {
		id: vi.fn(() => connId),
		isHibernatable: vi.fn(() => options?.isHibernatable ?? true),
	} as unknown as NativeConnHandle;
}

function createMockBindings() {
	return {
		pollCancelToken: vi.fn(() => false),
	};
}

function captureFactoryCallbacks(definition: ReturnType<typeof actor>) {
	const registryConfig = setup({
		use: {
			testActor: definition,
		},
		endpoint: "http://127.0.0.1:1",
		namespace: "test",
		token: "dev",
		envoy: {
			poolName: "default",
		},
	}).parseConfig();

	let capturedCallbacks: Record<string, unknown> | undefined;
	class FakeNapiActorFactory {
		constructor(callbacks: Record<string, unknown>) {
			capturedCallbacks = callbacks;
		}
	}

	buildNativeFactory(
		{
			NapiActorFactory: FakeNapiActorFactory,
		} as never,
		registryConfig,
		definition,
	);

	return capturedCallbacks ?? {};
}

describe("native saveState adapter", () => {
	const actorIds = new Set<string>();

	afterEach(() => {
		for (const actorId of actorIds) {
			resetNativePersistStateForTest(actorId);
		}
		actorIds.clear();
	});

	test("saveState({ immediate: true }) waits for the native durable write", async () => {
		const actorId = `native-save-${crypto.randomUUID()}`;
		actorIds.add(actorId);

		let resolveSave: (() => void) | undefined;
		const saveCommitted = new Promise<void>((resolve) => {
			resolveSave = resolve;
		});
		const nativeCtx = createMockNativeContext(actorId, {
			saveState: () => saveCommitted,
		});
		const actorCtx = new NativeActorContextAdapter(
			createMockBindings() as never,
			nativeCtx,
		);

		actorCtx.state = { count: 1 };
		nativeCtx.requestSave.mockClear();

		let resolved = false;
		const savePromise = actorCtx.saveState({ immediate: true }).then(() => {
			resolved = true;
		});

		await Promise.resolve();

		expect(nativeCtx.saveState).toHaveBeenCalledTimes(1);
		expect(resolved).toBe(false);

		const payload = nativeCtx.saveState.mock.calls[0]?.[0] as {
			state?: Buffer;
			connHibernation: Array<unknown>;
			connHibernationRemoved: string[];
		};
		expect(Buffer.isBuffer(payload.state)).toBe(true);
		expect(cbor.decode(payload.state!)).toEqual({ count: 1 });
		expect(payload.connHibernation).toEqual([]);
		expect(payload.connHibernationRemoved).toEqual([]);

		resolveSave?.();
		await savePromise;

		expect(resolved).toBe(true);
	});

	test("saveState({ maxWait }) requests a bounded deferred save", async () => {
		const actorId = `native-save-${crypto.randomUUID()}`;
		actorIds.add(actorId);

		const nativeCtx = createMockNativeContext(actorId);
		const actorCtx = new NativeActorContextAdapter(
			createMockBindings() as never,
			nativeCtx,
		);

		actorCtx.state = { count: 1 };
		nativeCtx.requestSave.mockClear();

		await actorCtx.saveState({ maxWait: 100 });

		expect(nativeCtx.requestSaveWithin).toHaveBeenCalledWith(100);
		expect(nativeCtx.requestSave).not.toHaveBeenCalled();
		expect(nativeCtx.saveState).not.toHaveBeenCalled();
	});

	test("saveState preserves queued hibernation removals until serialize", async () => {
		const actorId = `native-save-${crypto.randomUUID()}`;
		actorIds.add(actorId);

		const nativeCtx = createMockNativeContext(actorId, {
			hasPendingHibernationChanges: () => true,
			takePendingHibernationChanges: () => ["conn-queued"],
		});
		const actorCtx = new NativeActorContextAdapter(
			createMockBindings() as never,
			nativeCtx,
		);

		await actorCtx.saveState();

		expect(nativeCtx.hasPendingHibernationChanges).toHaveBeenCalledTimes(1);
		expect(nativeCtx.takePendingHibernationChanges).not.toHaveBeenCalled();
		expect(nativeCtx.requestSave).toHaveBeenCalledWith(false);

		const payload = actorCtx.serializeForTick("save");
		expect(payload.connHibernationRemoved).toEqual(["conn-queued"]);
		expect(nativeCtx.takePendingHibernationChanges).toHaveBeenCalledTimes(1);
	});

	test("saveState({ immediate: true }) flushes queued hibernation removals", async () => {
		const actorId = `native-save-${crypto.randomUUID()}`;
		actorIds.add(actorId);

		const nativeCtx = createMockNativeContext(actorId, {
			takePendingHibernationChanges: () => ["conn-1"],
		});
		const actorCtx = new NativeActorContextAdapter(
			createMockBindings() as never,
			nativeCtx,
		);

		await actorCtx.saveState({ immediate: true });

		expect(nativeCtx.saveState).toHaveBeenCalledTimes(1);
		expect(nativeCtx.takePendingHibernationChanges).toHaveBeenCalledTimes(1);
		expect(nativeCtx.saveState.mock.calls[0]?.[0]).toMatchObject({
			connHibernationRemoved: ["conn-1"],
		});
	});

	test("buildNativeFactory wires the serializeState callback", async () => {
		const actorId = `native-save-${crypto.randomUUID()}`;
		actorIds.add(actorId);

		const definition = actor({
			state: { count: 0 },
			actions: {},
		});
		const capturedCallbacks = captureFactoryCallbacks(definition);

		const serializeState = capturedCallbacks?.serializeState;
		expect(typeof serializeState).toBe("function");

		const nativeCtx = createMockNativeContext(actorId);
		nativeCtx.state.mockReturnValue(Buffer.from(cbor.encode({ count: 7 })));
		const payload = await (serializeState as (
			error: unknown,
			payload: { ctx: NativeActorContext; reason: "save" | "inspector" },
		) => Promise<{
			state?: Buffer;
			connHibernation: Array<unknown>;
			connHibernationRemoved: string[];
		}>)(undefined, {
			ctx: nativeCtx,
			reason: "save",
		});

		expect(cbor.decode(payload.state!)).toEqual({ count: 7 });
		expect(payload.connHibernation).toEqual([]);
		expect(payload.connHibernationRemoved).toEqual([]);
	});

	test("serializeState snapshots hibernation removals for inspector without requeueing", async () => {
		const actorId = `native-save-${crypto.randomUUID()}`;
		actorIds.add(actorId);

		const definition = actor({
			state: { count: 0 },
			actions: {},
		});
		const capturedCallbacks = captureFactoryCallbacks(definition);
		const serializeState = capturedCallbacks?.serializeState as (
			error: unknown,
			payload: { ctx: NativeActorContext; reason: "save" | "inspector" },
		) => Promise<{
			state?: Buffer;
			connHibernation: Array<unknown>;
			connHibernationRemoved: string[];
		}>;

		const nativeCtx = createMockNativeContext(actorId, {
			takePendingHibernationChanges: () => ["conn-inspector"],
		});

		const payload = await serializeState(undefined, {
			ctx: nativeCtx,
			reason: "inspector",
		});

		expect(payload.connHibernationRemoved).toEqual(["conn-inspector"]);
	});

	test("explicit conn disconnect queues hibernation removals through native ctx", async () => {
		const actorId = `native-disconnect-${crypto.randomUUID()}`;
		actorIds.add(actorId);

		const nativeConn = {
			...createMockNativeConn("conn-removed"),
			disconnect: vi.fn(() => Promise.resolve()),
			params: vi.fn(() => Buffer.from(cbor.encode(undefined))),
			state: vi.fn(() => Buffer.from(cbor.encode(undefined))),
			send: vi.fn(),
			setState: vi.fn(),
		} as unknown as NativeConnHandle & {
			disconnect: ReturnType<typeof vi.fn>;
		};
		const nativeCtx = createMockNativeContext(actorId, {
			conns: [nativeConn],
		});
		const actorCtx = new NativeActorContextAdapter(
			createMockBindings() as never,
			nativeCtx,
		);
		const connAdapter = actorCtx.conns.get("conn-removed") as {
			disconnect: () => Promise<void>;
		};

		await connAdapter.disconnect();

		expect(nativeCtx.queueHibernationRemoval).toHaveBeenCalledWith(
			"conn-removed",
		);
	});

	test("buildNativeFactory splits startup callbacks into the new callback bag", async () => {
		const actorId = `native-startup-${crypto.randomUUID()}`;
		actorIds.add(actorId);

		const inputs: {
			createStateInput?: unknown;
			onCreateInput?: unknown;
		} = {};
		const definition = actor({
			createState: (_c, input) => {
				inputs.createStateInput = input;
				return { count: (input as { count: number }).count };
			},
			onCreate: (_c, input) => {
				inputs.onCreateInput = input;
			},
			createVars: () => ({ mode: "test" }),
			onWake: () => {},
			actions: {},
		});

		const capturedCallbacks = captureFactoryCallbacks(definition);
		expect(capturedCallbacks).not.toHaveProperty("onInit");
		expect(typeof capturedCallbacks.createState).toBe("function");
		expect(typeof capturedCallbacks.onCreate).toBe("function");
		expect(typeof capturedCallbacks.createVars).toBe("function");
		expect(typeof capturedCallbacks.onBeforeActorStart).toBe("function");
		expect(capturedCallbacks.onWake).toBeUndefined();

		const nativeCtx = createMockNativeContext(actorId);
		const input = Buffer.from(cbor.encode({ count: 3 }));

		const createState = capturedCallbacks.createState as (
			error: unknown,
			payload: { ctx: NativeActorContext; input?: Buffer },
		) => Promise<Buffer>;
		const onCreate = capturedCallbacks.onCreate as (
			error: unknown,
			payload: { ctx: NativeActorContext; input?: Buffer },
		) => Promise<void>;
		const createVars = capturedCallbacks.createVars as (
			error: unknown,
			payload: { ctx: NativeActorContext },
		) => Promise<Buffer>;

		expect(cbor.decode(await createState(undefined, { ctx: nativeCtx, input }))).toEqual({
			count: 3,
		});
		await onCreate(undefined, { ctx: nativeCtx, input });
		expect(cbor.decode(await createVars(undefined, { ctx: nativeCtx }))).toEqual({
			mode: "test",
		});
		expect(inputs).toEqual({
			createStateInput: { count: 3 },
			onCreateInput: { count: 3 },
		});
	});

	test("action callbacks accept null conn payloads", async () => {
		const actorId = `native-action-${crypto.randomUUID()}`;
		actorIds.add(actorId);

		const definition = actor({
			actions: {
				echo: (c, value: number) => ({
					hasConn: "conn" in c,
					value,
				}),
			},
		});

		const capturedCallbacks = captureFactoryCallbacks(definition);
		const action = capturedCallbacks.actions as Record<
			string,
			(
				error: unknown,
				payload: {
					ctx: NativeActorContext;
					conn: null;
					name: string;
					args: Buffer;
				},
			) => Promise<Buffer>
		>;

		const payload = await action.echo(undefined, {
			ctx: createMockNativeContext(actorId),
			conn: null,
			name: "echo",
			args: Buffer.from(cbor.encode([7])),
		});

		expect(cbor.decode(payload)).toEqual({
			hasConn: false,
			value: 7,
		});
	});

	test("static state is cloned per actor instance", async () => {
		const definition = actor({
			state: { count: 0 },
			actions: {},
		});
		const capturedCallbacks = captureFactoryCallbacks(definition);
		const createState = capturedCallbacks.createState as (
			error: unknown,
			payload: { ctx: NativeActorContext; input?: Buffer },
		) => Promise<Buffer>;

		const first = cbor.decode(
			await createState(undefined, {
				ctx: createMockNativeContext(`native-static-a-${crypto.randomUUID()}`),
			}),
		) as { count: number };
		const second = cbor.decode(
			await createState(undefined, {
				ctx: createMockNativeContext(`native-static-b-${crypto.randomUUID()}`),
			}),
		) as { count: number };

		first.count = 99;
		expect(second).toEqual({ count: 0 });
	});
});
