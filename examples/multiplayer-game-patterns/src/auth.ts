// Token for actor-to-actor communication.
export const INTERNAL_TOKEN = "internal";

export function isInternalToken(params: { internalToken?: string } | null | undefined): boolean {
	return params?.internalToken === INTERNAL_TOKEN;
}

export function hasInvalidInternalToken(
	params: { internalToken?: string } | null | undefined,
): boolean {
	return params?.internalToken !== undefined && params.internalToken !== INTERNAL_TOKEN;
}
