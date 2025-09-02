// import { Bench } from "tinybench";
// import { Runner } from "@/mod";
// import type { ActorConfig } from "@/mod";
// import {
// 	createActor,
// 	destroyActor,
// 	setupBenchmarkRunner,
// 	createPromiseResolver,
// 	RIVET_ENDPOINT,
// } from "./utils.js";
// import { afterEach } from "node:test";
//
// async function runActorLifecycleBenchmark() {
// 	// Shared state for benchmarks
// 	let runner: Runner | null = null;
// 	let namespaceName: string;
// 	let runnerName: string;
// 	let createdActors: string[] = [];
// 	let wakeActorId: string | null = null;
// 	let stopped: { promise: Promise<void>; resolve: () => void };
// 	let started: { promise: Promise<void>; resolve: () => void };
//
// 	const bench = new Bench({
// 		time: 1000,
// 		iterations: 10,
// 		warmupTime: 0,
// 		warmupIterations: 0,
// 		throws: true,
// 		setup: async (task) => {
// 			// Setup benchmark runner
// 			console.log("Setting up benchmark runner...");
// 			stopped = createPromiseResolver<void>();
// 			started = createPromiseResolver<void>();
//
// 			const setup = await setupBenchmarkRunner(
// 				"lifecycle",
// 				5054,
// 				async (
// 					_actorId: string,
// 					_generation: number,
// 					_config: ActorConfig,
// 				) => {
// 					started.resolve();
// 				},
// 				async (_actorId: string, _generation: number) => {
// 					stopped.resolve();
// 				},
// 			);
// 			runner = setup.runner;
// 			namespaceName = setup.namespaceName;
// 			runnerName = setup.runnerName;
//
// 			console.log(
// 				`Benchmark setup complete. Namespace: ${namespaceName}, Runner: ${runnerName}`,
// 			);
// 		},
// 		teardown: async () => {
// 			if (runner) {
// 				await runner.shutdown(true);
// 			}
//
// 			// Clean up created actors from creation benchmark
// 			console.log(
// 				`Cleaning up ${createdActors.length} actors in ${namespaceName}...`,
// 			);
// 			const cleanupActor = createdActors;
// 			createdActors = [];
// 			wakeActorId = null;
// 			for (const actorId of cleanupActor) {
// 				try {
// 					await destroyActor(namespaceName, actorId);
// 				} catch (err) {
// 					console.warn(`Failed to clean up actor ${actorId}:`, err);
// 				}
// 			}
//
// 			console.log("Benchmark teardown complete!");
// 		},
// 	});
//
// 	bench.add("create actor", async () => {
// 		const actorResponse = await createActor(
// 			namespaceName,
// 			runnerName,
// 			false,
// 		);
// 		const actorId = actorResponse.actor.actor_id;
// 		createdActors.push(actorId);
//
// 		// Ping the actor
// 		const pingResponse = await fetch(`${RIVET_ENDPOINT}/ping`, {
// 			method: "GET",
// 			headers: {
// 				"x-rivet-target": "actor",
// 				"x-rivet-actor": actorId,
// 				"x-rivet-addr": "main",
// 			},
// 		});
// 		if (!pingResponse.ok) throw "Request failed";
// 	});
//
// 	//bench.add(
// 	//	"wake actor from sleep",
// 	//	async () => {
// 	//		if (!wakeActorId) throw "No wake actor ID";
// 	//
// 	//		// Ping the actor
// 	//		const pingResponse = await fetch(`${RIVET_ENDPOINT}/ping`, {
// 	//			method: "GET",
// 	//			headers: {
// 	//				"x-rivet-target": "actor",
// 	//				"x-rivet-actor": wakeActorId,
// 	//				"x-rivet-addr": "main",
// 	//			},
// 	//		});
// 	//
// 	//		if (!pingResponse.ok) {
// 	//			console.error(
// 	//				`Ping failed: ${pingResponse.status} ${pingResponse.statusText}`,
// 	//			);
// 	//			const errorText = await pingResponse.text();
// 	//			console.error(`Error response: ${errorText}`);
// 	//			throw `Request failed: ${pingResponse.status} ${pingResponse.statusText}`;
// 	//		}
// 	//	},
// 	//	{
// 	//		beforeEach: async () => {
// 	//			// Reset promise resolvers for this iteration
// 	//			started = createPromiseResolver<void>();
// 	//			stopped = createPromiseResolver<void>();
// 	//
// 	//			// Create the actor that will be used for wake benchmarking
// 	//			console.log('Creating actor');
// 	//			const wakeActorResponse = await createActor(
// 	//				namespaceName,
// 	//				runnerName,
// 	//				false,
// 	//				"wake-bench-actor",
// 	//			);
// 	//			wakeActorId = wakeActorResponse.actor.actor_id;
// 	//			createdActors.push(wakeActorId!);
// 	//
// 	//			// Wait for actor to start
// 	//			await started.promise;
// 	//
// 	//			// Put actor to sleep initially
// 	//			runner!.sleepActor(wakeActorId!);
// 	//			await stopped.promise;
// 	//		},
// 	//	},
// 	//	// TODO(RVT-4979): Add back after sleep cycles fixed
// 	//	//{
// 	//	//	beforeAll: async () => {
// 	//	//		// Create the actor that will be used for wake benchmarking
// 	//	//		console.log("Creating wake actor...");
// 	//	//		const wakeActorResponse = await createActor(
// 	//	//			namespaceName,
// 	//	//			runnerName,
// 	//	//			false,
// 	//	//			"wake-bench-actor",
// 	//	//		);
// 	//	//		wakeActorId = wakeActorResponse.actor.actor_id;
// 	//	//		createdActors.push(wakeActorId!);
// 	//	//
// 	//	//		// Wait for actor to start
// 	//	//		await started.promise;
// 	//	//	},
// 	//	//	beforeEach: async () => {
// 	//	//		console.log("Putting actor to sleep...");
// 	//	//
// 	//	//		// Put actor to sleep initially
// 	//	//		stopped = createPromiseResolver<void>();
// 	//	//		runner!.sleepActor(wakeActorId!);
// 	//	//		await stopped.promise;
// 	//	//	},
// 	//	//},
// 	//);
//
// 	// Run the benchmark
// 	console.log("Running benchmarks...");
// 	await bench.run();
//
// 	// Display results
// 	console.table(bench.table());
//
// 	console.log("Benchmark complete!");
// }
//
// // Run the benchmark if this file is executed directly
// if (import.meta.url === `file://${process.argv[1]}`) {
// 	runActorLifecycleBenchmark();
// }
