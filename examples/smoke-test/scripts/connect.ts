import { createClient } from "rivetkit/client";
import type { Registry } from "../src/registry";

async function main() {
	const client = createClient<Registry>();

	const counter = client.counter.getOrCreate("foo").connect();

	counter.on("newCount", (count: number) => console.log("Event:", count));
	await counter.increment(1);

	setInterval(() => {}, 1000);

	// for (let i = 0; i < 5; i++) {
	// 	const out = await counter.increment(5);
	// 	console.log("RPC:", out);
	//
	// 	await new Promise((resolve) => setTimeout(resolve, 1000));
	// }
	//
	// await new Promise((resolve) => setTimeout(resolve, 2000));
	// await counter.dispose();
	//
	// await new Promise((resolve) => setTimeout(resolve, 200));
	//
	// const counter2 = client.counter.getOrCreate().connect();
	//
	// counter2.on("newCount", (count: number) => console.log("Event:", count));
	//
	// for (let i = 0; i < 5; i++) {
	// 	const out = await counter2.increment(5);
	// 	console.log("RPC:", out);
	//
	// 	await new Promise((resolve) => setTimeout(resolve, 1000));
	// }
	//
	// await new Promise((resolve) => setTimeout(resolve, 2000));
	// await counter2.dispose();
}

main();
