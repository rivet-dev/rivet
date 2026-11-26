import { Client } from "@rivetkit/effect";
import { Effect, ManagedRuntime } from "effect";
import { Counter } from "./api";

// Build the client one time and share it between requests.
const runtime = ManagedRuntime.make(
	Client.layer({ endpoint: "http://localhost:6420" }),
);

export const increment = (amount: number): Promise<number> =>
	runtime.runPromise(
		Effect.gen(function* () {
			const counter = (yield* Counter.client).getOrCreate(
				"server-counter",
			);
			return yield* counter.Increment({ amount });
		}),
	);
