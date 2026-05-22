import { Action, Actor } from "@rivetkit/effect";
import { Schema } from "effect";
import { BannedWordsError } from "../mod";

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

export const Member = Schema.Struct({
	name: Schema.String,
	joinedAt: Schema.DateTimeUtc,
});

export const Message = Schema.Struct({
	id: Schema.Number,
	sender: Schema.String,
	text: Schema.String,
	createdAt: Schema.DateTimeUtc,
});

export const Initialize = Action.make("Initialize", {
	payload: { name: Schema.String },
});

export const Join = Action.make("Join", {
	payload: { name: Schema.String },
	success: Member,
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
	success: Schema.Array(Message),
});

export const GetMembers = Action.make("GetMembers", {
	success: Schema.Array(Member),
});

export const ScheduleAnnouncement = Action.make("ScheduleAnnouncement", {
	payload: {
		text: Schema.String,
		delay: Schema.DurationFromMillis,
	},
	success: Schema.Struct({
		firesAt: Schema.DateTimeUtc,
	}),
});

export const TriggerAnnouncement = Action.make("TriggerAnnouncement", {
	payload: { text: Schema.String },
});

export const Archive = Action.make("Archive");

// --- Messages (not yet implemented) ---
//
// // Non-completable (fire-and-forget)
// export const Reset = Message.make("Reset", {
// 	payload: { reason: Schema.String },
// })
//
// // Completable (sender can await a typed response)
// export const IncrementBy = Message.make("IncrementBy", {
// 	payload: { amount: Schema.Number },
// 	success: Schema.Number,
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
		GetMembers,
		ScheduleAnnouncement,
		TriggerAnnouncement,
		Archive,
	],
	// messages: [Reset, IncrementBy],	// durable, queued, background
	// events: { countChanged: Schema.Number },
});
