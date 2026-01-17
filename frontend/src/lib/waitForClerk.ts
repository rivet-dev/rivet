import type { Clerk } from "@clerk/clerk-js";
import * as Sentry from "@sentry/react";
import { posthog } from "posthog-js";

export function waitForClerk(clerk: Clerk): Promise<void> {
	const logState = (context: string) => {
		console.log(`[waitForClerk] ${context}`, {
			status: clerk.status,
			hasUser: !!clerk.user,
			userId: clerk.user?.id,
			hasSession: !!clerk.session,
			sessionId: clerk.session?.id,
		});
	};

	// Check if clerk is fully ready with all authentication state populated
	const isFullyReady = (): boolean => {
		// Must be in ready status
		if (clerk.status !== "ready") {
			return false;
		}
		// If there's a user, there should also be a session
		// This handles the race condition after SSO where user might be set before session
		if (clerk.user && !clerk.session) {
			return false;
		}
		return true;
	};

	logState("initial check");

	// Already fully ready
	if (isFullyReady()) {
		console.log("[waitForClerk] already fully ready, resolving immediately");
		if (clerk.user) {
			identify(clerk);
		}
		return Promise.resolve();
	}

	console.log("[waitForClerk] not ready, setting up listeners");

	return new Promise((resolve, reject) => {
		const timeout = setTimeout(() => {
			logState("timeout reached");
			Sentry.captureMessage("Can't confirm identity", "warning");
			reject(new Error("Clerk timeout"));
		}, 10_000);

		let resolved = false;

		// Listen for both status changes and resource changes
		const checkAndResolve = () => {
			if (resolved) return;
			logState("checkAndResolve called");
			if (isFullyReady()) {
				console.log("[waitForClerk] now fully ready, resolving");
				resolved = true;
				clearTimeout(timeout);
				unsubscribeResources();
				if (clerk.user) {
					identify(clerk);
				}
				resolve();
			}
		};

		// Listen for status changes (for initial "ready" state)
		// Note: clerk.on() doesn't return an unsubscribe function
		clerk.on("status", (status) => {
			console.log("[waitForClerk] status event:", status);
			checkAndResolve();
		});

		// Listen for resource changes (for user/session becoming available)
		const unsubscribeResources = clerk.addListener((resources) => {
			console.log("[waitForClerk] resources event:", {
				hasUser: !!resources.user,
				hasSession: !!resources.session,
			});
			checkAndResolve();
		});

		// Check immediately in case state changed between initial check and listener setup
		checkAndResolve();
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
