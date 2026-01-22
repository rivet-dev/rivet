import { describe, expect, test } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

export function runRawWebSocketTests(driverTestConfig: DriverTestConfig) {
	describe("raw websocket", () => {
		test("should establish raw WebSocket connection", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawWebSocketActor.getOrCreate(["basic"]);

			const ws = await actor.websocket();

			// The WebSocket should already be open since openWebSocket waits for openPromise
			// But we still need to ensure any buffered events are processed
			await new Promise<void>((resolve) => {
				// If already open, resolve immediately
				if (ws.readyState === WebSocket.OPEN) {
					resolve();
				} else {
					// Otherwise wait for open event
					ws.addEventListener(
						"open",
						() => {
							resolve();
						},
						{ once: true },
					);
				}
			});

			// Should receive welcome message
			const welcomeMessage = await new Promise<any>((resolve, reject) => {
				ws.addEventListener(
					"message",
					(event: any) => {
						resolve(JSON.parse(event.data as string));
					},
					{ once: true },
				);
				ws.addEventListener("close", reject);
			});

			expect(welcomeMessage.type).toBe("welcome");
			expect(welcomeMessage.connectionCount).toBe(1);

			ws.close();
		});

		test("should echo messages", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawWebSocketActor.getOrCreate(["echo"]);

			const ws = await actor.websocket();

			// Check if WebSocket is already open
			if (ws.readyState !== WebSocket.OPEN) {
				await new Promise<void>((resolve, reject) => {
					ws.addEventListener("open", () => resolve(), {
						once: true,
					});
					ws.addEventListener("close", reject);
				});
			}

			// Skip welcome message
			await new Promise<void>((resolve, reject) => {
				ws.addEventListener("message", () => resolve(), { once: true });
				ws.addEventListener("close", reject);
			});

			// Send and receive echo
			const testMessage = { test: "data", timestamp: Date.now() };
			ws.send(JSON.stringify(testMessage));

			const echoMessage = await new Promise<any>((resolve, reject) => {
				ws.addEventListener(
					"message",
					(event: any) => {
						resolve(JSON.parse(event.data as string));
					},
					{ once: true },
				);
				ws.addEventListener("close", reject);
			});

			expect(echoMessage).toEqual(testMessage);

			ws.close();
		});

		test("should handle ping/pong protocol", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawWebSocketActor.getOrCreate(["ping"]);

			const ws = await actor.websocket();

			// Check if WebSocket is already open
			if (ws.readyState !== WebSocket.OPEN) {
				await new Promise<void>((resolve, reject) => {
					ws.addEventListener("open", () => resolve(), {
						once: true,
					});
					ws.addEventListener("close", reject);
				});
			}

			// Skip welcome message
			await new Promise<void>((resolve, reject) => {
				ws.addEventListener("message", () => resolve(), { once: true });
				ws.addEventListener("close", reject);
			});

			// Send ping
			ws.send(JSON.stringify({ type: "ping" }));

			const pongMessage = await new Promise<any>((resolve, reject) => {
				ws.addEventListener("message", (event: any) => {
					const data = JSON.parse(event.data as string);
					if (data.type === "pong") {
						resolve(data);
					}
				});
				ws.addEventListener("close", reject);
			});

			expect(pongMessage.type).toBe("pong");
			expect(pongMessage.timestamp).toBeDefined();

			ws.close();
		});

		test("should track stats across connections", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor1 = client.rawWebSocketActor.getOrCreate(["stats"]);

			// Create first connection to ensure actor exists
			const ws1 = await actor1.websocket();
			const ws1MessagePromise = new Promise<void>((resolve, reject) => {
				ws1.addEventListener("message", () => resolve(), {
					once: true,
				});
				ws1.addEventListener("close", reject);
			});

			// Wait for first connection to establish before getting the actor
			await ws1MessagePromise;

			// Now get reference to same actor
			const actor2 = client.rawWebSocketActor.get(["stats"]);
			const ws2 = await actor2.websocket();
			const ws2MessagePromise = new Promise<void>((resolve, reject) => {
				ws2.addEventListener("message", () => resolve(), {
					once: true,
				});
				ws2.addEventListener("close", reject);
			});

			// Wait for welcome messages
			await Promise.all([ws1MessagePromise, ws2MessagePromise]);

			// Send some messages
			const pingPromise = new Promise<any>((resolve, reject) => {
				ws2.addEventListener("message", (event: any) => {
					const data = JSON.parse(event.data as string);
					if (data.type === "pong") {
						resolve(undefined);
					}
				});
				ws2.addEventListener("close", reject);
			});
			ws1.send(JSON.stringify({ data: "test1" }));
			ws1.send(JSON.stringify({ data: "test3" }));
			ws2.send(JSON.stringify({ type: "ping" }));
			await pingPromise;

			// Get stats
			const statsPromise = new Promise<any>((resolve, reject) => {
				ws1.addEventListener("message", (event: any) => {
					const data = JSON.parse(event.data as string);
					if (data.type === "stats") {
						resolve(data);
					}
				});
				ws1.addEventListener("close", reject);
			});
			ws1.send(JSON.stringify({ type: "getStats" }));
			const stats = await statsPromise;
			expect(stats.connectionCount).toBe(2);
			expect(stats.messageCount).toBe(4);

			// Verify via action
			const actionStats = await actor1.getStats();
			expect(actionStats.connectionCount).toBe(2);
			expect(actionStats.messageCount).toBe(4);

			ws1.close();
			ws2.close();
		});

		test("should handle binary data", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawWebSocketBinaryActor.getOrCreate([
				"binary",
			]);

			const ws = await actor.websocket();

			// Check if WebSocket is already open
			if (ws.readyState !== WebSocket.OPEN) {
				await new Promise<void>((resolve, reject) => {
					ws.addEventListener("open", () => resolve(), {
						once: true,
					});
					ws.addEventListener("close", reject);
				});
			}

			// Helper to receive and convert binary message
			const receiveBinaryMessage = async (): Promise<Uint8Array> => {
				const response = await new Promise<ArrayBuffer | Blob>(
					(resolve, reject) => {
						ws.addEventListener(
							"message",
							(event: any) => {
								resolve(event.data);
							},
							{ once: true },
						);
						ws.addEventListener("close", reject);
					},
				);

				// Convert Blob to ArrayBuffer if needed
				const buffer =
					response instanceof Blob
						? await response.arrayBuffer()
						: response;

				return new Uint8Array(buffer);
			};

			// Test 1: Small binary data
			const smallData = new Uint8Array([1, 2, 3, 4, 5]);
			ws.send(smallData);
			const smallReversed = await receiveBinaryMessage();
			expect(Array.from(smallReversed)).toEqual([5, 4, 3, 2, 1]);

			// Test 2: Large binary data (1KB)
			const largeData = new Uint8Array(1024);
			for (let i = 0; i < largeData.length; i++) {
				largeData[i] = i % 256;
			}
			ws.send(largeData);
			const largeReversed = await receiveBinaryMessage();

			// Verify it's reversed correctly
			for (let i = 0; i < largeData.length; i++) {
				expect(largeReversed[i]).toBe(
					largeData[largeData.length - 1 - i],
				);
			}

			ws.close();
		});

		test("should work with custom paths", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawWebSocketActor.getOrCreate(["paths"]);

			const ws = await actor.websocket("custom/path");

			await new Promise<void>((resolve, reject) => {
				ws.addEventListener("open", () => {
					resolve();
				});
				ws.addEventListener("error", reject);
				ws.addEventListener("close", reject);
			});

			// Should still work
			const welcomeMessage = await new Promise<any>((resolve) => {
				ws.addEventListener(
					"message",
					(event: any) => {
						resolve(JSON.parse(event.data as string));
					},
					{ once: true },
				);
			});

			expect(welcomeMessage.type).toBe("welcome");

			ws.close();
		});

		test("should handle connection close properly", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawWebSocketActor.getOrCreate(["close-test"]);

			const ws = await actor.websocket();

			// Check if WebSocket is already open
			if (ws.readyState !== WebSocket.OPEN) {
				await new Promise<void>((resolve, reject) => {
					ws.addEventListener("open", () => resolve(), {
						once: true,
					});
					ws.addEventListener("close", reject);
				});
			}

			// Get initial stats
			const initialStats = await actor.getStats();
			expect(initialStats.connectionCount).toBe(1);

			// Wait for close event on client side
			const closePromise = new Promise<void>((resolve) => {
				ws.addEventListener("close", () => resolve(), { once: true });
			});

			// Close connection
			ws.close();
			await closePromise;

			// Poll getStats until connection count is 0
			let finalStats: any;
			for (let i = 0; i < 20; i++) {
				finalStats = await actor.getStats();
				if (finalStats.connectionCount === 0) {
					break;
				}
				await new Promise((resolve) => setTimeout(resolve, 50));
			}

			// Check stats after close
			expect(finalStats?.connectionCount).toBe(0);
		});

		test("should properly handle onWebSocket open and close events", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawWebSocketActor.getOrCreate([
				"open-close-test",
			]);

			// Create first connection
			const ws1 = await actor.websocket();

			// Wait for open event
			await new Promise<void>((resolve, reject) => {
				ws1.addEventListener("open", () => resolve(), { once: true });
				ws1.addEventListener("close", reject);
			});

			// Wait for welcome message which confirms onWebSocket was called
			const welcome1 = await new Promise<any>((resolve, reject) => {
				ws1.addEventListener(
					"message",
					(event: any) => {
						resolve(JSON.parse(event.data as string));
					},
					{ once: true },
				);
				ws1.addEventListener("close", reject);
			});

			expect(welcome1.type).toBe("welcome");
			expect(welcome1.connectionCount).toBe(1);

			// Create second connection to same actor
			const ws2 = await actor.websocket();

			await new Promise<void>((resolve, reject) => {
				ws2.addEventListener("open", () => resolve(), { once: true });
				ws2.addEventListener("close", reject);
			});

			const welcome2 = await new Promise<any>((resolve, reject) => {
				ws2.addEventListener(
					"message",
					(event: any) => {
						resolve(JSON.parse(event.data as string));
					},
					{ once: true },
				);
				ws2.addEventListener("close", reject);
			});

			expect(welcome2.type).toBe("welcome");
			expect(welcome2.connectionCount).toBe(2);

			// Verify stats
			const midStats = await actor.getStats();
			expect(midStats.connectionCount).toBe(2);

			// Close first connection
			ws1.close();
			await new Promise<void>((resolve) => {
				ws1.addEventListener("close", () => resolve(), { once: true });
			});

			// Poll getStats until connection count decreases to 1
			let afterFirstClose: any;
			for (let i = 0; i < 20; i++) {
				afterFirstClose = await actor.getStats();
				if (afterFirstClose.connectionCount === 1) {
					break;
				}
				await new Promise((resolve) => setTimeout(resolve, 50));
			}

			// Verify connection count decreased
			expect(afterFirstClose?.connectionCount).toBe(1);

			// Close second connection
			ws2.close();
			await new Promise<void>((resolve) => {
				ws2.addEventListener("close", () => resolve(), { once: true });
			});

			// Poll getStats until connection count is 0
			let finalStats: any;
			for (let i = 0; i < 20; i++) {
				finalStats = await actor.getStats();
				if (finalStats.connectionCount === 0) {
					break;
				}
				await new Promise((resolve) => setTimeout(resolve, 50));
			}

			// Verify final state
			expect(finalStats?.connectionCount).toBe(0);
		});

		test("should handle query parameters in websocket paths", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawWebSocketActor.getOrCreate([
				"query-params",
			]);

			// Test WebSocket with query parameters
			const ws = await actor.websocket(
				"api/v1/stream?token=abc123&user=test",
			);

			await new Promise<void>((resolve, reject) => {
				ws.addEventListener("open", () => resolve(), { once: true });
				ws.addEventListener("error", reject);
			});

			const requestInfoPromise = new Promise<any>((resolve, reject) => {
				ws.addEventListener("message", (event: any) => {
					const data = JSON.parse(event.data as string);
					if (data.type === "requestInfo") {
						resolve(data);
					}
				});
				ws.addEventListener("close", reject);
			});

			// Send request to get the request info
			ws.send(JSON.stringify({ type: "getRequestInfo" }));

			const requestInfo = await requestInfoPromise;

			// Verify the path and query parameters were preserved
			expect(requestInfo.url).toContain("api/v1/stream");
			expect(requestInfo.url).toContain("token=abc123");
			expect(requestInfo.url).toContain("user=test");

			ws.close();
		});

		test("should handle query parameters on base websocket path (no subpath)", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.rawWebSocketActor.getOrCreate([
				"base-path-query-params",
			]);

			// Test WebSocket with ONLY query parameters on the base path
			// This tests the case where path is "/websocket?foo=bar" without trailing slash
			const ws = await actor.websocket("?token=secret&session=123");

			await new Promise<void>((resolve, reject) => {
				ws.addEventListener("open", () => resolve(), { once: true });
				ws.addEventListener("error", reject);
				ws.addEventListener("close", (evt: any) => {
					reject(new Error(`WebSocket closed: code=${evt.code} reason=${evt.reason}`));
				});
			});

			const requestInfoPromise = new Promise<any>((resolve, reject) => {
				ws.addEventListener("message", (event: any) => {
					const data = JSON.parse(event.data as string);
					if (data.type === "requestInfo") {
						resolve(data);
					}
				});
				ws.addEventListener("close", reject);
			});

			// Send request to get the request info
			ws.send(JSON.stringify({ type: "getRequestInfo" }));

			const requestInfo = await requestInfoPromise;

			// Verify query parameters were preserved even on base websocket path
			expect(requestInfo.url).toContain("token=secret");
			expect(requestInfo.url).toContain("session=123");

			ws.close();
		});
	});
}
