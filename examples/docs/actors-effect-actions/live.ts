import { Effect } from "effect";
import { Calculator, DivisionByZero } from "./api";

export const CalculatorLive = Calculator.toLayer(
	Calculator.of({
		Divide: ({ payload }) =>
			payload.divisor === 0
				? Effect.fail(
						new DivisionByZero({ dividend: payload.dividend }),
					)
				: Effect.succeed(payload.dividend / payload.divisor),
	}),
);
