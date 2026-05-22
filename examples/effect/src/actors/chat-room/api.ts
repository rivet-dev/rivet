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

// The plain RivetKit example uses createState input to name the room at
// creation time. The Effect SDK does not expose create input yet, so this
// action initializes the persisted room state explicitly after getOrCreate.
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

// Scheduled actions receive the same single schema payload that normal
// Effect actions use. This replaces the plain SDK example's positional
// triggerAnnouncement(text) action.
export const TriggerAnnouncement = Action.make("TriggerAnnouncement", {
	payload: { text: Schema.String },
});

// The plain RivetKit example closes the room from onDestroy. The Effect SDK
// does not expose onDestroy yet, so archive performs cleanup before destroy.
export const Archive = Action.make("Archive");

export const ChatRoom = Actor.make("chatRoom", {
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
});
