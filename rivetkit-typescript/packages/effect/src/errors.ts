import { Schema } from "effect";

export class RuntimeExecutionError extends Schema.TaggedError<RuntimeExecutionError>()(
	"RuntimeExecutionError",
	{
		message: Schema.String,
		operation: Schema.optional(Schema.String),
		cause: Schema.optional(Schema.Unknown),
	},
) {}

export class StatePersistenceError extends Schema.TaggedError<StatePersistenceError>()(
	"StatePersistenceError",
	{
		message: Schema.String,
		cause: Schema.optional(Schema.Unknown),
	},
) {}

export class QueueError extends Schema.TaggedError<QueueError>()("QueueError", {
	message: Schema.String,
	cause: Schema.optional(Schema.Unknown),
}) {}
