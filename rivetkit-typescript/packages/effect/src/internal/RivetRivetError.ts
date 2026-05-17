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

export const makeActionErrorMetadata = (
	error: unknown,
): ActionErrorMetadata => ({
	_tag: ActionErrorMetadataTag,
	version: ActionErrorSchemaVersion,
	error,
});

export const makeActionError = (
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

const ActorSpecifier = Schema.Struct({
	actorId: Schema.String,
	generation: Schema.Number,
	key: Schema.optional(Schema.String),
}) satisfies Schema.Codec<NonNullable<Rivetkit.RivetErrorLike["actor"]>>;

export const RivetkitRivetError = Schema.Struct({
	group: Schema.String,
	code: Schema.String,
	message: Schema.String,
	metadata: Schema.optional(Schema.Unknown),
	public: Schema.optional(Schema.Boolean),
	statusCode: Schema.optional(Schema.Number),
	actor: Schema.optional(ActorSpecifier),
}) satisfies Schema.Codec<Rivetkit.RivetErrorLike>;

export type RivetkitRivetError = typeof RivetkitRivetError.Type;
