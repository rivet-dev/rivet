import type { Query } from "@tanstack/react-query";

export const shouldRetryAllExpect403 = (failureCount: number, error: Error) => {
	if (error && "statusCode" in error) {
		if (error.statusCode === 403 || error.statusCode === 401) {
			// Don't retry on auth errors, when app is not engine
			return __APP_TYPE__ !== "engine";
		}
		if (error.statusCode === 400) {
			return false;
		}
	}

	if (failureCount >= 3) {
		return false;
	}

	return true;
};

export const noThrow = <T extends Query<any, any, any, any>>(error: Error) => {
	return false;
};
