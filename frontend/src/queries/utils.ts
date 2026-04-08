import type { Query } from "@tanstack/react-query";
import { features } from "@/lib/features";

export const shouldRetryAllExpect403 = (failureCount: number, error: Error) => {
	if (error && "statusCode" in error) {
		if (error.statusCode === 403 || error.statusCode === 401) {
			// Retry on auth errors when auth is enabled (auth system handles the redirect)
			return features.auth;
		}
		if (error.statusCode === 404) {
			// Don't retry on not found errors, as they are unlikely to succeed on retry
			return false;
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
