import { Schema } from "effect";

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
