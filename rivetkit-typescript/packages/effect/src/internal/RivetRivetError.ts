import { Schema } from "effect";
import type * as Rivetkit from "rivetkit";

export const ActionErrorMetadataTag = "EffectActionError" as const;

export const ActionErrorSchemaVersion = 1 as const;

export const ActionErrorMetadata = Schema.Struct({
	_tag: Schema.tag(ActionErrorMetadataTag),
	version: Schema.Literal(ActionErrorSchemaVersion),
	error: Schema.Unknown,
});

export type ActionErrorMetadata = typeof ActionErrorMetadata.Type;

export const RivetkitRivetError = Schema.Struct({
	message: Schema.String,
	group: Schema.String,
	code: Schema.String,
	metadata: Schema.optional(Schema.Unknown),
}) satisfies Schema.Codec<Rivetkit.RivetErrorLike>;

export type RivetkitRivetError = typeof RivetkitRivetError.Type;
