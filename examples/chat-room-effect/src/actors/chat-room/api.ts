import { Action, Actor, Event } from "@rivetkit/effect";
import { Schema } from "effect";
import { BannedWordsError } from "../moderator/api.ts";

// --- Errors ---

export class MemberNotInRoomError extends Schema.TaggedError<MemberNotInRoomError>()(
	"MemberNotInRoomError",
	{
		name: Schema.String,
		message: Schema.String,
	},
) {}

// --- Actions ---

// Actions use explicit schemas which enable:
//
// - Runtime validation. Schemas validate encoded data end to end
//    which protects from malformed, stale, or malicious data.
//
// - Encoding/decoding control. Effect Schema distinguishes encoded
//    (wire) and decoded (runtime) types. Values like `URL`, `bigint`,
//    or custom domain types can have safe encoded forms on the wire
//    and rich decoded forms in action handlers. Schemas can also require
//    custom services during encode/decode.

// Sets the room name. Only the first call has an effect.
export const Initialize = Action.make("Initialize", {
	payload: { name: Schema.String },
});

export const Join = Action.make("Join", {
	payload: { name: Schema.String },
	success: Schema.Struct({
		memberCount: Schema.Number,
	}),
});

export const Leave = Action.make("Leave", {
	payload: { name: Schema.String },
	error: MemberNotInRoomError,
});

export const SendMessage = Action.make("SendMessage", {
	payload: {
		sender: Schema.String,
		text: Schema.String,
	},
	error: Schema.Union([MemberNotInRoomError, BannedWordsError]),
});

export const GetHistory = Action.make("GetHistory", {
	success: Schema.Array(
		Schema.Struct({
			id: Schema.Number,
			sender: Schema.String,
			text: Schema.String,
			createdAt: Schema.DateTimeUtc,
		}),
	),
});

// Destroys the room and deletes its data.
export const Archive = Action.make("Archive");

// --- Events ---

// The room sends events to its connections. Payloads are encoded with their
// schemas, like action payloads.

export const MemberJoined = Event.make("MemberJoined", {
	payload: { name: Schema.String, joinedAt: Schema.DateTimeUtc },
});

export const MemberLeft = Event.make("MemberLeft", {
	payload: { name: Schema.String },
});

export const NewMessage = Event.make("NewMessage", {
	payload: {
		sender: Schema.String,
		text: Schema.String,
		createdAt: Schema.DateTimeUtc,
	},
});

// --- Actor Definition ---

// The definition is the actor's public contract. It carries no
// implementation or server-only configuration, so it does not leak
// server-specific implementation details when importing from the client.
export const ChatRoom = Actor.make("ChatRoom", {
	// Actions are standalone values (vs. embedded in the actor definition)
	// as it allows for shared action protocols (e.g., a `Ping` health check
	// or `GetMetrics` action defined once and composed into multiple actors).
	actions: [Initialize, Join, Leave, SendMessage, GetHistory, Archive],
	events: [MemberJoined, MemberLeft, NewMessage],
});
