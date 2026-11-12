import { Effect } from "effect";
import { LanguageModel, type Prompt } from "effect/ai";
import { db } from "rivetkit/db";
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
	Effect.fnUntraced(function* ({ rawRivetkitContext }) {
		const appendTurn = (turn: Message) =>
			Effect.tryPromise(() =>
				rawRivetkitContext.db.execute(
					"INSERT INTO messages (role, content) VALUES (?, ?)",
					turn.role,
					turn.content,
				),
			).pipe(Effect.orDie);

		const readHistory = Effect.tryPromise(() =>
			rawRivetkitContext.db.execute<Message>(
				"SELECT role, content FROM messages ORDER BY id",
			),
		).pipe(Effect.orDie);

		return Agent.of({
			SendMessage: Effect.fnUntraced(function* ({ payload }) {
				const content = payload.content.trim();

				// Reject before writing, so the error path leaves the history
				// untouched. The failure is a value in the typed error channel,
				// not a throw.
				if (content.length === 0) {
					return yield* new EmptyMessageError({
						message: "message content cannot be empty",
					});
				}

				// Store the user turn in SQLite first, so the model always sees
				// the message it is replying to even if the actor restarts
				// mid-call.
				yield* appendTurn({ role: "user", content });
				const history = yield* readHistory;

				// Call the LLM with the running history. The system prompt plus
				// every stored turn is sent on each call, which is what gives
				// the agent memory across calls and restarts.
				const response = yield* LanguageModel.generateText({
					prompt: toPrompt(history),
				}).pipe(Effect.orDie);

				const reply = response.text;

				// Store the assistant turn.
				yield* appendTurn({ role: "assistant", content: reply });

				return reply;
			}),
			GetHistory: () => readHistory,
		});
	}),
	{
		db: db({
			onMigrate: async (client) => {
				await client.execute(`
					CREATE TABLE IF NOT EXISTS messages (
						id INTEGER PRIMARY KEY AUTOINCREMENT,
						role TEXT NOT NULL,
						content TEXT NOT NULL
					)
				`);
			},
		}),
		name: "Agent", // Human-friendly display name
		icon: "robot", // FontAwesome icon name
	},
);

// Build an Effect AI prompt from the stored history. The system prompt is
// prepended on every call; stored turns become the user/assistant messages.
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
