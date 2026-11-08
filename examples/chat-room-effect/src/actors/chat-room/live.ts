import { Actor } from "@rivetkit/effect";
import { Context, DateTime, Effect, Layer } from "effect";
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
	Effect.fnUntraced(function* ({ rawRivetkitContext }) {
		// Actor-provided services, custom services, and actor clients are all
		// yielded from the Effect context for this wake. They are scoped to
		// this actor instance, not to individual action calls.
		const address = yield* Actor.CurrentAddress;
		const roomPolicy = yield* RoomPolicy;
		const moderatorClient = yield* Moderator.client;

		yield* Effect.log("room awake", {
			actorId: address.actorId,
			key: address.key.join("/"),
		});

		// Runs a query against the actor's SQLite database as an Effect.
		const query = <TRow extends Record<string, unknown>>(
			sql: string,
			...args: unknown[]
		) =>
			Effect.tryPromise(() =>
				rawRivetkitContext.db.execute<TRow>(sql, ...args),
			).pipe(Effect.orDie);

		const getRoomName = query<{ name: string }>(
			"SELECT name FROM room WHERE id = 1",
		).pipe(Effect.map((rows) => rows[0]?.name ?? ""));

		const getMembers = query<{ name: string }>(
			"SELECT name FROM members ORDER BY id",
		);

		// Logs every committed membership change, right after the write.
		const logMembersChanged = Effect.fnUntraced(function* () {
			const roomName = yield* getRoomName;
			const members = yield* getMembers;
			yield* Effect.log("room members changed", {
				actorId: address.actorId,
				roomName,
				memberCount: members.length,
			});
			return members.length;
		});

		// Finalizers run on sleep
		yield* Effect.addFinalizer(
			Effect.fnUntraced(function* () {
				// Read the persisted room name from the actor's SQLite database
				const roomName = yield* getRoomName;
				yield* Effect.log("room sleeping", {
					actorId: address.actorId,
					key: address.key.join("/"),
					roomName,
				});
			}),
		);

		// Combine persisted room members with a custom service-owned domain guard.
		const ensureMember = (name: string) =>
			getMembers.pipe(
				Effect.flatMap((members) => roomPolicy.requireMember(members, name)),
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
		// 				yield* state.set(0)
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
				// This replaces passing an `input` when creating an actor. Callers
				// should initialize a room before actions that depend on a
				// persisted room name. Only the first call sets the name.
				query(
					"UPDATE room SET name = ?, initialized = 1 WHERE id = 1 AND initialized = 0",
					payload.name,
				).pipe(Effect.asVoid),
			Join: Effect.fnUntraced(function* ({ payload }) {
				const joinedAt = yield* DateTime.now;
				const member = {
					name: payload.name,
					joinedAt,
				};
				yield* query(
					"INSERT INTO members (name, joined_at) VALUES (?, ?)",
					member.name,
					DateTime.toEpochMillis(member.joinedAt),
				);
				const memberCount = yield* logMembersChanged();

				rawRivetkitContext.broadcast("memberJoined", {
					member: {
						...member,
						joinedAt: DateTime.formatIso(member.joinedAt),
					},
				});

				// The raw scheduler dispatches the Effect action by name
				// with the same object payload that a client would send.
				rawRivetkitContext.schedule.after(1_000, "SendMessage", {
					sender: "Admin",
					text: `Welcome to the room, ${payload.name}!`,
				});

				return { memberCount };
			}),
			Leave: Effect.fnUntraced(function* ({ payload }) {
				yield* ensureMember(payload.name);

				yield* query("DELETE FROM members WHERE name = ?", payload.name);
				yield* logMembersChanged();

				rawRivetkitContext.broadcast("memberLeft", {
					name: payload.name,
				});
			}),
			SendMessage: Effect.fnUntraced(function* ({ payload }) {
				yield* ensureMember(payload.sender);

				// Actor-to-actor RPC uses the same API as client-to-actor RPC.
				const moderator = moderatorClient.getOrCreate([
					...address.key,
					"main",
				]);

				// If Review fails with BannedWordsError, that typed error
				// flows through SendMessage's declared error channel.
				yield* moderator
					.Review({ text: payload.text })
					.pipe(Effect.catchTag("RivetError", Effect.die));

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
		db: db({
			onMigrate: async (client) => {
				await client.execute(`
					CREATE TABLE IF NOT EXISTS room (
						id INTEGER PRIMARY KEY CHECK (id = 1),
						name TEXT NOT NULL,
						initialized INTEGER NOT NULL
					)
				`);
				await client.execute(`
					CREATE TABLE IF NOT EXISTS members (
						id INTEGER PRIMARY KEY AUTOINCREMENT,
						name TEXT NOT NULL,
						joined_at INTEGER NOT NULL
					)
				`);
				// Seed a new room with the Admin member. The room row only
				// exists after the first start, so this runs once.
				await client.execute(
					"INSERT INTO members (name, joined_at) SELECT 'Admin', ? WHERE NOT EXISTS (SELECT 1 FROM room)",
					Date.now(),
				);
				await client.execute(
					"INSERT OR IGNORE INTO room (id, name, initialized) VALUES (1, '', 0)",
				);
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
