import type { Query } from "@tanstack/react-query";

export const shouldRetryAllExpect403 = (failureCount: number, error: Error) => {
	if (error && "statusCode" in error) {
		if (error.statusCode === 403) {
			// Don't retry on auth errors
			return false;
		}
	}

	if (failureCount >= 3) {
		return false;
	}

	return true;
};

export const throwAllExpect403 = <T extends Query<any, any, any, any>>(
	error: Error,
	_query: T,
) => {
	if (error && "statusCode" in error) {
		if (error.statusCode === 403) {
			// Don't throw on auth errors
			return false;
		}
	}

	return true;
};
