import { Actor, State } from "@rivetkit/effect";
import { DateTime, Duration, Effect, Random, Schema } from "effect";
import { db } from "rivetkit/db";
import { Directory, Moderator } from "../mod.ts";
import { ChatRoom } from "./api.ts";

export const ChatRoomLive = ChatRoom.toLayer(
	({ rawRivetkitContext, state }) =>
		Effect.gen(function* () {
			const database = rawRivetkitContext.db;
			const address = yield* Actor.CurrentAddress;
			const moderatorClient = yield* Moderator.client;
			const directoryClient = yield* Directory.client;

			const directory = directoryClient.getOrCreate(["main"]);
			const moderator = moderatorClient.getOrCreate(["main"]);
			// The plain SDK example stores this in createVars. The Effect SDK
			// does not expose vars yet, so the wake-scope closure owns it.
			const sessionId = yield* Random.nextUUIDv4;

			yield* State.update(state, (current) => ({
				...current,
				wakeCount: current.wakeCount + 1,
			})).pipe(Effect.orDie);

			yield* Effect.log("room awake", {
				actorId: address.actorId,
				key: address.key.join("/"),
				sessionId,
			});

			yield* Effect.addFinalizer(() =>
				Effect.gen(function* () {
					const current = yield* State.get(state).pipe(Effect.orDie);
					yield* Effect.log("room sleeping", {
						actorId: address.actorId,
						key: address.key.join("/"),
						roomName: current.name,
						sessionId,
						wakeCount: current.wakeCount,
					});
				}),
			);

			const roomName = State.get(state).pipe(
				Effect.orDie,
				Effect.map((s) => s.name),
			);

			return ChatRoom.of({
				Initialize: ({ payload }) =>
					// This replaces createState(input). Callers should initialize
					// a room before actions that depend on a persisted room name.
					State.update(state, (current) => {
						if (current.initialized) return current;
						return {
							...current,
							name: payload.name,
							members: [],
							initialized: true,
						};
					}),
				Join: ({ payload }) =>
					Effect.gen(function* () {
						const joinedAt = yield* DateTime.now;
						const member = {
							name: payload.name,
							joinedAt,
						};
						const next = yield* State.updateAndGet(
							state,
							(current) => ({
								...current,
								members: [...current.members, member],
							}),
						);

						rawRivetkitContext.broadcast("memberJoined", {
							member: {
								...member,
								joinedAt: DateTime.formatIso(member.joinedAt),
							},
						});

						if (next.name !== "") {
							// Directory registration is still actor-to-actor RPC, but
							// it uses the Effect action name and object payload.
							yield* directory.RegisterRoom({ name: next.name });
						}

						return member;
					}),
				Leave: ({ payload }) =>
					Effect.gen(function* () {
						yield* State.update(state, (current) => ({
							...current,
							members: current.members.filter(
								(member) => member.name !== payload.name,
							),
						})).pipe(Effect.orDie);
						rawRivetkitContext.broadcast("memberLeft", {
							name: payload.name,
						});
					}),
				SendMessage: ({ payload }) =>
					Effect.gen(function* () {
						// The normal example sends moderation work through a
						// completable queue drained by run(). The Effect SDK does
						// not expose queues or run loops yet, so moderation is a
						// direct actor RPC and has no queue timeout path.
						const verdict = yield* moderator.Review({
							text: payload.text,
						});

						if (!verdict.approved) {
							return { ok: false, reason: verdict.reason };
						}

						const createdAt = yield* DateTime.now;
						yield* Effect.tryPromise(() =>
							database.execute(
								"INSERT INTO messages (sender, text, created_at) VALUES (?, ?, ?)",
								payload.sender,
								payload.text,
								DateTime.toEpochMillis(createdAt),
							),
						).pipe(Effect.orDie);

						rawRivetkitContext.broadcast("newMessage", {
							sender: payload.sender,
							text: payload.text,
							createdAt: DateTime.formatIso(createdAt),
						});
						return { ok: true, createdAt };
					}),
				GetHistory: () =>
					Effect.tryPromise(() =>
						database.execute<{
							id: number;
							sender: string;
							text: string;
							createdAt: number;
						}>(
							"SELECT id, sender, text, created_at as createdAt FROM messages ORDER BY id",
						),
					).pipe(
						Effect.map((rows) =>
							rows.map((row) => ({
								...row,
								createdAt: DateTime.makeUnsafe(row.createdAt),
							})),
						),
						Effect.orDie,
					),
				GetMembers: () =>
					State.get(state).pipe(
						Effect.orDie,
						Effect.map((s) => s.members),
					),
				ScheduleAnnouncement: ({ payload }) =>
					Effect.sync(() => {
						const firesAt = DateTime.addDuration(
							DateTime.nowUnsafe(),
							payload.delay,
						);
						// The raw scheduler dispatches the Effect action by name
						// with the same object payload that a client would send.
						rawRivetkitContext.schedule.after(
							Duration.toMillis(payload.delay),
							"TriggerAnnouncement",
							{
								text: payload.text,
							},
						);
						return { firesAt };
					}),
				TriggerAnnouncement: ({ payload }) =>
					Effect.sync(() => {
						rawRivetkitContext.broadcast("announcement", {
							text: payload.text,
						});
					}),
				Archive: () =>
					Effect.gen(function* () {
						const name = yield* roomName;
						if (name !== "") {
							// This only covers destruction through Archive. A future
							// Effect onDestroy hook would cover every destroy path.
							yield* directory.CloseRoom({ name });
						}
						yield* Effect.sync(() => {
							rawRivetkitContext.destroy();
						});
					}),
			});
		}),
	{
		state: {
			schema: Schema.Struct({
				name: Schema.String,
				members: Schema.Array(
					Schema.Struct({
						name: Schema.String,
						joinedAt: Schema.DateTimeUtc,
					}),
				),
				wakeCount: Schema.Number,
				initialized: Schema.Boolean,
			}),
			initialValue: () => ({
				name: "",
				members: [],
				wakeCount: 0,
				initialized: false,
			}),
		},
		db: db({
			onMigrate: async (client) => {
				await client.execute(`
					CREATE TABLE IF NOT EXISTS messages (
						id INTEGER PRIMARY KEY AUTOINCREMENT,
						sender TEXT NOT NULL,
						text TEXT NOT NULL,
						created_at INTEGER NOT NULL
					)
				`);
			},
		}),
		name: "Chat Room",
		icon: "comments",
	},
);
