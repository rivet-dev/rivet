import { actor } from "rivetkit";
import type { AppDeployment, UIMessage } from "../../shared/types";
import type { CoreMessage } from "ai";
import { sendMessage } from "../ai-service";
import { requestDevServer as requestDevServerFromFreestyle, freestyle } from "../freestyle";

/**
 * Input for creating a new UserApp actor
 */
export interface UserAppInput {
	name: string;
	description: string;
	gitRepo: string;
	templateId: string;
}

/**
 * State for the UserApp actor
 */
export interface UserAppState {
	id: string;
	name: string;
	description: string;
	gitRepo: string;
	templateId: string;
	createdAt: number;
	previewDomain?: string;
	freestyleIdentity?: string;
	freestyleAccessToken?: string;
	freestyleAccessTokenId?: string;
	messages: UIMessage[];
	deployments: AppDeployment[];
}

/**
 * UserApp actor - stores data for a single user app and handles all app operations
 * Each app gets its own actor instance keyed by app ID
 *
 * This actor combines the functionality of:
 * - app info, messages, deployments
 * - stream/generation status
 * - AI chat interactions
 * - dev server management (per-app)
 */
export const userApp = actor({
	options: {
		actionTimeout: 10 * 60 * 1000,
	},

	// Ephemeral variables for non-serializable objects like AbortController
	// Also includes streamStatus and streamLastUpdate which shouldn't persist between restarts
	createVars: () => ({
		abortController: null as AbortController | null,
		streamStatus: undefined as string | undefined,
		streamLastUpdate: 0,
	}),

	// Initialize state from input when the actor is created
	createState: (c, input: UserAppInput): UserAppState => ({
		// App info - flat properties from input plus derived fields
		id: c.key[0] as string,
		name: input.name,
		description: input.description,
		gitRepo: input.gitRepo,
		templateId: input.templateId,
		createdAt: Date.now(),
		// Optional properties are omitted initially and set later when needed:
		// previewDomain, freestyleIdentity, freestyleAccessToken, freestyleAccessTokenId
		// Chat and deployment state
		messages: [],
		deployments: [],
	}),

	actions: {
		// ==================
		// App Info Actions
		// ==================
		getInfo: (c) => ({
			id: c.state.id,
			name: c.state.name,
			createdAt: c.state.createdAt,
		}),

		// ==================
		// Message Actions
		// ==================
		addMessage: (c, message: UIMessage) => {
			c.state.messages.push(message);
			c.broadcast("newMessage", message);
			return message;
		},

		getMessages: (c) => c.state.messages,

		clearMessages: (c) => {
			c.state.messages = [];
			return { success: true };
		},

		// ==================
		// Deployment Actions
		// ==================
		addDeployment: (c, deployment: Omit<AppDeployment, "createdAt">) => {
			const appDeployment: AppDeployment = {
				...deployment,
				createdAt: Date.now(),
			};
			c.state.deployments.push(appDeployment);
			return appDeployment;
		},

		getDeployments: (c) => c.state.deployments,

		getAll: (c) => ({
			info: {
				id: c.state.id,
				name: c.state.name,
				createdAt: c.state.createdAt,
				gitRepo: c.state.gitRepo,
				previewDomain: c.state.previewDomain,
			},
			messages: c.state.messages,
			deployments: c.state.deployments,
		}),

		// ==================
		// Stream State Actions
		// ==================
		getStreamStatus: (c) => {
			if (c.vars.streamStatus && Date.now() - c.vars.streamLastUpdate > 15000) {
				c.vars.streamStatus = undefined;
			}
			return c.vars.streamStatus;
		},

		abortStream: (c) => {
			// Abort the current AI request if one is running
			if (c.vars.abortController) {
				c.vars.abortController.abort();
				c.vars.abortController = null;
			}
			c.broadcast("abort");
			c.vars.streamStatus = undefined;
			return { success: true };
		},

		// ==================
		// Chat Agent Actions
		// ==================
		/**
		 * Send a chat message and get an AI response.
		 * This action handles everything internally:
		 * - Adds the user message to state
		 * - Sets stream status to running
		 * - Calls the AI service
		 * - Adds the assistant message to state
		 * - Clears stream status
		 * - Broadcasts newMessage events
		 */
		sendChatMessage: async (
			c,
			{ message }: { message: UIMessage },
		): Promise<UIMessage> => {
			const gitRepo = c.state.gitRepo;

			// Add user message to state and broadcast (only if not already present)
			const existingMessage = c.state.messages.find((m) => m.id === message.id);
			if (!existingMessage) {
				c.state.messages.push(message);
				c.broadcast("newMessage", message);
			}

			// Set stream status to running
			c.vars.streamStatus = "running";
			c.vars.streamLastUpdate = Date.now();

			// Create abort controller for this request
			const abortController = new AbortController();
			c.vars.abortController = abortController;

			try {
				// Get dev server
				console.log(
					"[appStore.sendChatMessage] Requesting dev server for repo:",
					gitRepo,
				);
				const devServerResult = await requestDevServerFromFreestyle({
					repoId: gitRepo,
				});
				console.log("[appStore.sendChatMessage] Dev server ready:", {
					mcpEphemeralUrl: devServerResult.mcpEphemeralUrl,
				});
				const { mcpEphemeralUrl, fs } = devServerResult;

				// Convert previous messages to CoreMessage format (excluding the new user message)
				// Filter out messages with empty content (can happen from failed/aborted streams)
				const previousMessages = c.state.messages.slice(0, -1);
				console.log("[appStore.sendChatMessage] Converting messages...");
				const coreMessages: CoreMessage[] = previousMessages
					.map((m: UIMessage) => {
						const content = m.parts
							.map((part) => {
								if (part.type === "text") {
									return part.text;
								}
								return "";
							})
							.join("");
						return { role: m.role as "user" | "assistant", content };
					})
					.filter((m) => m.content.trim() !== "");
				console.log(
					"[appStore.sendChatMessage] Converted",
					coreMessages.length,
					"messages",
				);

				// Send message to AI with abort signal
				console.log(
					"[appStore.sendChatMessage] Calling sendMessage...",
				);
				// Create a message ID that will be used for streaming updates
				const assistantMessageId = crypto.randomUUID();

				const response = await sendMessage(
					c.key[0] as string,
					mcpEphemeralUrl,
					fs,
					message,
					coreMessages,
					{
						maxSteps: 100,
						maxOutputTokens: 64000,
						abortSignal: abortController.signal,
						onStepUpdate: (text: string) => {
							// Broadcast step updates so the frontend can show progress
							c.broadcast("stepUpdate", {
								id: assistantMessageId,
								text,
							});
						},
					},
				);
				console.log(
					"[appStore.sendChatMessage] sendMessage completed",
					{
						responseTextLength: response.text?.length || 0,
					},
				);

				// Create assistant message from response (using the same ID from step updates)
				const assistantMessage: UIMessage = {
					id: assistantMessageId,
					role: "assistant",
					parts: [
						{
							type: "text",
							text: response.text,
						},
					],
				};

				// Add assistant message to state and broadcast
				c.state.messages.push(assistantMessage);
				c.broadcast("newMessage", assistantMessage);

				console.log("[appStore.sendChatMessage] === ACTION COMPLETED ===");
				console.log(
					"[appStore.sendChatMessage] Returning assistant message:",
					assistantMessage.id,
				);
				return assistantMessage;
			} catch (err) {
				// Check if this was an abort error
				if (err instanceof Error && err.name === "AbortError") {
					console.log("[appStore.sendChatMessage] Request was aborted");
					const abortedMessage: UIMessage = {
						id: crypto.randomUUID(),
						role: "assistant",
						parts: [
							{
								type: "text",
								text: "Generation stopped.",
							},
						],
					};
					c.state.messages.push(abortedMessage);
					c.broadcast("newMessage", abortedMessage);
					return abortedMessage;
				}

				console.error(
					"[appStore.sendChatMessage] Error:",
					err,
				);
				// Add error message to state and broadcast
				const errorMessage: UIMessage = {
					id: crypto.randomUUID(),
					role: "assistant",
					parts: [
						{
							type: "text",
							text: `Error: ${err instanceof Error ? err.message : "Failed to get AI response"}`,
						},
					],
				};
				c.state.messages.push(errorMessage);
				c.broadcast("newMessage", errorMessage);
				return errorMessage;
			} finally {
				// Clear stream status and abort controller
				c.vars.streamStatus = undefined;
				c.vars.abortController = null;
			}
		},

		// ==================
		// Dev Server Actions
		// ==================
		requestDevServer: async (c) => {
			const result = await requestDevServerFromFreestyle({
				repoId: c.state.gitRepo,
			});
			// Only return serializable data - fs is an object with methods that can't be serialized
			return {
				ephemeralUrl: result.ephemeralUrl,
				mcpEphemeralUrl: result.mcpEphemeralUrl,
				devCommandRunning: result.devCommandRunning,
				installCommandRunning: result.installCommandRunning,
				codeServerUrl: result.codeServerUrl,
				consoleUrl: result.consoleUrl,
			};
		},

		publishApp: async (c, { domain }: { domain: string }) => {
			const result = await freestyle.deployWeb(
				{
					kind: "git",
					url: `https://git.freestyle.sh/${c.state.gitRepo}`,
				},
				{
					build: true,
					domains: [domain],
				}
			);
			c.state.previewDomain = domain;
			return {
				domain,
				deploymentId: result.deploymentId,
			};
		},
	},
});
