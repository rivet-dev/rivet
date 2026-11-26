import { Schema } from "effect";
import * as RivetkitErrors from "rivetkit/errors";
import { hasStringProperty } from "./utils.ts";

export const tag = "EffectActionError" as const;

export const schemaVersion = 1 as const;

export const ActionErrorEnvelope = Schema.Struct({
	_tag: Schema.tag(tag),
	version: Schema.Literal(schemaVersion),
	error: Schema.Unknown,
});

export type ActionErrorEnvelope = typeof ActionErrorEnvelope.Type;

export const make = (error: unknown): ActionErrorEnvelope => ({
	_tag: tag,
	version: schemaVersion,
	error,
});

/** Wraps an encoded declared error so the client can decode it. */
export const toUserError = (encodedError: unknown, fallbackMessage: string) =>
	new RivetkitErrors.UserError(
		hasStringProperty("message")(encodedError)
			? encodedError.message
			: fallbackMessage,
		{
			code: hasStringProperty("_tag")(encodedError)
				? encodedError._tag
				: undefined,
			metadata: make(encodedError),
		},
	);
