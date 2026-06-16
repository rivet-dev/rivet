import { Action, Actor } from "@rivetkit/effect";
import { Schema } from "effect";

// --- Domain types ---

// A single turn in the conversation. `role` distinguishes who said it
// ("user" or "assistant"); the system prompt is applied at call time and
// is not persisted as a turn.
export const Message = Schema.Struct({
	role: Schema.Literals(["user", "assistant"]),
	content: Schema.String,
});
export type Message = typeof Message.Type;

// --- Errors ---

// A typed, schema-validated error. It travels through the action's error
// channel and arrives on the caller as a real tagged instance that can be
// matched with `Effect.catchTag`, not a string or an opaque exception.
export class EmptyMessageError extends Schema.TaggedErrorClass<EmptyMessageError>()(
	"EmptyMessageError",
	{
		message: Schema.String,
	},
) {}

// --- Actions ---

// Actions are standalone values with explicit `effect/Schema` payloads,
// successes, and errors. The schemas validate encoded data end to end and
// control how values are encoded on the wire and decoded inside handlers.

// Sends a user message to the agent and returns the assistant's reply. The
// whole conversation history is persisted in actor state, so the model sees
// every prior turn even across actor restarts.
export const SendMessage = Action.make("SendMessage", {
	payload: { content: Schema.String },
	success: Schema.String,
	error: EmptyMessageError,
});

// Returns the full persisted conversation.
export const GetHistory = Action.make("GetHistory", {
	success: Schema.Array(Message),
});

// --- Actor Definition ---

// The definition is the actor's public contract. It carries no implementation
// or server-only configuration, so it can be imported from client code without
// leaking server details (or the LLM provider wiring).
export const Agent = Actor.make("Agent", {
	actions: [SendMessage, GetHistory],
});
