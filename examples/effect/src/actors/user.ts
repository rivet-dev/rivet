import { Data, Effect } from "effect";
import { actor } from "rivetkit";
import { Activity } from "@effect/workflow";
import { Action } from "../effect/index.ts";

// User actor - demonstrates Effect workflows with external service calls
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

function updateStripeCustomerEmail(
	customerId: string,
	email: string,
): Effect.Effect<void> {
	return Effect.void;
}

function sendResendEmailConfirmation(email: string): Effect.Effect<void> {
	return Effect.void;
}

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
