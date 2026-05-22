import { Action, Actor } from "@rivetkit/effect";
import { Schema } from "effect";
import { BannedWordsError } from "../moderator/api.ts";

// --- Errors ---

export class MemberNotInRoomError extends Schema.TaggedErrorClass<MemberNotInRoomError>()(
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

// This action replaces passing an `input` when creating an actor.
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

// This action replaces passing an `input` when creating an actor.
export const Archive = Action.make("Archive");

// --- Messages (not yet implemented) ---
//
// // Non-completable (fire-and-forget)
// export const Reset = Message.make("Reset", {
// 	payload: { reason: Schema.String },
// })
//
// // Completable (sender can await a typed response)
// export const SendSystemMessage = Message.make("SendSystemMessage", {
// 	payload: { text: Schema.String },
// 	success: Schema.String,
// })

// --- Actor Definition ---

// The definition is the actor's public contract. It carries no
// implementation or server-only configuration, so it does not leak
// server-specific implementation details when importing from the client.
export const ChatRoom = Actor.make("chatRoom", {
	// Actions are standalone values (vs. embedded in the actor definition)
	// as it allows for shared action protocols (e.g., a `Ping` health check
	// or `GetMetrics` action defined once and composed into multiple actors).
	actions: [
		Initialize,
		Join,
		Leave,
		SendMessage,
		GetHistory,
		Archive,
	],
	// messages: [Reset, SendSystemMessage],	// durable, queued, background
	// events: { messageAdded: Schema.String },
});
