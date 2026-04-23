import { Layer } from "effect"
import { NodeRuntime } from "@effect/platform-node"
import { Registry, TestRegistry } from "@rivetkit/effect"
import { CounterLive } from "./actors/Counter.ts"
// import { ChatRoomLive } from "./actors/ChatRoom.ts"

const ActorsLayer = Layer.mergeAll(
	CounterLive,
//	ChatRoomLive,
)

const MainLayer = ActorsLayer.pipe(
  Layer.provide(Registry.layer({ storagePath: "./data" })),
)

const TestLayer = ActorsLayer.pipe(
  Layer.provide(TestRegistry.layer),
)

// Keeps the layer alive. Tears down on SIGINT/SIGTERM.
Layer.launch(MainLayer).pipe(NodeRuntime.runMain)
