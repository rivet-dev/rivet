import { Effect } from "effect";
import { Calculator } from "./api";

export const program = Effect.gen(function* () {
	const calculator = (yield* Calculator.client).getOrCreate("main");

	const quotient = yield* calculator
		.Divide({ dividend: 1, divisor: 0 })
		.pipe(
			Effect.catchTag("DivisionByZero", ({ dividend }) =>
				Effect.logWarning(`Cannot divide ${dividend} by zero`).pipe(
					Effect.as(Number.NaN),
				),
			),
		);
	yield* Effect.log(`Quotient: ${quotient}`);
});
