import type { Clerk } from "@clerk/clerk-js";
import * as Sentry from "@sentry/react";
import { posthog } from "posthog-js";

export function waitForClerk(clerk: Clerk): Promise<void> {
	// Wait for clerk to be ready
	const waitForReady = (): Promise<void> => {
		if (clerk.status === "ready") {
			return Promise.resolve();
		}

		return new Promise((resolve, reject) => {
			const timeout = setTimeout(() => {
				Sentry.captureMessage("Can't confirm identity", "warning");
				reject(new Error("Clerk timeout"));
			}, 10_000);
			clerk.on("status", (payload) => {
				if (payload === "ready") {
					clearTimeout(timeout);
					resolve();
				}
			});
		});
	};

	// Wait for session to be available when user exists (e.g., after SSO callback)
	const waitForSession = (): Promise<void> => {
		// If no user, no session needed
		if (!clerk.user) {
			return Promise.resolve();
		}
		// If session already exists, we're good
		if (clerk.session) {
			return Promise.resolve();
		}

		return new Promise((resolve, reject) => {
			const timeout = setTimeout(() => {
				// Session timeout - user exists but session never became available
				Sentry.captureMessage("Session not available after auth", "warning");
				reject(new Error("Session timeout"));
			}, 10_000);

			// Listen for clerk state changes using addListener
			const unsubscribe = clerk.addListener((resources) => {
				if (resources.session) {
					clearTimeout(timeout);
					unsubscribe();
					resolve();
				}
			});
		});
	};

	return waitForReady()
		.then(waitForSession)
		.then(() => {
			if (clerk.user) {
				identify(clerk);
			}
		});
}

function identify(clerk: Clerk) {
	Sentry.setUser({
		id: clerk.user?.id,
		email: clerk.user?.primaryEmailAddress?.emailAddress,
	});
	posthog.setPersonProperties({
		id: clerk.user?.id,
		email: clerk.user?.primaryEmailAddress?.emailAddress,
	});

	// if (typeof Plain !== "undefined") {
	// 	Plain?.setCustomerDetails({
	// 		clerkId: clerk.user?.id,
	// 		email: clerk.user?.primaryEmailAddress?.emailAddress,
	// 	});
	// }
}
