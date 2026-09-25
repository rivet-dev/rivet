import { afterEach, describe, expect, test, vi } from "vitest";
import { createClientWithDriver } from "../src/client/client";
import { jsonParseCompat, jsonStringifyCompat } from "../src/common/encoding";
import type { UniversalWebSocket } from "../src/common/websocket-interface";
import type { EngineControlClient } from "../src/engine-client/driver";

type SocketListener = (event: any) => void;

class FakeWebSocket {
	readonly CONNECTING = 0 as const;
	readonly OPEN = 1 as const;
	readonly CLOSING = 2 as const;
	readonly CLOSED = 3 as const;

	readyState: number = this.CONNECTING;
	binaryType = "blob";
	bufferedAmount = 0;
	extensions = "";
	protocol = "";
	url = "ws://test.invalid";

	#listeners = new Map<string, Set<SocketListener>>();

	addEventListener(type: string, listener: SocketListener): void {
		const listeners = this.#listeners.get(type) ?? new Set();
		listeners.add(listener);
		this.#listeners.set(type, listeners);
	}

	removeEventListener(type: string, listener: SocketListener): void {
		this.#listeners.get(type)?.delete(listener);
	}

	dispatchEvent(event: { type: string }): boolean {
		this.#emit(event.type, event);
		return true;
	}

	send(data: string | ArrayBufferLike | Blob | ArrayBufferView): void {
		if (typeof data !== "string") {
			return;
		}

		const message = jsonParseCompat(data);
		if (message.body?.tag !== "ActionRequest") {
			return;
		}

		setTimeout(() => {
			this.#emit("message", {
				data: jsonStringifyCompat({
					body: {
						tag: "ActionResponse",
						val: {
							id: message.body.val.id,
							output: "queued action completed",
						},
					},
				}),
			});
		}, 0);
	}

	close(code = 1000, reason = ""): void {
		if (this.readyState === this.CLOSED) {
			return;
		}
		this.readyState = this.CLOSED;
		this.#emit("close", { code, reason, wasClean: code === 1000 });
	}

	accept(actorId: string, connectionId: string): void {
		this.readyState = this.OPEN;
		this.#emit("open", { type: "open" });
		this.#emit("message", {
			data: jsonStringifyCompat({
				body: {
					tag: "Init",
					val: { actorId, connectionId },
				},
			}),
		});
	}

	failOpaqueHandshake(): void {
		this.readyState = this.CLOSED;
		this.#emit("error", { type: "error" });
		this.#emit("close", { code: 1006, reason: "", wasClean: false });
	}

	rejectWithStructuredError(group: string, code: string): void {
		this.readyState = this.OPEN;
		this.#emit("open", { type: "open" });
		this.#emit("message", {
			data: jsonStringifyCompat({
				body: {
					tag: "Error",
					val: {
						group,
						code,
						message: "connection rejected",
						metadata: null,
						actionId: null,
					},
				},
			}),
		});
	}

	#emit(type: string, event: any): void {
		for (const listener of this.#listeners.get(type) ?? []) {
			listener(event);
		}
	}
}

const clients: Array<{ dispose(): Promise<void> }> = [];

afterEach(async () => {
	for (const client of clients.splice(0)) {
		await client.dispose();
	}
});

describe("actor connection opening retries", () => {
	test("recovers after an opaque failed reconnect handshake", async () => {
		const sockets: FakeWebSocket[] = [];
		let attempts = 0;
		const driver = {
			openWebSocket: async () => {
				attempts += 1;
				const attempt = attempts;
				const socket = new FakeWebSocket();
				sockets.push(socket);

				setTimeout(
					() => {
						if (attempt === 2) {
							socket.failOpaqueHandshake();
						} else {
							socket.accept("actor-123", `conn-${attempt}`);
						}
					},
					attempt === 2 ? 25 : 0,
				);

				return socket as unknown as UniversalWebSocket;
			},
		} as EngineControlClient;
		const client = createClientWithDriver(driver, { encoding: "json" });
		clients.push(client);
		const conn = client.getForId("test-actor", "actor-123").connect();
		const statuses: string[] = [];
		const errors: unknown[] = [];
		conn.onStatusChange((status) => statuses.push(status));
		conn.onError((error) => errors.push(error));

		await conn.ready;
		expect(conn.connStatus).toBe("connected");
		await new Promise((resolve) => setTimeout(resolve, 0));

		sockets[0].close(1006);
		await vi.waitFor(() => expect(attempts).toBe(2));

		const queuedAction = conn.action({ name: "queued", args: [] });

		await vi.waitFor(() => expect(attempts).toBe(3), { timeout: 2_000 });
		await expect(queuedAction).resolves.toBe("queued action completed");
		await vi.waitFor(() => expect(conn.connStatus).toBe("connected"));

		expect(statuses).toEqual(
			expect.arrayContaining(["disconnected", "connecting", "connected"]),
		);
		expect(errors).toEqual([]);
	});

	test("keeps structured permanent opening failures terminal", async () => {
		let attempts = 0;
		const driver = {
			openWebSocket: async () => {
				attempts += 1;
				const socket = new FakeWebSocket();
				setTimeout(
					() =>
						socket.rejectWithStructuredError(
							"auth",
							"unauthorized",
						),
					0,
				);
				return socket as unknown as UniversalWebSocket;
			},
		} as EngineControlClient;
		const client = createClientWithDriver(driver, { encoding: "json" });
		clients.push(client);
		const conn = client.getForId("test-actor", "actor-123").connect();
		const errors: Array<{ group: string; code: string }> = [];
		conn.onError((error) => errors.push(error));

		await vi.waitFor(() => expect(conn.connStatus).toBe("idle"));
		await new Promise((resolve) => setTimeout(resolve, 600));

		expect(attempts).toBe(1);
		expect(errors.length).toBeGreaterThanOrEqual(1);
		expect(
			errors.every(
				(error) =>
					error.group === "auth" && error.code === "unauthorized",
			),
		).toBe(true);
	});
});
