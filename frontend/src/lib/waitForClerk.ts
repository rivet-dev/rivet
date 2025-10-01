import type { Clerk } from "@clerk/clerk-js";
import * as Sentry from "@sentry/react";
import { posthog } from "posthog-js";

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
		clerk.on("status", (payload) => {
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
	posthog.setPersonProperties({
		id: clerk.user?.id,
		email: clerk.user?.primaryEmailAddress?.emailAddress,
	});

	Plain.setCustomerDetails({
		clerkId: clerk.user?.id,
		email: clerk.user?.primaryEmailAddress?.emailAddress,
	});
}
