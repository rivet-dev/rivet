import type { Clerk } from "@clerk/clerk-js";
import * as Sentry from "@sentry/react";

export function waitForClerk(clerk: Clerk): Promise<void> {
	if (clerk.status === "ready") {
		identify(clerk);
		return Promise.resolve();
	}

	return new Promise((resolve, reject) => {
		const timeout = setTimeout(() => {
			Sentry.captureMessage("Can't confirm identity", "warning");
			reject(new Error("Clerk timeout"));
		}, 10_000);
		clerk.on("status", (payload: Clerk["status"]) => {
			if (payload === "ready") {
				clearTimeout(timeout);
				if (clerk.user) {
					identify(clerk);
				}
				resolve();
			}
		});
	});
}

function identify(clerk: Clerk) {
	Sentry.setUser({
		id: clerk.user?.id,
		email: clerk.user?.primaryEmailAddress?.emailAddress,
	});
	// Dynamic import guarded by app type so posthog-js is not in the engine build.
	if (__APP_TYPE__ === "cloud") {
		import("posthog-js").then(({ default: posthog }) => {
			posthog.setPersonProperties({
				id: clerk.user?.id,
				email: clerk.user?.primaryEmailAddress?.emailAddress,
			});
		});
	}

	// if (typeof Plain !== "undefined") {
	// 	Plain?.setCustomerDetails({
	// 		clerkId: clerk.user?.id,
	// 		email: clerk.user?.primaryEmailAddress?.emailAddress,
	// 	});
	// }
}
