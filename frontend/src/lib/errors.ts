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
