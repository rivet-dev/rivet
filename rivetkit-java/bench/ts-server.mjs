// TypeScript SDK side of the SDK comparison benchmark (`mise run bench:compare`).
// Same actor shape as the Rust and Java bench servers: increment(amount) adds to the
// state and returns it. Runs on the napi runtime (rivetkit-napi built from this repo).
//
// Env: RIVET_ENDPOINT, RIVET_TOKEN, RIVET_NAMESPACE, RIVET_POOL (read by rivetkit).
import { actor, setup } from "../../rivetkit-typescript/packages/rivetkit/dist/tsup/mod.js";

const counter = actor({
	state: { count: 0 },
	actions: {
		increment: (c, amount) => {
			c.state.count += amount;
			return c.state.count;
		},
	},
});

const registry = setup({ use: { counter } });
registry.start();
console.log("bench server ready (typescript)");
