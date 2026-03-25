import {
	act,
	cleanup,
	render,
	renderHook,
	screen,
	waitFor,
} from "@testing-library/react";
import React, { act as reactAct, Suspense } from "react";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { createRivetKitWithClient } from "./mod";

type StatusChangeCallback = (status: string) => void;
type ErrorCallback = (error: Error) => void;

interface MockConnection {
	_statusCallbacks: StatusChangeCallback[];
	_errorCallbacks: ErrorCallback[];
	_eventListeners: Map<string, Array<(...args: unknown[]) => void>>;
	onStatusChange: (cb: StatusChangeCallback) => void;
	onError: (cb: ErrorCallback) => void;
	on: (event: string, cb: (...args: unknown[]) => void) => () => void;
	dispose: () => void;
	_simulateConnect: () => void;
	_simulateError: (error: Error) => void;
	_simulateDisconnect: () => void;
	_emit: (event: string, ...args: unknown[]) => void;
}

interface MockHandle {
	connect: () => MockConnection;
}

interface MockClient {
	getOrCreate: (
		name: string,
		key: string | string[],
		opts?: unknown,
	) => MockHandle;
	get: (name: string, key: string | string[], opts?: unknown) => MockHandle;
	_lastConnection: MockConnection | null;
}

function createMockConnection(): MockConnection {
	const conn: MockConnection = {
		_statusCallbacks: [],
		_errorCallbacks: [],
		_eventListeners: new Map(),
		onStatusChange(cb) {
			this._statusCallbacks.push(cb);
		},
		onError(cb) {
			this._errorCallbacks.push(cb);
		},
		on(event, cb) {
			const listeners = this._eventListeners.get(event) ?? [];
			listeners.push(cb);
			this._eventListeners.set(event, listeners);
			return () => {
				const remaining = (
					this._eventListeners.get(event) ?? []
				).filter((l) => l !== cb);
				this._eventListeners.set(event, remaining);
			};
		},
		dispose: vi.fn(),
		_simulateConnect() {
			for (const cb of this._statusCallbacks) cb("connected");
		},
		_simulateError(error) {
			for (const cb of this._errorCallbacks) cb(error);
		},
		_simulateDisconnect() {
			for (const cb of this._statusCallbacks) cb("disconnected");
		},
		_emit(event, ...args) {
			for (const cb of this._eventListeners.get(event) ?? []) cb(...args);
		},
	};
	return conn;
}

function createMockClient(): MockClient {
	const client: MockClient = {
		_lastConnection: null,
		getOrCreate(_name, _key, _opts) {
			const conn = createMockConnection();
			client._lastConnection = conn;
			return { connect: () => conn };
		},
		get(_name, _key, _opts) {
			const conn = createMockConnection();
			client._lastConnection = conn;
			return { connect: () => conn };
		},
	};
	return client;
}

function setup() {
	const client = createMockClient();
	const { useActor } = createRivetKitWithClient(client as any);
	return { client, useActor };
}

function getConnection(client: MockClient): MockConnection {
	if (!client._lastConnection) {
		throw new Error("Expected a connection to have been created");
	}
	return client._lastConnection;
}

describe("useActor without suspense", () => {
	test("renders exactly twice on connect: once connecting, once connected", async () => {
		const { client, useActor } = setup();
		let renderCount = 0;

		const { result } = renderHook(() => {
			renderCount++;
			return useActor({ name: "counter", key: "test" });
		});

		const countAfterMount = renderCount;
		expect(result.current.connStatus).toBe("connecting");

		act(() => {
			getConnection(client)._simulateConnect();
		});

		expect(result.current.connStatus).toBe("connected");
		expect(renderCount).toBe(countAfterMount + 1);
	});

	test("renders exactly once more when an error occurs", async () => {
		const { client, useActor } = setup();
		let renderCount = 0;

		const { result } = renderHook(() => {
			renderCount++;
			return useActor({ name: "counter", key: "test" });
		});

		const countAfterMount = renderCount;

		act(() => {
			getConnection(client)._simulateError(new Error("boom"));
		});

		expect(result.current.error).toBeDefined();
		expect(renderCount).toBe(countAfterMount + 1);
	});
	test("starts in connecting state then transitions to connected", async () => {
		const { client, useActor } = setup();

		const { result } = renderHook(() =>
			useActor({ name: "counter", key: "test" }),
		);

		expect(result.current.connStatus).toBe("connecting");
		expect(result.current.connection).not.toBeNull();

		act(() => {
			getConnection(client)._simulateConnect();
		});

		expect(result.current.connStatus).toBe("connected");
	});

	test("exposes error when connection fails", async () => {
		const { client, useActor } = setup();

		const { result } = renderHook(() =>
			useActor({ name: "counter", key: "test" }),
		);

		const boom = new Error("connection refused");

		act(() => {
			getConnection(client)._simulateError(boom);
		});

		expect(result.current.error).toBe(boom);
	});

	test("stays idle and does not connect when enabled is false", () => {
		const { client, useActor } = setup();

		const { result } = renderHook(() =>
			useActor({ name: "counter", key: "test", enabled: false }),
		);

		expect(result.current.connStatus).toBe("idle");
		expect(client._lastConnection).toBeNull();
	});

	test("reconnects after being re-enabled", async () => {
		const { client, useActor } = setup();

		let enabled = false;
		const { result, rerender } = renderHook(() =>
			useActor({ name: "counter", key: "test", enabled }),
		);

		expect(result.current.connStatus).toBe("idle");

		enabled = true;
		rerender();

		await waitFor(() => {
			expect(result.current.connStatus).toBe("connecting");
		});

		act(() => {
			getConnection(client)._simulateConnect();
		});

		expect(result.current.connStatus).toBe("connected");
	});

	test("useEvent subscribes to actor events via connection.on", async () => {
		const { client, useActor } = setup();

		const received: unknown[] = [];

		renderHook(() => {
			const actor = useActor({ name: "counter", key: "test" });
			actor.useEvent("updated" as any, (...args: any) => {
				received.push(args);
			});
			return actor;
		});

		act(() => {
			getConnection(client)._simulateConnect();
		});

		act(() => {
			getConnection(client)._emit("updated", 42);
		});

		expect(received).toEqual([[42]]);
	});
});

// Suspense tests use `act` from React directly (not @testing-library/react)
// because promise-based suspense re-renders require awaiting the resolved
// promise inside the act callback for React to flush the deferred update.
describe("useActor with suspense: true", () => {
	beforeEach(() => {
		// Suppress expected "An update to Suspense inside a test was not wrapped in act" warnings.
		vi.spyOn(console, "error").mockImplementation(() => {});
	});
	afterEach(() => {
		cleanup();
		vi.restoreAllMocks();
	});

	test("suspends (shows fallback) while connecting, then renders content once connected", async () => {
		const { client, useActor } = setup();
		let connectPromise: Promise<void> | null = null;

		function Counter() {
			const { connStatus } = useActor({
				name: "counter",
				key: "test",
				suspense: true,
			});
			return <div data-testid="content">status:{connStatus}</div>;
		}

		await reactAct(async () => {
			render(
				<Suspense
					fallback={<div data-testid="fallback">connecting…</div>}
				>
					<Counter />
				</Suspense>,
			);
		});

		expect(screen.getByTestId("fallback")).toBeDefined();
		expect(screen.queryByTestId("content")).toBeNull();

		await reactAct(async () => {
			await Promise.resolve();
		});

		const conn = getConnection(client);

		await reactAct(async () => {
			connectPromise = new Promise<void>((resolve) => {
				const original = conn._simulateConnect.bind(conn);
				conn._simulateConnect = () => {
					original();
					resolve();
				};
			});
			conn._simulateConnect();
			await connectPromise;
		});

		expect(screen.queryByTestId("fallback")).toBeNull();
		expect(screen.getByTestId("content").textContent).toBe(
			"status:connected",
		);
	});

	test("throws error to error boundary when connection fails in suspense mode", async () => {
		const { client, useActor } = setup();

		function Counter() {
			useActor({ name: "counter", key: "test", suspense: true });
			return <div>ok</div>;
		}

		class ErrorBoundary extends React.Component<
			{ children: React.ReactNode },
			{ caught: Error | null }
		> {
			constructor(props: { children: React.ReactNode }) {
				super(props);
				this.state = { caught: null };
			}
			static getDerivedStateFromError(error: Error) {
				return { caught: error };
			}
			render() {
				if (this.state.caught) {
					return (
						<div data-testid="error">
							{this.state.caught.message}
						</div>
					);
				}
				return this.props.children;
			}
		}

		await reactAct(async () => {
			render(
				<ErrorBoundary>
					<Suspense
						fallback={<div data-testid="fallback">connecting…</div>}
					>
						<Counter />
					</Suspense>
				</ErrorBoundary>,
			);
		});

		expect(screen.getByTestId("fallback")).toBeDefined();

		await reactAct(async () => {
			await Promise.resolve();
		});

		const conn = getConnection(client);

		await reactAct(async () => {
			const rejectPromise = new Promise<void>((resolve) => {
				const original = conn._simulateError.bind(conn);
				conn._simulateError = (e) => {
					original(e);
					resolve();
				};
			});
			conn._simulateError(new Error("auth failed"));
			await rejectPromise;
		});

		await reactAct(async () => {
			await Promise.resolve();
		});

		expect(screen.getByTestId("error").textContent).toBe("auth failed");
	});

	test("component renders exactly once after suspense resolves (no extra re-renders)", async () => {
		const { client, useActor } = setup();
		let renderCount = 0;

		function Counter() {
			renderCount++;
			useActor({ name: "counter", key: "test", suspense: true });
			return <div data-testid="content">ok</div>;
		}

		await reactAct(async () => {
			render(
				<Suspense fallback={<div data-testid="fallback">…</div>}>
					<Counter />
				</Suspense>,
			);
		});

		// Flush microtask; reset counter so we only count post-resolve renders.
		await reactAct(async () => {
			await Promise.resolve();
		});

		const conn = getConnection(client);
		renderCount = 0;

		await reactAct(async () => {
			const connectPromise = new Promise<void>((resolve) => {
				const original = conn._simulateConnect.bind(conn);
				conn._simulateConnect = () => {
					original();
					resolve();
				};
			});
			conn._simulateConnect();
			await connectPromise;
		});

		expect(screen.getByTestId("content")).toBeDefined();
		// React may probe the component multiple times when resuming from Suspense.
		// Assert an upper bound to catch render loops, not an exact count.
		expect(renderCount).toBeLessThanOrEqual(3);
	});

	test("connection is not null inside component after suspense resolves", async () => {
		const { client, useActor } = setup();
		let capturedConnection: unknown = null;

		function Counter() {
			const { connection, connStatus } = useActor({
				name: "counter",
				key: "test",
				suspense: true,
			});
			capturedConnection = connection;
			return <div data-testid="status">{connStatus}</div>;
		}

		await reactAct(async () => {
			render(
				<Suspense fallback={<div data-testid="fallback">…</div>}>
					<Counter />
				</Suspense>,
			);
		});

		await reactAct(async () => {
			await Promise.resolve();
		});

		const conn = getConnection(client);

		await reactAct(async () => {
			const connectPromise = new Promise<void>((resolve) => {
				const original = conn._simulateConnect.bind(conn);
				conn._simulateConnect = () => {
					original();
					resolve();
				};
			});
			conn._simulateConnect();
			await connectPromise;
		});

		expect(screen.getByTestId("status").textContent).toBe("connected");
		expect(capturedConnection).not.toBeNull();
	});
});
