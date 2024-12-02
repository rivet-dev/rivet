import { ActorClient } from "../../sdks/actors/client/src/mod.ts"

async function main() {
	const actorClient = new ActorClient("http://127.0.0.1:20025");

	// Broadcast event
	let broadcastActor;
	{
		const mod = 3;
		broadcastActor = await actorClient.withTags({ name: "counter" });
		broadcastActor.on("directCount", (count: unknown) => {
			console.log(`Direct (n % ${mod}):`, count);
		});
	}

	// Direct event
	let directActor;
	{
		const mod = 3;
		directActor = await actorClient.withTags({ name: "counter" });
		directActor.on("directCount", (count: unknown) => {
			console.log(`Broadcast:`, count);
		});
	}

	// Simple RPC
	{
		const actor = await actorClient.withTags({ name: "counter" })
		const newCount: number = await actor.rpc("increment", 5);
		console.log('Simple RPC:', newCount);
		actor.disconnect();
	}

	// Multiple RPC calls
	{
		const actor = await actorClient.withTags({ name: "counter" });

		for (let i = 0; i < 10; i++) {
			const output = await actor.rpc("increment", 5);
			console.log('Reusing handle:', output);
		}

		actor.disconnect();
	}

	// WebSocket
	{
		const actor = await actorClient.withTags({ name: "counter" });
		for (let i = 0; i < 10; i++) {
			const newOutput = await actor.rpc("increment", 5);
			console.log('WebSocket:', newOutput);
		}

		actor.disconnect();
	}

	await directActor.rpc("destroyMe");

	// Disconnect all actors before 
	broadcastActor.disconnect();
	directActor.disconnect();
}

await main();
