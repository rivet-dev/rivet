"use server";

import { getApp } from "@/actions/get-app";
import { freestyle } from "@/lib/freestyle";
import { UIMessage, type CoreMessage } from "ai";
import { getRivetClient } from "@/rivet/server";
import { AIService } from "@/lib/internal/ai-service";

/**
 * Server action to send a chat message and get a response.
 * Simple request/response - no streaming.
 */
export async function sendChatMessage(
	appId: string,
	message: UIMessage
): Promise<UIMessage> {
	console.log("[sendChatMessage] Starting...", { appId, messageId: message.id });

	const client = getRivetClient();

	console.log("[sendChatMessage] Getting app...");
	const app = await getApp(appId);
	if (!app) {
		throw new Error("App not found");
	}
	console.log("[sendChatMessage] App found", { appName: app.info?.name });

	// Check if a stream is already running
	console.log("[sendChatMessage] Checking stream status...");
	const streamActor = client.streamState.getOrCreate([appId]);
	const status = await streamActor.getStatus();
	console.log("[sendChatMessage] Stream status:", status);

	if (status === "running") {
		// Stop previous stream
		console.log("[sendChatMessage] Aborting previous stream...");
		await streamActor.abort();
		// Wait a bit for cleanup
		await new Promise((resolve) => setTimeout(resolve, 500));
	}

	// Mark as running
	console.log("[sendChatMessage] Setting stream to running...");
	await streamActor.setRunning();

	try {
		// Get dev server
		console.log("[sendChatMessage] Requesting dev server...");
		const { mcpEphemeralUrl, fs } = await freestyle.requestDevServer({
			repoId: app.info.gitRepo,
		});
		console.log("[sendChatMessage] Dev server ready", { mcpEphemeralUrl });

		// Get previous messages from the app store
		console.log("[sendChatMessage] Getting previous messages...");
		const appActor = client.appStore.getOrCreate([appId]);
		const storedMessages = await appActor.getMessages();
		console.log("[sendChatMessage] Got previous messages", { count: storedMessages.length });

		// Convert UIMessages to CoreMessage format expected by the AI service
		const previousMessages: CoreMessage[] = storedMessages.map((m: UIMessage) => {
			const content = m.parts
				.map((part) => {
					if (part.type === "text") {
						return part.text;
					}
					return "";
				})
				.join("");
			return { role: m.role as "user" | "assistant", content };
		});

		// Send message to AI
		console.log("[sendChatMessage] Calling AIService.sendMessage...");
		const response = await AIService.sendMessage(
			appId,
			mcpEphemeralUrl,
			fs,
			message,
			previousMessages,
			{
				maxSteps: 100,
				maxOutputTokens: 64000,
			}
		);
		console.log("[sendChatMessage] AIService.sendMessage completed", { responseTextLength: response.text?.length || 0 });

		// Clear running status
		console.log("[sendChatMessage] Clearing stream status...");
		await streamActor.clear();

		// Create assistant message from response
		const assistantMessage: UIMessage = {
			id: crypto.randomUUID(),
			role: "assistant",
			parts: [
				{
					type: "text",
					text: response.text,
				},
			],
		};

		// Save both messages to the app store
		console.log("[sendChatMessage] Saving messages to app store...");
		await appActor.addMessage(message);
		await appActor.addMessage(assistantMessage);
		console.log("[sendChatMessage] Messages saved");

		console.log("[sendChatMessage] Returning assistant message");
		return assistantMessage;
	} catch (error) {
		console.error("[sendChatMessage] Error occurred:", error);
		await streamActor.clear();
		throw error;
	}
}
