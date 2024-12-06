import { Client, ActorHandle } from "../../sdks/actors/client/src/mod.ts"
import type Counter from "./counter.ts";

async function main() {
	const endpoint = Deno.env.get("MANAGER_ENDPOINT");
	if (!endpoint) throw new Error("Missing MANAGER_ENDPOINT");
	const actorClient = new Client(endpoint);


	// Broadcast event
	let broadcastActor: ActorHandle<Counter>;
	{
		broadcastActor = await actorClient.withTags({ name: "counter" });
		broadcastActor.on("broadcastCount", (count: unknown) => {
			console.log("Broadcast:", count);
		});
	}

	// Direct event
	let directActor: ActorHandle<Counter>;
	{
		const mod = 3;
		directActor = await actorClient.withTags(
			{ name: "counter" },
			{
				parameters: { mod },
			}
		);
		directActor.on("directCount", (count: unknown) => {
			console.log(`Direct (n % ${mod}):`, count);
		});
	}

	// Simple RPC
	{
		const actor = await actorClient.withTags<Counter>({ name: "counter" });
		const newCount = await actor.increment(1);
		console.log('Simple RPC:', newCount);
		actor.disconnect();
	}

	// Multiple RPC calls
	{
		const actor = await actorClient.withTags<Counter>({ name: "counter" });

		for (let i = 0; i < 10; i++) {
			const output = await actor.increment(1);
			console.log('Multiple RPC:', output);
		}

		actor.disconnect();
	}

	await directActor.destroyMe();

	// Disconnect all actors before 
	broadcastActor.disconnect();
	directActor.disconnect();
}

await main();
