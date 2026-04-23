import { describeDriverMatrix } from "./shared-matrix";
import { describe, expect, test, vi } from "vitest";
import { getHibernatableWebSocketAckState } from "@/common/websocket-test-hooks";
import { setupDriverTest, waitFor } from "./shared-utils";

const HIBERNATABLE_ACK_SETTLE_TIMEOUT_MS = 12_000;

async function waitForJsonMessage(
	ws: WebSocket,
	timeoutMs: number,
): Promise<Record<string, unknown> | undefined> {
	const messagePromise = new Promise<Record<string, unknown> | undefined>(
		(resolve, reject) => {
			ws.addEventListener(
				"message",
				(event: any) => {
					try {
						resolve(JSON.parse(event.data as string));
					} catch {
						resolve(undefined);
					}
				},
				{ once: true },
			);
			ws.addEventListener("close", reject, { once: true });
		},
	);

	return await Promise.race([
		messagePromise,
		new Promise<undefined>((resolve) =>
			setTimeout(() => resolve(undefined), timeoutMs),
		),
	]);
}

async function waitForMatchingJsonMessages(
	ws: WebSocket,
	count: number,
	matcher: (message: Record<string, unknown>) => boolean,
	timeoutMs: number,
): Promise<Array<Record<string, unknown>>> {
	return await new Promise<Array<Record<string, unknown>>>(
		(resolve, reject) => {
			const messages: Array<Record<string, unknown>> = [];
			const timeout = setTimeout(() => {
				cleanup();
				reject(
					new Error(
						`timed out waiting for ${count} matching websocket messages`,
					),
				);
			}, timeoutMs);
			const onMessage = (event: { data: string }) => {
				let parsed: Record<string, unknown> | undefined;
				try {
					parsed = JSON.parse(event.data as string);
				} catch {
					return;
				}
				if (!parsed) {
					return;
				}
				if (!matcher(parsed)) {
					return;
				}
				messages.push(parsed);
				if (messages.length >= count) {
					cleanup();
					resolve(messages);
				}
			};
			const onClose = (event: unknown) => {
				cleanup();
				reject(event);
			};
			const cleanup = () => {
				clearTimeout(timeout);
				ws.removeEventListener(
					"message",
					onMessage as (event: any) => void,
				);
				ws.removeEventListener(
					"close",
					onClose as (event: any) => void,
				);
			};
			ws.addEventListener("message", onMessage as (event: any) => void);
			ws.addEventListener("close", onClose as (event: any) => void, {
				once: true,
			});
		},
	);
}

async function readHibernatableAckState(websocket: WebSocket): Promise<{
	lastSentIndex: number;
	lastAckedIndex: number;
	pendingIndexes: number[];
}> {
	const hookUnavailableErrorPattern =
		/remote hibernatable websocket ack hooks are unavailable/;
	for (let attempt = 0; attempt < 20; attempt += 1) {
		try {
			const state = getHibernatableWebSocketAckState(
				websocket as unknown as any,
			);
			if (state) {
				return state;
			}
		} catch (error) {
			if (
				error instanceof Error &&
				hookUnavailableErrorPattern.test(error.message)
			) {
				await new Promise((resolve) => setTimeout(resolve, 25));
				continue;
			}
			throw error;
		}
	}

	websocket.send(
		JSON.stringify({
			__rivetkitTestHibernatableAckStateV1: true,
		}),
	);
	const message = await waitForJsonMessage(websocket, 1_000);
	expect(message).toBeDefined();
	expect(message?.__rivetkitTestHibernatableAckStateV1).toBe(true);
	const fallbackCounter = websocket as unknown as {
		__rivetFallbackAckProbeCount?: number;
	};
	fallbackCounter.__rivetFallbackAckProbeCount =
		(fallbackCounter.__rivetFallbackAckProbeCount ?? 0) + 1;

	return {
		lastSentIndex: message?.lastSentIndex as number,
		lastAckedIndex: message?.lastAckedIndex as number,
		pendingIndexes: message?.pendingIndexes as number[],
	};
}

describeDriverMatrix("Hibernatable Websocket Protocol", (driverTestConfig) => {
	describe.skipIf(!driverTestConfig.features?.hibernatableWebSocketProtocol)(
		"hibernatable websocket protocol",
		() => {
			test("replays only unacked indexed websocket messages after sleep and wake", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const actor = client.rawWebSocketActor.getOrCreate([
					"hibernatable-replay",
				]);
				const ws = await actor.webSocket();

				try {
					expect(await waitForJsonMessage(ws, 4_000)).toMatchObject({
						type: "welcome",
					});

					const firstProbePromise = waitForMatchingJsonMessages(
						ws,
						1,
						(message) => message.type === "indexedAckProbe",
						1_000,
					);
					ws.send(
						JSON.stringify({
							type: "indexedAckProbe",
							payload: "durable-before-sleep",
						}),
					);
					expect((await firstProbePromise)[0]).toMatchObject({
						type: "indexedAckProbe",
						rivetMessageIndex: 1,
					});

					// Ack propagation is asynchronous through the remote WebSocket transport.
					await vi.waitFor(
						async () => {
							expect(await readHibernatableAckState(ws)).toEqual({
								lastSentIndex: 1,
								lastAckedIndex: 1,
								pendingIndexes: [],
							});
						},
						{
							timeout: HIBERNATABLE_ACK_SETTLE_TIMEOUT_MS,
							interval: 50,
						},
					);
					const replayIndexOffset =
						(
							ws as unknown as {
								__rivetFallbackAckProbeCount?: number;
							}
						).__rivetFallbackAckProbeCount ?? 0;

					const sleepScheduledPromise = waitForMatchingJsonMessages(
						ws,
						1,
						(message) => message.type === "sleepScheduled",
						1_000,
					);
					ws.send(
						JSON.stringify({
							type: "scheduleSleep",
						}),
					);
					await sleepScheduledPromise;
					await waitFor(driverTestConfig, 250);

					const replayedMessagesPromise = waitForMatchingJsonMessages(
						ws,
						2,
						(message) => message.type === "indexedEcho",
						6_000,
					);
					ws.send(
						JSON.stringify({
							type: "indexedEcho",
							payload: "after-sleep-1",
						}),
					);
					ws.send(
						JSON.stringify({
							type: "indexedEcho",
							payload: "after-sleep-2",
						}),
					);

					const replayedIndexes = (await replayedMessagesPromise).map(
						(message) => message.rivetMessageIndex as number,
					);

					expect(replayedIndexes).toEqual([
						3 + replayIndexOffset,
						4 + replayIndexOffset,
					]);

					// Ack propagation is asynchronous through the remote WebSocket transport.
					await vi.waitFor(
						async () => {
							expect(await readHibernatableAckState(ws)).toEqual({
								lastSentIndex: 4 + replayIndexOffset,
								lastAckedIndex: 4 + replayIndexOffset,
								pendingIndexes: [],
							});
						},
						{
							timeout: HIBERNATABLE_ACK_SETTLE_TIMEOUT_MS,
							interval: 50,
						},
					);

					const actorObservedOrderPromise =
						waitForMatchingJsonMessages(
							ws,
							1,
							(message) => message.type === "indexedMessageOrder",
							1_000,
						);
					ws.send(
						JSON.stringify({
							type: "getIndexedMessageOrder",
						}),
					);
					expect((await actorObservedOrderPromise)[0].order).toEqual([
						1,
						3 + replayIndexOffset,
						4 + replayIndexOffset,
					]);
				} finally {
					ws.close();
				}
			}, 20_000);

			test("cleans up stale hibernatable websocket connections on restore", async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const conn = client.fileSystemHibernationCleanupActor
					.getOrCreate()
					.connect();
				let wakeConn: typeof conn | undefined;
				let connDisposed = false;

				try {
					expect(await conn.ping()).toBe("pong");
					await conn.triggerSleep();
					await waitFor(driverTestConfig, 700);

					// Disconnect the original client while the actor is asleep so the
					// persisted websocket metadata is stale on the next wake.
					await conn.dispose();
					connDisposed = true;
					await waitFor(driverTestConfig, 100);

					// Wake the actor through a new connection so restore must clean up
					// the stale persisted websocket from the sleeping generation.
					wakeConn = client.fileSystemHibernationCleanupActor
						.getOrCreate()
						.connect();

					// Restore cleanup runs after the wake connection is accepted.
					await vi.waitFor(
						async () => {
							const counts = await wakeConn!.getCounts();
							expect(counts.sleepCount).toBeGreaterThanOrEqual(1);
							expect(counts.wakeCount).toBeGreaterThanOrEqual(2);
						},
						{ timeout: 5_000, interval: 100 },
					);

					// Restore cleanup runs after the wake connection is accepted.
					await vi.waitFor(
						async () => {
							const disconnectWakeCounts =
								await wakeConn!.getDisconnectWakeCounts();
							expect(disconnectWakeCounts).toEqual([2]);
						},
						{ timeout: 5_000, interval: 100 },
					);
				} finally {
					await wakeConn?.dispose().catch(() => undefined);
					if (!connDisposed) {
						await conn.dispose().catch(() => undefined);
					}
				}
			}, 15_000);
		},
	);
});
