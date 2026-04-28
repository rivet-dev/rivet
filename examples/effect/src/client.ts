import { Effect, Stream } from "effect"
import { Client } from "@rivetkit/effect"
import {
	Counter, IncrementBy,
	// ChatRoom,
} from "./actors/mod.ts"

const program = Effect.gen(function* () {
	const counterClient = yield* Counter.client

	const counter = counterClient.getOrCreate(["counter-123"])

	// Action calls return Effects with types inferred from the schema.
	//   counter.Increment: (payload: { amount: number }) => Effect<number, CounterOverflowError>
	const count = yield* counter.Increment({ amount: 5 })
	yield* Effect.log(`Count: ${count}`)

	const newCount = yield* counter.send(IncrementBy({ amount: 3 }))
	yield* Effect.log(`Count: ${newCount}`)

	// subscribe returns a Stream typed from the event schema.
	yield* counter.subscribe("countChanged").pipe(
		Stream.take(3),
		Stream.runForEach((n) => Effect.log(`Changed: ${n}`)),
	)
})
// program: Effect<void, CounterOverflowError, Client>
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
