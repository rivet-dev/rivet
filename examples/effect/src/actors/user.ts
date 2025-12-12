import { Data, Effect } from "effect";
import { actor } from "rivetkit";
import { Action } from "@rivetkit/effect";
import { Activity } from "@effect/workflow";

export const user = actor({
	createState: (c, input: UserInput): UserState => ({
		email: input.email,
		customerId: crypto.randomUUID(),
	}),
	actions: {
		getEmail: Action.effect(function* (c) {
			const s = yield* Action.state(c);
			return s.email;
		}),
		updateEmail: Action.workflow(function* (c, newEmail: string) {
			if (!validateEmail(newEmail)) {
				return yield* Effect.fail(
					new InvalidEmailError({ email: newEmail }),
				);
			}

			const s = yield* Action.state(c);
			yield* Activity.make({
				name: "UpdateStripeEmail",
				execute: updateStripeCustomerEmail(s.customerId, newEmail),
			});
			yield* Action.updateState(c, (state) => {
				state.email = newEmail;
			});
			yield* Activity.make({
				name: "SendConfirmationEmail",
				execute: sendResendEmailConfirmation(newEmail),
			});
		}),
	},
});

// ===
interface UserInput {
	email: string;
}

interface UserState {
	email: string;
	customerId: string;
}

class InvalidEmailError extends Data.TaggedError("InvalidEmailError")<{
	email: string;
}> {}

function validateEmail(email: string): boolean {
	return true;
}

// External service functions (empty implementations for demonstration)
function createStripeCustomer(email: string): Effect.Effect<string> {
	return Effect.succeed(`cus_${Date.now()}`);
	// Real implementation would use Effect.tryPromise() or similar
	// return Effect.tryPromise(() => stripe.customers.create({ email }))
}

function updateStripeCustomerEmail(
	customerId: string,
	email: string,
): Effect.Effect<void> {
	return Effect.void;
	// Real implementation:
	// return Effect.tryPromise(() => stripe.customers.update(customerId, { email }))
}

function sendResendWelcomeEmail(email: string): Effect.Effect<void> {
	return Effect.void;
	// Real implementation:
	// return Effect.tryPromise(() => resend.emails.send({ to: email, ... }))
}

function sendResendEmailConfirmation(email: string): Effect.Effect<void> {
	return Effect.void;
	// Real implementation:
	// return Effect.tryPromise(() => resend.emails.send({ to: email, ... }))
}
