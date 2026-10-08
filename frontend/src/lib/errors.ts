import type { RivetError } from "@rivetkit/engine-api-full";
import { toast } from "@/components";

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

export function isNotFoundError(error: unknown): boolean {
	return isRivetApiError(error) && error.statusCode === 404;
}

export interface ApiFieldError {
	field?: string;
	message: string;
}

export function getApiFieldErrors(error: unknown): ApiFieldError[] {
	if (!isRivetApiError(error)) return [];
	const errors = (error.body as { errors?: unknown } | undefined)?.errors;
	if (!Array.isArray(errors)) return [];
	return errors.filter(
		(e): e is ApiFieldError =>
			typeof e === "object" &&
			e !== null &&
			typeof e.message === "string" &&
			(e.field === undefined || typeof e.field === "string"),
	);
}

export function getStructuredApiErrorMessage(
	error: unknown,
): string | undefined {
	// Error responses without a JSON body, such as a bare 500, have no `body`.
	const bodyMessage = isRivetApiError(error)
		? (error.body as { message?: unknown } | undefined)?.message
		: undefined;
	return typeof bodyMessage === "string" ? bodyMessage : undefined;
}

function getApiErrorMessage(error: unknown): string | undefined {
	return (
		getStructuredApiErrorMessage(error) ??
		(error instanceof Error ? error.message : undefined)
	);
}

export function toastApiError(error: unknown, description?: string) {
	toast.error("Operation failed", {
		description: description ?? getApiErrorMessage(error),
	});
}
