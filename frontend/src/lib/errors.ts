import type { RivetError } from "@rivetkit/engine-api-full";

export function isRivetApiError(
	error: unknown,
): error is RivetError & { body: { message: string } } {
	return (
		typeof error === "object" &&
		error !== null &&
		"statusCode" in error &&
		"message" in error &&
		typeof (error as any).statusCode === "number" &&
		typeof (error as any).message === "string"
	);
}

// Auth errors that should trigger the credentials modal:
// - 403 with no/missing token (api.forbidden)
// - 401 from the ACL system when a stale or invalid token was sent
//   (acl.token_not_found, acl.token_expired)
export function isAuthError(error: unknown): boolean {
	if (!isRivetApiError(error)) return false;
	if (error.statusCode === 403) return true;
	if (error.statusCode !== 401) return false;
	const body = error.body as { group?: unknown; code?: unknown } | undefined;
	return (
		body?.group === "acl" &&
		(body.code === "token_not_found" || body.code === "token_expired")
	);
}
