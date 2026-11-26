import { Context } from "effect";

/**
 * Aborts when the actor tears down while its workflow still runs: at a
 * destroy, or when a sleep reaches the end of its grace period. Its reason is
 * the eviction error of the workflow engine.
 */
export const WorkflowTeardownSignal = Context.Reference<AbortSignal>(
	"@rivetkit/effect/internal/WorkflowTeardownSignal",
	{ defaultValue: () => new AbortController().signal },
);
