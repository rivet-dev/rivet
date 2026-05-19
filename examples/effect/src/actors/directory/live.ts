import { Effect, Schema } from "effect";
import { ActorState, State } from "@rivetkit/effect";
import { Directory } from "./api.ts";

const DirectoryState = ActorState.make("DirectoryState", {
	schema: Schema.Struct({
		rooms: Schema.Array(
			Schema.Struct({
				name: Schema.String,
				openedAt: Schema.Number,
				closedAt: Schema.optionalKey(Schema.Number),
			}),
		),
	}),
	initialValue: () => ({ rooms: [] }),
});

export const DirectoryLive = Directory.toLayer(
	({ state }) =>
		Effect.gen(function* () {
			return Directory.of({
				RegisterRoom: ({ payload }) =>
					// State writes go through Effect Schema validation. This
					// example treats schema failures as defects instead of adding
					// typed error channels to the action contract.
					State.update(state, (current) => {
						if (
							current.rooms.some(
								(room) => room.name === payload.name,
							)
						) {
							return current;
						}

						return {
							rooms: [
								...current.rooms,
								{ name: payload.name, openedAt: Date.now() },
							],
						};
					}).pipe(Effect.orDie),
				CloseRoom: ({ payload }) =>
					State.update(state, (current) => ({
						rooms: current.rooms.map((room) =>
							room.name === payload.name
								? { ...room, closedAt: Date.now() }
								: room,
						),
					})).pipe(Effect.orDie),
				ListRooms: () =>
					State.get(state).pipe(
						Effect.orDie,
						Effect.map((s) => s.rooms),
					),
			});
		}),
	{ state: DirectoryState, name: "Directory", icon: "folder" },
);
