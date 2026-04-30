import { Effect } from "effect"
import { Client } from "@rivetkit/effect"
import { Counter } from "./actors/mod.ts"

const program = Effect.gen(function* () {
	const counterClient = yield* Counter.client

	const counter = counterClient.getOrCreate(["counter-123"])

	// Action calls return Effects with types inferred from the schema.
	const count = yield* counter.Increment({ amount: 5 })
	yield* Effect.log(`Count: ${count}`)

	const total = yield* counter.GetCount()
	yield* Effect.log(`Total: ${total}`)
})
// program: Effect<void, CounterOverflowError | ClientError, Client>
//                                             ^^^^^^
//  Missing Client -> compile error naming the central runtime dependency.

// ------------------------------------------------------------------
// Wiring: provide Client once. Each actor's .client effect
// uses that transport to create a contract-specific typed accessor.
// ------------------------------------------------------------------
const ClientLayer = Client.layer({
	endpoint: "https://api.rivet.dev",
	token: "...",
})

program.pipe(Effect.provide(ClientLayer), Effect.runPromise)
