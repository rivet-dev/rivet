import { Actor, db } from "@rivetkit/effect";
import { Context, DateTime, Effect, Layer } from "effect";
import { Moderator } from "../moderator/api.ts";
import {
	ChatRoom,
	MemberJoined,
	MemberLeft,
	MemberNotInRoomError,
	NewMessage,
	SendMessage,
} from "./api.ts";

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
	Effect.fn(function* () {
		// Actor-provided services, custom services, and actor clients are all
		// yielded from the Effect context for this wake. They are scoped to
		// this actor instance, not to individual action calls.
		const address = yield* Actor.CurrentAddress;
		const roomPolicy = yield* RoomPolicy;
		const moderatorClient = yield* Moderator.client;
		// An `effect/sql` client on the actor's SQLite database.
		const sql = yield* ChatRoom.sql;
		const schedule = yield* ChatRoom.schedule;
		// The events API for this actor's declared events.
		const events = yield* ChatRoom.events;

		yield* Effect.log("room awake", {
			actorId: address.actorId,
			key: address.key.join("/"),
		});

		const getRoomName = sql<{
			name: string;
		}>`SELECT name FROM room WHERE id = 1`.pipe(
			Effect.map((rows) => rows[0]?.name ?? ""),
			Effect.orDie,
		);

		const getMembers = sql<{
			name: string;
		}>`SELECT name FROM members ORDER BY id`.pipe(Effect.orDie);

		// Logs every committed membership change, right after the write.
		const logMembersChanged = Effect.fn(function* () {
			const roomName = yield* getRoomName;
			const members = yield* getMembers;
			yield* Effect.log("room members changed", {
				actorId: address.actorId,
				roomName,
				memberCount: members.length,
			});
			return members.length;
		});

		// Finalizers run when the wake ends: on sleep and on destroy
		yield* Effect.addFinalizer(
			Effect.fn(function* () {
				// Read the persisted room name from the actor's SQLite database
				const roomName = yield* getRoomName;
				yield* Effect.log("room stopping", {
					actorId: address.actorId,
					key: address.key.join("/"),
					roomName,
				});
			}),
		);

		// Combine persisted room members with a custom service-owned domain guard.
		const ensureMember = (name: string) =>
			getMembers.pipe(
				Effect.flatMap((members) =>
					roomPolicy.requireMember(members, name),
				),
			);

		// --- Action handlers (request-response) ---
		return ChatRoom.of({
			Initialize: ({ payload }) =>
				// Callers should initialize a room before actions that depend on
				// a persisted room name. Only the first call sets the name.
				sql`UPDATE room SET name = ${payload.name}, initialized = 1 WHERE id = 1 AND initialized = 0`.pipe(
					Effect.asVoid,
					Effect.orDie,
				),
			Join: Effect.fn(function* ({ payload }) {
				const joinedAt = yield* DateTime.now;
				yield* sql`INSERT INTO members (name, joined_at) VALUES (${payload.name}, ${DateTime.toEpochMillis(joinedAt)})`.pipe(
					Effect.orDie,
				);
				const memberCount = yield* logMembersChanged();

				yield* events.broadcast(MemberJoined, {
					name: payload.name,
					joinedAt,
				});

				/**
				 * Schedule a call to this actor's own `SendMessage` action.
				 * The schedule is durable, so it survives sleep.
				 */
				yield* schedule
					.after("1 second", SendMessage, {
						sender: "Admin",
						text: `Welcome to the room, ${payload.name}!`,
					})
					.pipe(Effect.orDie);

				return { memberCount };
			}),
			Leave: Effect.fn(function* ({ payload }) {
				yield* ensureMember(payload.name);

				yield* sql`DELETE FROM members WHERE name = ${payload.name}`.pipe(
					Effect.orDie,
				);
				yield* logMembersChanged();

				yield* events.broadcast(MemberLeft, { name: payload.name });
			}),
			SendMessage: Effect.fn(function* ({ payload }) {
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
				yield* sql`INSERT INTO messages (sender, text, created_at) VALUES (${payload.sender}, ${payload.text}, ${DateTime.toEpochMillis(createdAt)})`.pipe(
					Effect.orDie,
				);

				yield* events.broadcast(NewMessage, {
					sender: payload.sender,
					text: payload.text,
					createdAt,
				});
			}),
			GetHistory: () =>
				sql<{
					id: number;
					sender: string;
					text: string;
					createdAt: number;
				}>`SELECT id, sender, text, created_at as createdAt FROM messages ORDER BY id`.pipe(
					Effect.map((rows) =>
						rows.map((row) => ({
							...row,
							createdAt: DateTime.makeUnsafe(row.createdAt),
						})),
					),
					Effect.orDie,
				),
			// Destroys the actor and deletes its database.
			Archive: () => Actor.destroy,
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
