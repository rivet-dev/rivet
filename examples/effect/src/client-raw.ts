import { createClient } from "rivetkit/client";

const client = createClient("http://127.0.0.1:6420") as any;

async function main() {
	const runId = crypto.randomUUID();
	const counter = client.Counter.getOrCreate(`counter-raw-${runId}`);

	const initial = await counter.GetCount();
	console.log("GetCount (initial):", initial);

	const afterFive = await counter.Increment({ amount: 5 });
	console.log("Increment(5):", afterFive);

	const afterEight = await counter.Increment({ amount: 3 });
	console.log("Increment(3):", afterEight);

	const total = await counter.GetCount();
	console.log("GetCount (total):", total);

	// Trigger overflow (limit: 20). Plain client surfaces this as a
	// thrown rivetkit RivetError with Effect action-error metadata.
	try {
		const overflowed = await counter.Increment({ amount: 20 });
		console.log("Increment(20) [unexpected success]:", overflowed);
	} catch (err) {
		console.log("Increment(20) [expected error]:", err);
	}
}

main().catch((err) => {
	console.error("client smoke test failed:", err);
	process.exit(1);
});
