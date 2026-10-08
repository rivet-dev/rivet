const GENERIC_DESCRIPTION =
	"An unexpected error occurred. Please try again later.";

function field(error: unknown, key: string): unknown {
	return typeof error === "object" && error !== null && key in error
		? (error as Record<string, unknown>)[key]
		: undefined;
}

export function describeError(error: unknown): {
	title: string;
	description: string;
} {
	if (field(error, "statusCode") === 404) {
		return {
			title: "Resource not found",
			description:
				"The resource you are looking for does not exist or you do not have access to it.",
		};
	}

	const bodyMessage = field(field(error, "body"), "message");
	const description = field(error, "description");
	const message =
		(typeof bodyMessage === "string" && bodyMessage) ||
		(typeof description === "string" && description) ||
		(error instanceof Error && error.message) ||
		GENERIC_DESCRIPTION;

	return { title: "Something went wrong", description: message };
}
