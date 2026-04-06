// import { Runner } from "@/mod";
// import type { RunnerConfig, ActorConfig } from "@/mod";
//
// export const RIVET_ENDPOINT =
// 	process.env.RIVET_ENDPOINT ?? "http://localhost:6420";
//
// export async function createActor(
// 	namespaceName: string,
// 	runnerNameSelector: string,
// 	durable: boolean,
// 	actorName: string = "bench-actor",
// ): Promise<any> {
// 	const response = await fetch(
// 		`${RIVET_ENDPOINT}/actors?namespace=${namespaceName}`,
// 		{
// 			method: "POST",
// 			headers: {
// 				"Content-Type": "application/json",
// 			},
// 			body: JSON.stringify({
// 				name: actorName,
// 				input: btoa("bench-input"),
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
// export async function destroyActor(
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
// export async function createNamespace(
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
// export interface BenchmarkRunnerSetup {
// 	runner: Runner;
// 	namespaceName: string;
// 	runnerName: string;
// }
//
// export async function setupBenchmarkRunner(
// 	namespaceSuffix: string,
// 	port: number,
// 	onActorStart?: (
// 		actorId: string,
// 		generation: number,
// 		config: ActorConfig,
// 	) => Promise<void>,
// 	onActorStop?: (actorId: string, generation: number) => Promise<void>,
// ): Promise<BenchmarkRunnerSetup> {
// 	const namespaceName = `bench-${crypto.randomUUID().slice(0, 8)}`;
// 	const runnerName = `bench-runner`;
//
// 	let runnerStartedResolver: () => void;
// 	const runnerStarted = new Promise<void>((resolve) => {
// 		runnerStartedResolver = resolve;
// 	});
//
// 	const config: RunnerConfig = {
// 		version: 1,
// 		endpoint: RIVET_ENDPOINT,
// 		namespace: namespaceName,
// 		addresses: { main: { host: "127.0.0.1", port } },
// 		totalSlots: 100,
// 		prepopulateActorNames: [],
// 		runnerName: runnerName,
// 		runnerKey: "default",
// 		onConnected: () => {
// 			runnerStartedResolver();
// 		},
// 		onDisconnected: () => {},
// 		fetch: async (_actorId: string, request: Request) => {
// 			return new Response("ok", { status: 200 });
// 		},
// 		onActorStart: onActorStart || (async () => {}),
// 		onActorStop: onActorStop || (async () => {}),
// 	};
//
// 	await createNamespace(namespaceName, `Bench ${namespaceSuffix} Namespace`);
// 	const runner = new Runner(config);
// 	runner.start();
// 	await runnerStarted;
//
// 	return { runner, namespaceName, runnerName };
// }
//
// export function createPromiseResolver<T = void>(): {
// 	promise: Promise<T>;
// 	resolve: (value: T) => void;
// 	reject: (error: any) => void;
// } {
// 	let resolve: (value: T) => void;
// 	let reject: (error: any) => void;
// 	const promise = new Promise<T>((res, rej) => {
// 		resolve = res;
// 		reject = rej;
// 	});
// 	return { promise, resolve: resolve!, reject: reject! };
// }
//
