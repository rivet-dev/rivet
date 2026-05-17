import { Schema } from "effect";
import * as Rivetkit from "rivetkit";
import { hasStringProperty } from "./utils";

export const ActionErrorMetadataTag = "EffectActionError" as const;

export const ActionErrorSchemaVersion = 1 as const;

export const ActionErrorMetadata = Schema.Struct({
	_tag: Schema.tag(ActionErrorMetadataTag),
	version: Schema.Literal(ActionErrorSchemaVersion),
	error: Schema.Unknown,
});

export type ActionErrorMetadata = typeof ActionErrorMetadata.Type;

export const isActionErrorMetadata = Schema.is(ActionErrorMetadata);

const makeActionErrorMetadata = (error: unknown): ActionErrorMetadata => ({
	_tag: ActionErrorMetadataTag,
	version: ActionErrorSchemaVersion,
	error,
});

export const make = (
	actionTag: string,
	encodedError: unknown,
): Rivetkit.UserError =>
	new Rivetkit.UserError(
		hasStringProperty("message")(encodedError)
			? encodedError.message
			: `${actionTag} failed`,
		{
			code: hasStringProperty("_tag")(encodedError)
				? encodedError._tag
				: undefined,
			metadata: makeActionErrorMetadata(encodedError),
		},
	);
