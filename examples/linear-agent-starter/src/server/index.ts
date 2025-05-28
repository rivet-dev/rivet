import { Hono } from "hono";
import { serve } from "@hono/node-server";
import { app, type App, actorClient } from "../actors/app";
import { createRouter } from "@actor-core/nodejs";
import crypto from "node:crypto";
import { LinearClient } from "@linear/sdk";
import * as openidClient from "openid-client";
import { atob } from "node:buffer";
import type { OAuthExpectedState } from "../actors/oauth-session";
import {
	BASE_PATH,
	LINEAR_OAUTH_CLIENT_AUTHENTICATION,
	LINEAR_OAUTH_CLIENT_ID,
	LINEAR_OAUTH_REDIRECT_URI,
	LINEAR_WEBHOOK_SECRET,
	PORT,
} from "../config";
import type { LinearWebhookEvent } from "../linear-types";

// Create Hono app
const router = new Hono();

// Mount ActorCore
const { router: actorRouter, injectWebSocket } = createRouter(app);
router.route(BASE_PATH, actorRouter);

// Setup OAuth
const openidConfig = new openidClient.Configuration(
	{
		issuer: "https://linear.app",
		authorization_endpoint: "https://linear.app/oauth/authorize",
		token_endpoint: "https://api.linear.app/oauth/token",
	},
	LINEAR_OAUTH_CLIENT_ID,
	{
		client_secret: LINEAR_OAUTH_CLIENT_AUTHENTICATION,
	},
);

// Step 1: Connect Linear
router.get("/connect-linear", async (c) => {
	// Setup session
	//
	// Nonce is required to verify that we generated this request
	const sessionId = crypto.randomUUID();
	const nonce = openidClient.randomNonce();
	const oauthState = btoa(
		JSON.stringify({ sessionId, nonce } satisfies OAuthExpectedState),
	);

	await actorClient.oauthSession.create(sessionId, {
		input: { nonce, oauthState },
	});

	const parameters: Record<string, string> = {
		redirect_uri: LINEAR_OAUTH_REDIRECT_URI,

		state: oauthState,

		// See https://linear.app/developers/agents#actor-and-scopes
		//
		// app:assignable = lets tickets be assigned to your agent
		// app:mentionable = lets your agent be mentioned
		scope: "read write app:assignable app:mentionable",

		// IMPORTANT: Changes the authentication type
		actor: "app",
	};

	const redirectTo: URL = openidClient.buildAuthorizationUrl(
		openidConfig,
		parameters,
	);

	console.log("Redirecting to OAuth", redirectTo);

	return c.redirect(redirectTo.href);
});

// Step 2:
// GET https://example.com/oauth/callback?code=9a5190f637d8b1ad0ca92ab3ec4c0d033ad6c862&state=b1ad0ca92 HTTP/1.1
router.get("/oauth/callback/linear", async (c) => {
	const stateRaw = c.req.query("state");
	const state = JSON.parse(atob(stateRaw!)) as OAuthExpectedState;

	// Validate that the OAuth session has the same nonce
	const expectedState = await actorClient.oauthSession
		.get(state.sessionId)
		.getOAuthState();

	console.log("Validating tokens");
	const tokens: openidClient.TokenEndpointResponse =
		await openidClient.authorizationCodeGrant(
			openidConfig,
			// HACK: Fix protocol when hosting locally
			new URL(c.req.url.replace("http://", "https://")),
			{
				expectedState,
			},
		);

	console.log("Fetching app user ID");
	const linearClient = new LinearClient({ accessToken: tokens.access_token });
	const viewer = await linearClient.viewer;
	const appUserId = viewer.id;

	console.log(`Saving access token ${appUserId}`);
	await actorClient.linearAppUser
		.getOrCreate(appUserId)
		.setAccessToken(tokens.access_token);

	return c.text(`Successfully linked with app user ID ${appUserId}`);
});

// Step 3: Receive events from Linear
router.post("/webhook/linear", async (c) => {
	const rawBody = await c.req.text();

	// Verify signature
	const signature = c.req.header("linear-signature");
	const computedSignature = crypto
		.createHmac("sha256", LINEAR_WEBHOOK_SECRET)
		.update(rawBody)
		.digest("hex");
	if (signature !== computedSignature) {
		throw new Error("Signature does not match");
	}

	// Parse event
	const event: LinearWebhookEvent = JSON.parse(rawBody);
	console.log(
		`received linear webhook: ${event.appUserId} - ${event.type} - ${event.action}`,
	);

	// Create actor (if needed) and sendt o actor
	if (event.type === "AppUserNotification") {
		console.log("App user event", JSON.parse(rawBody));
		const notification = event.notification;
		const issueId = event.notification.issueId;

		// Get issue agent
		const issueAgent = actorClient.issueAgent.getOrCreate(issueId, {
			createWithInput: { issueId },
		});

		// Forward event
		switch (notification.type) {
			case "issueMention":
				issueAgent.issueMention(event.appUserId, notification.issue);
				break;
			case "issueEmojiReaction":
				issueAgent.issueEmojiReaction(
					event.appUserId,
					notification.issue,
					notification.reactionEmoji || "",
				);
				break;
			case "issueCommentMention":
				issueAgent.issueCommentMention(
					event.appUserId,
					notification.issue,
					notification.comment!,
				);
				break;
			case "issueCommentReaction":
				issueAgent.issueCommentReaction(
					event.appUserId,
					notification.issue,
					notification.comment!,
					notification.reactionEmoji!,
				);
				break;
			case "issueAssignedToYou":
				issueAgent.issueAssignedToYou(
					event.appUserId,
					notification.issue,
				);
				break;
			case "issueUnassignedFromYou":
				issueAgent.issueUnassignedFromYou(
					event.appUserId,
					notification.issue,
				);
				break;
			case "issueNewComment":
				issueAgent.issueNewComment(
					event.appUserId,
					notification.issue,
					notification.comment!,
				);
				break;
			case "issueStatusChanged":
				issueAgent.issueStatusChanged(
					event.appUserId,
					notification.issue,
				);
				break;
			default:
				console.warn(
					`Unknown notification event: ${event.type} - ${event.action}`,
				);
		}
	} else {
		console.warn(`Unknown webhook event: ${event.type} - ${event.action}`);
	}

	return c.text("ok");
});

router.get("/health", (c) => {
	return c.text("ok");
});

// Start the server
const server = serve(
	{
		fetch: router.fetch,
		port: PORT,
	},
	(info) => {
		console.log(`Running on port ${info.port}`);
		console.log(
			`Start by visiting http://127.0.0.1:${info.port}/connect-linear`,
		);
	},
);
injectWebSocket(server);
