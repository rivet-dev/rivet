/**
 * Actor Error Utilities
 *
 * Helpers for inspecting errors from actor actions. The `@rivetkit/svelte`
 * action middleware normalizes all errors to `Error` instances, but at runtime
 * errors from `UserError` throws arrive as `ActorError` from `rivetkit/client`
 * with `.code`, `.group`, and `.metadata` intact.
 *
 * These utilities let consumers discriminate `ActorError` from generic `Error`
 * without importing `rivetkit/client` directly.
 *
 * @module
 */

import { ActorError } from "rivetkit/client";

/** Serializable shape recognized by RivetKit's actor-error guard. */
export interface ActorErrorLike {
  /** Optional runtime discriminator; modern RivetKit uses `RivetError`. */
  __type?: "RivetError" | "ActorError";
  /** Error family (`user`, `actor`, `client`, and so on). */
  group: string;
  /** Machine-readable failure code. */
  code: string;
  /** Human-readable failure message. */
  message: string;
  /** Optional structured details safe for the caller. */
  metadata?: unknown;
  /** Optional request identifier used to correlate the error with engine logs. */
  rayId?: string;
  /** Whether the error is safe to expose outside the actor runtime. */
  public?: boolean;
  /** Optional HTTP status override associated with the error. */
  statusCode?: number;
  /** Actor generation that was handling work when the error was produced. */
  actor?: {
    actorId: string;
    generation: number;
    key?: string;
  };
}

/**
 * Type guard: is the error an `ActorError` from rivetkit/client?
 *
 * Delegates to RivetKit's structural guard so modern `RivetError` instances,
 * legacy `ActorError` tags, and serialized errors all work across realms. The
 * `Error` intersection preserves the package's historical narrowing contract
 * for existing callers; use {@link ActorErrorLike} when typing a serialized
 * value before it reaches this guard.
 */
export function isActorError(err: unknown): err is Error & ActorErrorLike {
  return ActorError.isActorError(err);
}

/**
 * Extract the machine-readable error code from an error, if it's an ActorError.
 * Returns `undefined` for non-ActorError instances.
 */
export function actorErrorCode(err: unknown): string | undefined {
  if (!err) return undefined;
  if (isActorError(err)) return err.code;
  return undefined;
}

/**
 * Extract the human-readable error message from an error.
 * Works for both `ActorError` (message = UserError's first argument)
 * and generic `Error` instances.
 *
 * Returns `undefined` for null/undefined input.
 */
export function actorErrorMessage(err: unknown): string | undefined {
  if (!err) return undefined;
  if (typeof err === "object" && "message" in err) {
    const message = (err as { message?: unknown }).message;
    return typeof message === "string" && message ? message : undefined;
  }
  return undefined;
}

/**
 * Structured error info extracted from an actor handle's `lastActionError`.
 */
export interface ActionErrorInfo {
  /** Human-readable error message. */
  message: string | undefined;
  /** Machine-readable error code (only present for `ActorError`). */
  code: string | undefined;
  /** Whether this is an `ActorError` (from a `UserError` throw on the server). */
  isActorError: boolean;
}

/**
 * Extract structured error info from any actor handle's `lastActionError`.
 *
 * Useful for presenting structured action failures in application UI. Returns `null` when there is no error.
 *
 * @example
 * ```typescript
 * const error = getActionError(threadHandle);
 * if (error) {
 *   showToast(error.message ?? "Something went wrong");
 *   if (error.code === "RATE_LIMITED") retryLater();
 * }
 * ```
 */
export function getActionError(handle: {
  lastActionError: unknown;
}): ActionErrorInfo | null {
  const err = handle.lastActionError;
  if (!err) return null;
  return {
    message: actorErrorMessage(err),
    code: actorErrorCode(err),
    isActorError: isActorError(err),
  };
}
