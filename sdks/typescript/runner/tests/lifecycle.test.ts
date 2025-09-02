// import { describe, it, expect, vi } from "vitest";
// import { Runner } from "@/mod";
// import type { RunnerConfig, ActorConfig } from "@/mod";
// import WebSocket, { type CloseEvent } from "ws";
//
// const RIVET_ENDPOINT = process.env.RIVET_ENDPOINT ?? "http://localhost:6420";
// const RIVET_ENDPOINT_WS = RIVET_ENDPOINT.replace("http://", "ws://").replace(
// 	"https://",
// 	"wss://",
// );
//
// async function createActor(
// 	namespaceName: string,
// 	runnerNameSelector: string,
// 	durable: boolean,
// 	name?: string,
// ): Promise<any> {
// 	const response = await fetch(
// 		`${RIVET_ENDPOINT}/actors?namespace=${namespaceName}`,
// 		{
// 			method: "POST",
// 			headers: {
// 				"Content-Type": "application/json",
// 			},
// 			body: JSON.stringify({
// 				name: name ?? "thingy",
// 				input: btoa("hello"),
// 				runner_name_selector: runnerNameSelector,
// 				durable,
// 			}),
// 		},
// 	);
//
// 	if (!response.ok) {
// 		throw new Error(
// 			`Failed to create actor: ${response.status} ${response.statusText}\n${await response.text()}`,
// 		);
// 	}
//
// 	return response.json();
// }
//
// async function destroyActor(
// 	namespaceName: string,
// 	actorId: string,
// ): Promise<void> {
// 	const response = await fetch(
// 		`${RIVET_ENDPOINT}/actors/${actorId}?namespace=${namespaceName}`,
// 		{
// 			method: "DELETE",
// 		},
// 	);
//
// 	if (!response.ok) {
// 		throw new Error(
// 			`Failed to delete actor: ${response.status} ${response.statusText}\n${await response.text()}`,
// 		);
// 	}
// }
//
// async function createNamespace(
// 	name: string,
// 	displayName: string,
// ): Promise<any> {
// 	const response = await fetch(`${RIVET_ENDPOINT}/namespaces`, {
// 		method: "POST",
// 		headers: {
// 			"Content-Type": "application/json",
// 		},
// 		body: JSON.stringify({
// 			name,
// 			display_name: displayName,
// 		}),
// 	});
//
// 	if (!response.ok) {
// 		console.warn(
// 			`Failed to create namespace: ${response.status} ${response.statusText}\n${await response.text()}`,
// 		);
// 	}
// }
//
// async function getActorNames(namespaceName: string): Promise<any> {
// 	const response = await fetch(
// 		`${RIVET_ENDPOINT}/actors/names?namespace=${namespaceName}`,
// 		{
// 			method: "GET",
// 			headers: {
// 				"Content-Type": "application/json",
// 			},
// 		},
// 	);
//
// 	if (!response.ok) {
// 		throw new Error(
// 			`Failed to get actor names: ${response.status} ${response.statusText}\n${await response.text()}`,
// 		);
// 	}
//
// 	return await response.json();
// }
//
// describe("Runner E2E", () => {
// 	it("performs end-to-end actor lifecycle", async () => {
// 		const namespaceName = `test-${Math.floor(Math.random() * 10000)}`;
// 		const runnerName = "test-runner";
// 		const prepopulateActorNames: string[] = Array.from(
// 			{ length: 8 },
// 			() =>
// 				`actor-${Math.random().toString(36).substring(2, 10)}-${Date.now()}`,
// 		);
// 		let runnerStarted = Promise.withResolvers();
// 		let websocketOpen = Promise.withResolvers();
// 		let websocketClosed = Promise.withResolvers();
// 		let runner: Runner | null = null;
// 		const actorWebSockets = new Map<string, WebSocket>();
//
// 		// Use objects to hold the current promise resolvers so callbacks always get the latest
// 		const startedRef = { current: Promise.withResolvers() };
// 		const stoppedRef = { current: Promise.withResolvers() };
//
// 		const config: RunnerConfig = {
// 			version: 1,
// 			endpoint: RIVET_ENDPOINT,
// 			namespace: namespaceName,
// 			addresses: { main: { host: "127.0.0.1", port: 5051 } },
// 			totalSlots: 100,
// 			runnerName: runnerName,
// 			runnerKey: "default",
// 			prepopulateActorNames,
// 			onConnected: () => {
// 				runnerStarted.resolve(undefined);
// 			},
// 			onDisconnected: () => { },
// 			fetch: async (actorId: string, request: Request) => {
// 				const url = new URL(request.url);
// 				if (url.pathname === "/ping") {
// 					// Return the actor ID in response
// 					return new Response(
// 						JSON.stringify({
// 							actorId,
// 							status: "ok",
// 							timestamp: Date.now(),
// 						}),
// 						{
// 							status: 200,
// 							headers: { "Content-Type": "application/json" },
// 						},
// 					);
// 				}
// 				return new Response("ok", { status: 200 });
// 			},
// 			onActorStart: async (
// 				_actorId: string,
// 				_generation: number,
// 				_config: ActorConfig,
// 			) => {
// 				console.log(
// 					`Actor ${_actorId} started (generation ${_generation})`,
// 				);
// 				startedRef.current.resolve(undefined);
// 			},
// 			onActorStop: async (_actorId: string, _generation: number) => {
// 				console.log(
// 					`Actor ${_actorId} stopped (generation ${_generation})`,
// 				);
// 				stoppedRef.current.resolve(undefined);
// 			},
// 			websocket: async (
// 				actorId: string,
// 				ws: WebSocket,
// 				request: Request,
// 			) => {
// 				console.log(`WebSocket connected for actor ${actorId}`);
// 				websocketOpen.resolve(undefined);
// 				actorWebSockets.set(actorId, ws);
//
// 				// Echo server - send back any messages received
// 				ws.on("message", (data) => {
// 					console.log(
// 						`WebSocket message from actor ${actorId}:`,
// 						data.toString(),
// 					);
// 					ws.send(`Echo: ${data}`);
// 				});
//
// 				ws.on("close", () => {
// 					console.log(`WebSocket closed for actor ${actorId}`);
// 					actorWebSockets.delete(actorId);
// 					websocketClosed.resolve(undefined);
// 				});
// 			},
// 		};
//
// 		// Create namespace first
// 		await createNamespace(namespaceName, "Test Namespace");
//
// 		runner = new Runner(config);
//
// 		// Check pegboard URL configuration
// 		expect(runner.pegboardUrl).toBe(
// 			`${RIVET_ENDPOINT_WS}/v1?namespace=${namespaceName}`,
// 		);
//
// 		// Start runner
// 		runner.start();
//
// 		// Wait for runner to be ready
// 		console.log("Waiting runner start...");
// 		await runnerStarted.promise;
//
// 		// Check actor names prepopulated
// 		console.log("Comparing actor names...");
// 		await vi.waitFor(
// 			async () => {
// 				const { names } = await getActorNames(namespaceName);
// 				expect(names.sort()).toStrictEqual(
// 					prepopulateActorNames.sort(),
// 				);
// 			},
// 			{ interval: 100 },
// 		);
//
// 		// Create an actor
// 		console.log("Creating actor...");
// 		const actorResponse = await createActor(
// 			namespaceName,
// 			runnerName,
// 			false,
// 		);
// 		console.log("Actor created:", actorResponse.actor);
// 		const actorId = actorResponse.actor.actor_id;
//
// 		// Wait for actor to start
// 		console.log("Waiting new actor start...");
// 		await startedRef.current.promise;
//
// 		// Ping actor to get actor ID in response (via Guard port)
// 		console.log("Pinging actor...");
// 		const actorPingResponse = await fetch(`${RIVET_ENDPOINT}/ping`, {
// 			method: "GET",
// 			headers: {
// 				"x-rivet-target": "actor",
// 				"x-rivet-actor": actorId,
// 				"x-rivet-addr": "main",
// 			},
// 		});
// 		expect(actorPingResponse.ok).toBe(true);
// 		const pingResult = (await actorPingResponse.json()) as any;
// 		expect(pingResult.actorId).toBe(actorId);
//
// 		// Test WebSocket connection
// 		console.log("Testing WebSocket connection...");
// 		const ws = new WebSocket(`${RIVET_ENDPOINT_WS}/ws`, {
// 			headers: {
// 				"x-rivet-target": "actor",
// 				"x-rivet-actor": actorId,
// 				"x-rivet-addr": "main",
// 			},
// 		});
//
// 		const testMessage = "Hello, actor!";
// 		const messagePromise = new Promise<string>((resolve, reject) => {
// 			ws.once("open", () => {
// 				console.log("WebSocket connected");
// 				ws.send(testMessage);
// 			});
// 			ws.once("message", (data) => {
// 				resolve(data.toString());
// 			});
// 			ws.once("error", reject);
// 		});
//
// 		await websocketOpen.promise;
//
// 		// Test WebSocket messaging
// 		console.log("Testing WebSocket messaging...");
// 		const response = await messagePromise;
// 		expect(response).toBe(`Echo: ${testMessage}`);
//
// 		// Close WebSocket for now
// 		ws.close();
// 		console.log("Waiting websocket close...");
// 		await websocketClosed.promise;
//
// 		await testKv(runner, actorId);
//
// 		// Sleep and wake actor 3 times in a loop
// 		for (let i = 1; i <= 3; i++) {
// 			console.log(`Sleep/wake cycle ${i}/3`);
//
// 			// Sleep actor
// 			console.log(`Sleeping actor (cycle ${i})...`);
// 			stoppedRef.current = Promise.withResolvers();
// 			runner.sleepActor(actorId);
//
// 			console.log("Waiting actor sleep...");
// 			await stoppedRef.current.promise;
//
// 			// Make network request to wake actor (via Guard)
// 			console.log(`Waking actor (cycle ${i})...`);
// 			startedRef.current = Promise.withResolvers();
// 			const wakeResponse = await fetch(`${RIVET_ENDPOINT}/wake`, {
// 				method: "GET",
// 				headers: {
// 					"x-rivet-target": "actor",
// 					"x-rivet-actor": actorId,
// 					"x-rivet-addr": "main",
// 				},
// 			});
// 			console.log(`Wake response status: ${wakeResponse.status}`);
// 			console.log(`Wake response body: ${await wakeResponse.text()}`);
// 			expect(wakeResponse.status).toBe(200);
//
// 			// TODO: Remove this
// 			// Wait for actor to wake
// 			console.log("Waiting actor start...");
// 			await startedRef.current.promise;
// 			console.log(`Actor started successfully for cycle ${i}`);
//
// 			await testKvAfterSleep(runner, actorId);
// 		}
//
// 		// Sleep and wake actor 3 times in a loop
// 		for (let i = 1; i <= 3; i++) {
// 			console.log(`Sleep/wake cycle ${i}/3`);
//
// 			// Sleep actor
// 			console.log(`Sleeping actor (cycle ${i})...`);
// 			stoppedRef.current = Promise.withResolvers();
// 			runner.sleepActor(actorId);
//
// 			console.log("Waiting actor sleep...");
// 			await stoppedRef.current.promise;
//
// 			// Open websocket to wake actor (via Guard)
// 			console.log(`Waking actor (cycle ${i})...`);
// 			startedRef.current = Promise.withResolvers();
// 			const ws = new WebSocket(`${RIVET_ENDPOINT_WS}/ws`, {
// 				headers: {
// 					"x-rivet-target": "actor",
// 					"x-rivet-actor": actorId,
// 					"x-rivet-addr": "main",
// 				},
// 			});
//
// 			await new Promise<void>((resolve, reject) => {
// 				ws.on("open", () => {
// 					console.log("WebSocket connected for wake test");
// 					resolve();
// 				});
// 				ws.on("error", reject);
// 			});
//
// 			// TODO: Remove this
// 			// Wait for actor to wake
// 			console.log("Waiting actor start...");
// 			await startedRef.current.promise;
// 			console.log(`Actor started successfully for cycle ${i}`);
//
// 			await testKvAfterSleep(runner, actorId);
// 		}
//
// 		// Create a fresh WebSocket connection for destroy testing
// 		console.log("Creating WebSocket for destroy test...");
// 		const wsForDestroy = new WebSocket(`${RIVET_ENDPOINT_WS}/ws`, {
// 			headers: {
// 				"x-rivet-target": "actor",
// 				"x-rivet-actor": actorId,
// 				"x-rivet-addr": "main",
// 			},
// 		});
//
// 		await new Promise<void>((resolve, reject) => {
// 			wsForDestroy.on("open", () => {
// 				console.log("WebSocket connected for destroy test");
// 				resolve();
// 			});
// 			wsForDestroy.on("error", reject);
// 		});
//
// 		// Test WebSocket closes on actor destroy
// 		const wsClosePromise = new Promise<void>((resolve) => {
// 			wsForDestroy.on("close", () => {
// 				console.log("WebSocket closed after actor destroy");
// 				resolve();
// 			});
// 		});
//
// 		// Destroy actor
// 		console.log("Destroying actor...");
// 		stoppedRef.current = Promise.withResolvers(); // Create new promise for actor destroy
//
// 		// Start destroy and wait for WebSocket close simultaneously
// 		const destroyPromise = destroyActor(namespaceName, actorId);
//
// 		// Wait for WebSocket to close
// 		console.log("Waiting WS close...");
// 		await wsClosePromise;
//
// 		// Ensure destroy API call completed
// 		await destroyPromise;
// 		console.log("Destroy API call completed");
//
// 		// Wait for actor to stop with timeout
// 		console.log("Waiting actor stopped...");
// 		await stoppedRef.current.promise;
// 		console.log("Actor stop callback completed");
//
// 		// Validate actor is destroyed
// 		console.log("Validating actor is destroyed...");
// 		const destroyedPingResponse = await fetch(`${RIVET_ENDPOINT}/ping`, {
// 			headers: {
// 				"x-rivet-target": "actor",
// 				"x-rivet-actor": actorId,
// 				"x-rivet-addr": "main",
// 			},
// 		});
// 		expect(destroyedPingResponse.status).toBe(404);
//
// 		// Test WebSocket connection to destroyed actor fails
// 		console.log("Testing WebSocket to destroyed actor...");
// 		const wsToDestroyed = new WebSocket(`${RIVET_ENDPOINT_WS}/ws`, {
// 			headers: {
// 				"x-rivet-target": "actor",
// 				"x-rivet-actor": actorId,
// 				"x-rivet-addr": "main",
// 			},
// 		});
//
// 		console.log(
// 			"Waiting WS close...",
// 		);
// 		const closeCode = await new Promise<number>((resolve, reject) => {
// 			wsToDestroyed.on("error", (err) => {
// 				console.log("WebSocket should not have errored");
// 				reject(err);
// 			});
// 			wsToDestroyed.on("close", (code) => {
// 				console.log("WebSocket closed");
// 				resolve(code);
// 			});
// 		});
// 		expect(closeCode).toBe(1011);
//
// 		console.log("E2E test completed successfully!");
//
// 		// Clean up - stop the runner
// 		if (runner) {
// 			await runner.shutdown(false);
// 		}
// 	}, 30_000);
// });
//
// async function testKv(runner: Runner, actorId: string) {
// 	// Test KV operations
// 	console.log("Testing KV operations...");
//
// 	// Test kvPut and kvGet
// 	const testEntries: [Uint8Array, Uint8Array][] = [
// 		[createTestKey(["user", "123"]), createTestValue("alice")],
// 		[createTestKey(["user", "456"]), createTestValue("bob")],
// 		[createTestKey(["config", "theme"]), createTestValue("dark")],
// 		[createTestKey(["config", "lang"]), createTestValue("en")],
// 	];
//
// 	console.log("Testing kvPut...");
// 	await runner.kvPut(actorId, testEntries);
//
// 	console.log("Testing kvGet...");
// 	const getKeys = testEntries.map(([key, _]) => key);
// 	const getResult = await runner.kvGet(actorId, getKeys);
//
// 	expect(getResult.length).toBe(4);
// 	expect(decodeValue(getResult[0]!)).toBe("alice");
// 	expect(decodeValue(getResult[1]!)).toBe("bob");
// 	expect(decodeValue(getResult[2]!)).toBe("dark");
// 	expect(decodeValue(getResult[3]!)).toBe("en");
//
// 	// Test getting non-existent key
// 	const nonExistentResult = await runner.kvGet(actorId, [
// 		createTestKey(["nonexistent"]),
// 	]);
// 	expect(nonExistentResult[0]).toBe(null);
//
// 	// Test kvListAll
// 	console.log("Testing kvListAll...");
// 	const allEntries = await runner.kvListAll(actorId);
// 	expect(allEntries.length).toBe(4);
//
// 	// Verify all entries are present (order may vary)
// 	const allValues = allEntries.map(([_, value]) => decodeValue(value));
// 	expect(allValues.sort()).toEqual(["alice", "bob", "dark", "en"]);
//
// 	// Test kvListAll with limit
// 	const limitedEntries = await runner.kvListAll(actorId, { limit: 2 });
// 	expect(limitedEntries.length).toBe(2);
//
// 	// Test kvListPrefix
// 	console.log("Testing kvListPrefix...");
// 	const userEntries = await runner.kvListPrefix(
// 		actorId,
// 		createTestKey(["user"]),
// 	);
// 	// Note: Prefix queries may not be working as expected on the server side
// 	// For now, we'll test that the method executes without error
// 	expect(userEntries.length).toBeGreaterThanOrEqual(0);
//
// 	const configEntries = await runner.kvListPrefix(
// 		actorId,
// 		createTestKey(["config"]),
// 	);
// 	expect(configEntries.length).toBeGreaterThanOrEqual(0);
//
// 	// Test kvListRange
// 	console.log("Testing kvListRange...");
// 	const rangeEntries = await runner.kvListRange(
// 		actorId,
// 		createTestKey(["config"]),
// 		createTestKey(["user"]),
// 		false, // inclusive
// 	);
// 	// Range queries may have varying behavior depending on key ordering
// 	expect(rangeEntries.length).toBeGreaterThanOrEqual(0);
//
// 	// Test kvDelete
// 	console.log("Testing kvDelete...");
// 	const keysToDelete = [createTestKey(["user", "456"])]; // Delete bob
// 	await runner.kvDelete(actorId, keysToDelete);
//
// 	// Verify deletion worked
// 	const afterDeleteResult = await runner.kvGet(actorId, [
// 		createTestKey(["user", "456"]),
// 	]);
// 	expect(afterDeleteResult[0]).toBe(null);
//
// 	// Verify other data still exists
// 	const remainingUserResult = await runner.kvGet(actorId, [
// 		createTestKey(["user", "123"]),
// 	]);
// 	expect(decodeValue(remainingUserResult[0]!)).toBe("alice");
//
// 	// Test kvDrop operation before destroy
// 	console.log("Testing kvDrop...");
// 	await runner.kvDrop(actorId);
//
// 	// Verify all data is cleared
// 	const afterDropData = await runner.kvGet(actorId, [
// 		createTestKey(["user", "123"]),
// 		createTestKey(["config", "theme"]),
// 		createTestKey(["config", "lang"]),
// 	]);
// 	expect(afterDropData[0]).toBe(null);
// 	expect(afterDropData[1]).toBe(null);
// 	expect(afterDropData[2]).toBe(null);
//
// 	// Verify list operations return empty after drop
// 	const afterDropList = await runner.kvListAll(actorId);
// 	expect(afterDropList.length).toBe(0);
//
// 	// Write data to test it exists during a sleep
// 	console.log("Writing data to live during sleep...");
// 	await runner.kvPut(actorId, [
// 		[createTestKey(["user", "789"]), createTestValue("max")],
// 	]);
// }
//
// async function testKvAfterSleep(runner: Runner, actorId: string) {
// 	// Verify data still exists after waking again
// 	const remainingUserResult = await runner.kvGet(actorId, [
// 		createTestKey(["user", "789"]),
// 	]);
// 	expect(decodeValue(remainingUserResult[0]!)).toBe("max");
// }
//
// function createTestKey(segments: string[]): Uint8Array {
// 	return flattenUint8Arrays(segments.map((s) => new TextEncoder().encode(s)));
// }
//
// function createTestValue(value: string): Uint8Array {
// 	return new TextEncoder().encode(value);
// }
//
// function decodeValue(value: Uint8Array): string {
// 	return new TextDecoder().decode(value);
// }
//
// function flattenUint8Arrays(arrays: Uint8Array[]): Uint8Array {
// 	// Calculate total length
// 	const totalLength = arrays.reduce((sum, arr) => sum + arr.length, 0);
//
// 	// Create result array
// 	const result = new Uint8Array(totalLength);
//
// 	// Copy each array
// 	let offset = 0;
// 	for (const arr of arrays) {
// 		result.set(arr, offset);
// 		offset += arr.length;
// 	}
//
// 	return result;
// }
