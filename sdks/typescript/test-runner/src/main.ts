import { Runner } from "@rivetkit/engine-runner";
import type { RunnerConfig, ActorConfig } from "@rivetkit/engine-runner";
import WebSocket from "ws";
import { serve } from "@hono/node-server";

const INTERNAL_SERVER_PORT = process.env.INTERNAL_SERVER_PORT ? Number(process.env.INTERNAL_SERVER_PORT) : 5051;
const RIVET_NAMESPACE = process.env.RIVET_NAMESPACE ?? 'default';
const RIVET_RUNNER_KEY = process.env.RIVET_RUNNER_KEY ?? `key-${Math.floor(Math.random() * 10000)}`;
const RIVET_RUNNER_VERSION = process.env.RIVET_RUNNER_VERSION ? Number(process.env.RIVET_RUNNER_VERSION) : 1;
const RIVET_RUNNER_TOTAL_SLOTS = process.env.RIVET_RUNNER_TOTAL_SLOTS ? Number(process.env.RIVET_RUNNER_TOTAL_SLOTS) : 100;
const RIVET_ENDPOINT = process.env.RIVET_ENDPOINT ?? "http://localhost:6420";

let runnerStarted = Promise.withResolvers();
let websocketOpen = Promise.withResolvers();
let websocketClosed = Promise.withResolvers();
let runner: Runner | null = null;
const actorWebSockets = new Map<string, WebSocket>();

// Start internal server
serve({
	fetch: async (request: Request) => {
		const url = new URL(request.url);
		if (url.pathname == '/wait-ready') {
			await runnerStarted.promise;
			return new Response(JSON.stringify(runner?.runnerId), { status: 200 });
		} else if (url.pathname == '/has-actor') {
			let actorIdQuery = url.searchParams.get('actor');
			let generationQuery = url.searchParams.get('generation');
			let generation = generationQuery ? Number(generationQuery) : undefined;

			if (!actorIdQuery || !runner?.hasActor(actorIdQuery, generation)) {
				return new Response(undefined, { status: 404 });
			}
		} else if (url.pathname == '/shutdown') {
			await runner?.shutdown(true);
		}

		return new Response("ok", { status: 200 });
	},
	port: INTERNAL_SERVER_PORT,
});
console.log(`Internal HTTP server listening on port ${INTERNAL_SERVER_PORT}`);

// Use objects to hold the current promise resolvers so callbacks always get the latest
const startedRef = { current: Promise.withResolvers() };
const stoppedRef = { current: Promise.withResolvers() };

const config: RunnerConfig = {
	version: RIVET_RUNNER_VERSION,
	endpoint: RIVET_ENDPOINT,
	namespace: RIVET_NAMESPACE,
	runnerName: "test-runner",
	runnerKey: RIVET_RUNNER_KEY,
	totalSlots: RIVET_RUNNER_TOTAL_SLOTS,
	prepopulateActorNames: {},
	onConnected: () => {
		runnerStarted.resolve(undefined);
	},
	onDisconnected: () => { },
	fetch: async (actorId: string, request: Request) => {
		console.log(`[TEST-RUNNER] Fetch called for actor ${actorId}, URL: ${request.url}`);
		const url = new URL(request.url);
		if (url.pathname === "/ping") {
			// Return the actor ID in response
			const responseData = {
				actorId,
				status: "ok",
				timestamp: Date.now(),
			};
			console.log(`[TEST-RUNNER] Returning ping response:`, responseData);
			return new Response(
				JSON.stringify(responseData),
				{
					status: 200,
					headers: { "Content-Type": "application/json" },
				},
			);
		}

		return new Response("ok", { status: 200 });
	},
	onActorStart: async (
		_actorId: string,
		_generation: number,
		_config: ActorConfig,
	) => {
		console.log(
			`Actor ${_actorId} started (generation ${_generation})`,
		);
		startedRef.current.resolve(undefined);
	},
	onActorStop: async (_actorId: string, _generation: number) => {
		console.log(
			`Actor ${_actorId} stopped (generation ${_generation})`,
		);
		stoppedRef.current.resolve(undefined);
	},
	websocket: async (
		actorId: string,
		ws: WebSocket,
		request: Request,
	) => {
		console.log(`WebSocket connected for actor ${actorId}`);
		websocketOpen.resolve(undefined);
		actorWebSockets.set(actorId, ws);

		// Echo server - send back any messages received
		ws.addEventListener("message", (event) => {
			const data = event.data;
			console.log(
				`WebSocket message from actor ${actorId}:`,
				data,
			);
			ws.send(`Echo: ${data}`);
		});

		ws.addEventListener("close", () => {
			console.log(`WebSocket closed for actor ${actorId}`);
			actorWebSockets.delete(actorId);
			websocketClosed.resolve(undefined);
		});

		ws.addEventListener("error", (error) => {
			console.error(`WebSocket error for actor ${actorId}:`, error);
		});
	},
};

runner = new Runner(config);

// Start runner
await runner.start();

// Wait for runner to be ready
console.log("Waiting runner start...");
await runnerStarted.promise;
