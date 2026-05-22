import { Actor, State } from "@rivetkit/effect";
import { Context, DateTime, Effect, Layer, Schema, Stream } from "effect";
import { db } from "rivetkit/db";
import { Moderator } from "../moderator/api.ts";
import { ChatRoom, MemberNotInRoomError } from "./api.ts";

// --- Services ---

// Actors can use custom Effect services like any other Effect program.
// Provide the service layer to the actor layer, then yield it in the wake scope.
export class RoomPolicy extends Context.Service<
	RoomPolicy,
	{
		readonly requireMember: (
			members: ReadonlyArray<{ readonly name: string }>,
			name: string,
		) => Effect.Effect<void, MemberNotInRoomError>;
	}
>()("RoomPolicy") {}

export const RoomPolicyLive = Layer.succeed(
	RoomPolicy,
	RoomPolicy.of({
		requireMember: (members, name) =>
			members.some((member) => member.name === name)
				? Effect.void
				: Effect.fail(
						new MemberNotInRoomError({
							name,
							message: `${name} is not a member of this room`,
						}),
					),
	}),
);

// --- Actor Implementation ---

// `.toLayer` produces a Layer that registers this actor
// with the `Registry` service that is in context. The first parameter
// is a `wake` function that runs once when the actor awakes
// and returns the action handlers.
export const ChatRoomLive = ChatRoom.toLayer(
	// Wake scope (runs on each wake)
	({ rawRivetkitContext, state }) =>
		Effect.gen(function* () {
			// Actor-provided services, custom services, and actor clients are all
			// yielded from the Effect context for this wake. They are scoped to
			// this actor instance, not to individual action calls.
			const address = yield* Actor.CurrentAddress;
			const roomPolicy = yield* RoomPolicy;
			const moderatorClient = yield* Moderator.client;

			// Access the actor's persisted `state` with a `SubscriptionRef`-like API
			const name = State.get(state).pipe(
				Effect.orDie,
				Effect.map((s) => s.name),
			);

			yield* Effect.log("room awake", {
				actorId: address.actorId,
				key: address.key.join("/"),
				name,
			});

			// Finalizers run on sleep
			yield* Effect.addFinalizer(() =>
				Effect.gen(function* () {
					yield* Effect.log("room sleeping", {
						actorId: address.actorId,
						key: address.key.join("/"),
						name,
					});
				}),
			);

			// `State.changes` streams every committed state change for this actor wake.
			yield* State.changes(state).pipe(
				Stream.runForEach((current) =>
					Effect.log("room state changed", {
						actorId: address.actorId,
						name: current.name,
						memberCount: current.members.length,
					}),
				),
				Effect.forkScoped,
			);

			// Combine persisted actor state with a custom service-owned domain guard.
			const ensureMember = (name: string) =>
				State.get(state).pipe(
					Effect.orDie,
					Effect.flatMap((current) =>
						roomPolicy.requireMember(current.members, name),
					),
				);

			// --- Message processing (not yet implemented) ---
			// Pull-based: the actor controls when to take the next message.
			// Forked into a scoped fiber, so it runs in the background and
			// is canceled on sleep. Re-enable once ChatRoom messages land.
			//
			// yield* Effect.gen(function* () {
			// 	const msg = yield* Queue.take(messages)
			// 	yield* Match.value(msg).pipe(
			// 		Match.tag("Reset", () =>
			// 			Effect.gen(function* () {
			// 				yield* State.set(state, 0)
			// 				yield* PubSub.publish(events.countChanged, 0)
			// 			})
			// 		),
			// 		Match.tag("SendSystemMessage", ({ payload, complete }) =>
			// 			Effect.gen(function* () {
			// 				yield* complete(payload.text)
			// 			})
			// 		),
			// 		Match.exhaustive,
			// 	)
			// }).pipe(Effect.forever, Effect.forkScoped)

			// --- Action handlers (request-response) ---
			return ChatRoom.of({
				Initialize: ({ payload }) =>
					// This replaces `createState(input)`. Callers should initialize
					// a room before actions that depend on a persisted room name.
					State.update(state, (current) => {
						if (current.initialized) return current;
						return {
							...current,
							name: payload.name,
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

						// The raw scheduler dispatches the Effect action by name
						// with the same object payload that a client would send.
						rawRivetkitContext.schedule.after(
							1_000,
							"SendMessage",
							{
								sender: "Admin",
								text: `Welcome to the room, ${payload.name}!`,
							},
						);

						return { memberCount: next.members.length };
					}),
				Leave: ({ payload }) =>
					Effect.gen(function* () {
						yield* ensureMember(payload.name);

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
						yield* ensureMember(payload.sender);

						// This is a workaround. Scope helper actors to this run so stale
						// singleton actors left in the local engine DB cannot trap nested RPCs.
						const runKey = ["run", ...address.key];
						// Actor-to-actor RPC uses the same API as client-to-actor RPC.
						const moderator = moderatorClient.getOrCreate(runKey);

						// If Review fails with BannedWordsError, that typed error
						// flows through SendMessage's declared error channel.
						yield* moderator.Review({ text: payload.text });

						const createdAt = yield* DateTime.now;
						yield* Effect.tryPromise(() =>
							rawRivetkitContext.db.execute(
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
					}),
				GetHistory: () =>
					Effect.tryPromise(() =>
						rawRivetkitContext.db.execute<{
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
				Archive: () =>
					Effect.sync(() => {
						rawRivetkitContext.destroy();
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
				initialized: Schema.Boolean,
			}),
			initialValue: () => ({
				name: "",
				members: [{ name: "Admin", joinedAt: DateTime.nowUnsafe() }],
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
		name: "Chat Room", // Human-friendly display name
		icon: "comments", // FontAwesome icon name
	},
);
