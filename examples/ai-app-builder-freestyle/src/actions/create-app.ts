"use server";

import { getRivetClient } from "@/rivet/server";
import { freestyle } from "@/lib/freestyle";
import { templates } from "@/lib/templates";
import { AIService } from "@/lib/internal/ai-service";

export async function createApp({
	appId,
	templateId,
	initialMessage,
}: {
	appId: string;
	templateId: string;
	initialMessage?: string;
}) {
	console.log("[createApp] Starting app creation", { appId, templateId, initialMessage });

	if (!appId) {
		throw new Error("App ID is required");
	}

	if (!templateId || !templates[templateId]) {
		throw new Error(`Invalid template: ${templateId}`);
	}

	// Create git repository
	console.log("[createApp] Creating git repository...");
	const repo = await freestyle.createGitRepository({
		name: initialMessage || "Unnamed App",
		public: true,
		source: {
			type: "git",
			url: templates[templateId].repo,
		},
	});
	console.log("[createApp] Git repository created", { repoId: repo.repoId });

	// Create git identity for this app
	console.log("[createApp] Creating git identity...");
	const gitIdentity = await freestyle.createGitIdentity();
	console.log("[createApp] Git identity created", { identityId: gitIdentity.id });

	// Grant write permission
	console.log("[createApp] Granting git permission...");
	await freestyle.grantGitPermission({
		identityId: gitIdentity.id,
		repoId: repo.repoId,
		permission: "write",
	});
	console.log("[createApp] Git permission granted");

	// Create access token
	console.log("[createApp] Creating git access token...");
	const token = await freestyle.createGitAccessToken({
		identityId: gitIdentity.id,
	});
	console.log("[createApp] Git access token created", { tokenId: token.id });

	// Request dev server
	console.log("[createApp] Requesting dev server...");
	const { mcpEphemeralUrl, fs } = await freestyle.requestDevServer({
		repoId: repo.repoId,
	});
	console.log("[createApp] Dev server ready", { mcpEphemeralUrl });

	// Create the app in the appStore actor
	console.log("[createApp] Creating app in appStore actor...");
	const client = getRivetClient();
	const appActor = client.appStore.getOrCreate([appId]);
	const appInfo = await appActor.createApp({
		name: initialMessage || "Unnamed App",
		description: "No description",
		gitRepo: repo.repoId,
		baseId: "nextjs-dkjfgdf",
		previewDomain: null,
		freestyleIdentity: gitIdentity.id,
		freestyleAccessToken: token.token,
		freestyleAccessTokenId: token.id,
	});
	console.log("[createApp] App created in appStore", { appId: appInfo.id, name: appInfo.name });

	// If there's an initial message, send it to the AI (simple request/response, no streaming)
	if (initialMessage) {
		console.log("[createApp] Sending initial message to AI...");
		const streamActor = client.streamState.getOrCreate([appId]);
		await streamActor.setRunning();
		console.log("[createApp] Stream state set to running");

		try {
			console.log("[createApp] Calling AIService.sendMessage...");
			await AIService.sendMessage(
				appId,
				mcpEphemeralUrl,
				fs,
				{
					id: crypto.randomUUID(),
					parts: [
						{
							text: initialMessage,
							type: "text",
						},
					],
					role: "user",
				},
				[], // No previous messages for initial creation
				{
					maxSteps: 100,
					maxOutputTokens: 64000,
				}
			);
			console.log("[createApp] AIService.sendMessage completed successfully");

			await streamActor.clear();
			console.log("[createApp] Stream state cleared");
		} catch (error) {
			console.error("[createApp] AIService.sendMessage failed:", error);
			await streamActor.clear();
			throw error;
		}
	}

	console.log("[createApp] App creation complete, returning result", { appId: appInfo.id });
	return {
		id: appInfo.id,
		name: appInfo.name,
		gitRepo: appInfo.gitRepo,
	};
}
