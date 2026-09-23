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

export function isAuthError(error: unknown): boolean {
	if (!isRivetApiError(error)) return false;
	const body = error.body as { group?: unknown; code?: unknown } | undefined;
	if (error.statusCode === 403) {
		return body?.group === "api" && body.code === "forbidden";
	}
	if (error.statusCode !== 401) return false;
	if (
		body?.group === "auth" &&
		(body.code === "invalid_token" || body.code === "token_expired")
	) {
		return true;
	}
	return (
		body?.group === "acl" &&
		(body.code === "token_not_found" || body.code === "token_expired")
	);
}
