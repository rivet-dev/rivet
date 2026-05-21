import { Action, Actor } from "@rivetkit/effect";
import { Schema } from "effect";

export const RoomEntry = Schema.Struct({
	name: Schema.String,
	openedAt: Schema.DateTimeUtc,
	closedAt: Schema.optionalKey(Schema.DateTimeUtc),
});

export const RegisterRoom = Action.make("RegisterRoom", {
	payload: { name: Schema.String },
});

export const CloseRoom = Action.make("CloseRoom", {
	payload: { name: Schema.String },
});

export const ListRooms = Action.make("ListRooms", {
	success: Schema.Array(RoomEntry),
});

export const Directory = Actor.make("directory", {
	actions: [RegisterRoom, CloseRoom, ListRooms],
});
