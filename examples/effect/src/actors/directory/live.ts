import { DateTime, Effect, Schema } from "effect";
import { State } from "@rivetkit/effect";
import { Directory, RoomEntry } from "./api.ts";

export const DirectoryLive = Directory.toLayer(
	({ state }) =>
		Effect.gen(function* () {
			return Directory.of({
				RegisterRoom: ({ payload }) =>
					// State writes go through Effect Schema validation. This
					// example treats schema failures as defects instead of adding
					// typed error channels to the action contract.
					Effect.gen(function* () {
						const openedAt = yield* DateTime.now;

						yield* State.update(state, (current) => {
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
									{ name: payload.name, openedAt },
								],
							};
						}).pipe(Effect.orDie);
					}),
				CloseRoom: ({ payload }) =>
					Effect.gen(function* () {
						const closedAt = yield* DateTime.now;

						yield* State.update(state, (current) => ({
							rooms: current.rooms.map((room) =>
								room.name === payload.name
									? { ...room, closedAt }
									: room,
							),
						})).pipe(Effect.orDie);
					}),
				ListRooms: () =>
					State.get(state).pipe(
						Effect.orDie,
						Effect.map((s) => s.rooms),
					),
			});
		}),
	{
		state: {
			schema: Schema.Struct({
				rooms: Schema.Array(
					Schema.Struct({
						name: Schema.String,
						openedAt: Schema.DateTimeUtc,
						closedAt: Schema.optionalKey(Schema.DateTimeUtc),
					}),
				),
			}),
			initialValue: () => ({ rooms: [] }),
		},
		name: "Directory",
		icon: "folder",
	},
);
