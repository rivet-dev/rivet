import { Cause, Schema } from "effect";

export class RuntimeExecutionError extends Schema.TaggedError<RuntimeExecutionError>()(
	"RuntimeExecutionError",
	{
		message: Schema.String,
		operation: Schema.optional(Schema.String),
		cause: Schema.optional(Schema.Unknown),
	},
) {}

export const makeRuntimeExecutionError = (operation: string, cause: Cause.Cause<unknown>) =>
	new RuntimeExecutionError({
		message: `Effect failed during ${operation}`,
		operation,
		cause: Cause.pretty(cause),
	});

export class StatePersistenceError extends Schema.TaggedError<StatePersistenceError>()(
	"StatePersistenceError",
	{
		message: Schema.String,
		cause: Schema.optional(Schema.Unknown),
	},
) {}
