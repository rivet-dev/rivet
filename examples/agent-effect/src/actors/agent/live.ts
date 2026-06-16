import { Actor, State } from "@rivetkit/effect";
import { Effect, Schema } from "effect";
import { LanguageModel, Prompt } from "effect/unstable/ai";
import { Agent, EmptyMessageError, Message } from "./api.ts";

// The system prompt is applied on every call but never persisted as a turn.
const SYSTEM_PROMPT =
	"You are a helpful assistant living inside a Rivet Actor. " +
	"Keep replies short and friendly.";

// --- Actor Implementation ---

// `.toLayer` produces a Layer that registers this actor with the `Registry`
// service in context. The first parameter is a `wake` function that runs once
// when the actor awakes and returns the action handlers.
//
// The action handlers require the `LanguageModel` service from Effect's
// context. It is never created here: the actor stays provider-agnostic and the
// concrete model Layer is provided where the actor layer is composed (a real
// OpenAI model in `main.ts`, a mock model in the test). That is the dependency
// injection seam that makes the LLM swappable.
export const AgentLive = Agent.toLayer(
	Effect.fnUntraced(function* ({ state }) {
		return Agent.of({
			SendMessage: Effect.fnUntraced(function* ({ payload }) {
				const content = payload.content.trim();

				// Reject before mutating, so the error path leaves state
				// untouched. The failure is a value in the typed error
				// channel, not a throw.
				if (content.length === 0) {
					return yield* new EmptyMessageError({
						message: "message content cannot be empty",
					});
				}

				// Append the user turn to persisted state first, so the model
				// always sees the message it is replying to even if the actor
				// restarts mid-call.
				const withUser = yield* State.updateAndGet(state, (history) => [
					...history,
					{ role: "user", content } satisfies Message,
				]).pipe(Effect.orDie);

				// Call the LLM with the running history. The system prompt plus
				// every persisted turn is sent on each call, which is what gives
				// the agent memory across calls and restarts.
				const response = yield* LanguageModel.generateText({
					prompt: toPrompt(withUser),
				}).pipe(Effect.orDie);

				const reply = response.text;

				// Append the assistant turn and persist it.
				yield* State.update(state, (history) => [
					...history,
					{ role: "assistant", content: reply } satisfies Message,
				]).pipe(Effect.orDie);

				return reply;
			}),
			GetHistory: () => State.get(state).pipe(Effect.orDie),
		});
	}),
	{
		state: {
			schema: Schema.Array(Message),
			initialValue: () => [],
		},
		name: "Agent", // Human-friendly display name
		icon: "robot", // FontAwesome icon name
	},
);

// Build an Effect AI prompt from the persisted history. The system prompt is
// prepended on every call; persisted turns become the user/assistant messages.
function toPrompt(
	history: ReadonlyArray<Message>,
): ReadonlyArray<Prompt.MessageEncoded> {
	return [
		{ role: "system", content: SYSTEM_PROMPT },
		...history.map(
			(turn): Prompt.MessageEncoded => ({
				role: turn.role,
				content: turn.content,
			}),
		),
	];
}
