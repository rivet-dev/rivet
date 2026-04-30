import { Layer } from "effect"
import { NodeRuntime } from "@effect/platform-node"
import { Registry } from "@rivetkit/effect"
import { CounterLive } from "./actors/counter/live.ts"
// import { ChatRoomLive } from "./actors/chat-room/live.ts"

const ActorsLayer = Layer.mergeAll(
	CounterLive,
//	ChatRoomLive,
)

const MainLayer = ActorsLayer.pipe(
	Layer.provide(Registry.layer({ storagePath: "./data" })),
)

// Keeps the layer alive. Tears down on SIGINT/SIGTERM.
Layer.launch(MainLayer).pipe(NodeRuntime.runMain)
