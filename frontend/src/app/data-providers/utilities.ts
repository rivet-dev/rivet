export const no404Retry = <TError extends { statusCode?: number }>(
	options: {
		retry?:
			| boolean
			| number
			| ((failureCount: number, error: TError) => boolean);
	} = {},
) => {
	return {
		...options,
		retry: (failureCount: number, error: TError) => {
			if ("statusCode" in error && error.statusCode === 404) {
				return false;
			}
			if (typeof options.retry === "function") {
				return options.retry(failureCount, error);
			}
			if (typeof options.retry === "boolean") {
				return options.retry;
			}
			if (typeof options.retry === "number") {
				return failureCount < options.retry;
			}
			return failureCount < 3; // default retry behavior
		},
	};
};
