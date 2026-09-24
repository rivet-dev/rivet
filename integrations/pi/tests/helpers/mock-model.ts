import {
	type AssistantMessage,
	createFauxCore,
	fauxAssistantMessage,
	fauxToolCall,
	type Message,
	type SimpleStreamOptions,
	type ToolCall,
	type TranscriptContext,
} from "@earendil-works/pi-ai";
import type { PiProviderConfig } from "../../src/index.js";

/** Model ids the mock provider serves. */
export const MOCK_MODEL_IDS = ["mock-model", "mock-model-2", "mock-model-3"] as const;

/** A scripted model reply. A `tokensPerSecond` reply streams slowly, for tests that act during a run. */
export type MockReply = AssistantMessage | { message: AssistantMessage; tokensPerSecond: number };

export interface MockModel {
	/** Pi's faux provider as provider `mock`, for `pi({ providers })`. */
	providerConfig: PiProviderConfig;
	/**
	 * Scripts the replies to a user message. The model calls of one prompt take
	 * them in order, including tool follow-ups and retries; the last one repeats.
	 */
	reply(userText: string, ...replies: MockReply[]): void;
	/** Every model call: the model id and the API key Pi resolved for it. */
	requests: { model: string; apiKey: string | undefined }[];
}

/** A reply that calls one tool. */
export function toolCall(name: string, args: ToolCall["arguments"]): AssistantMessage {
	return fauxAssistantMessage(fauxToolCall(name, args), { stopReason: "toolUse" });
}

/** A reply that streams at 25 tokens a second. */
export function slowly(message: AssistantMessage): MockReply {
	return { message, tokensPerSecond: 25 };
}

/** Pi's faux provider, which answers in-process with scripted replies. */
export function createMockModel(): MockModel {
	const models = MOCK_MODEL_IDS.map((id) => ({
		id,
		name: id,
		reasoning: false,
		input: ["text" as const],
		cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
		contextWindow: 128_000,
		maxTokens: 4_096,
	}));
	const replies = new Map<string, MockReply[]>();
	const calls = new Map<string, number>();
	const requests: MockModel["requests"] = [];
	const cores = new Map<number, ReturnType<typeof createFauxCore>>();
	const coreFor = (tokensPerSecond: number) => {
		let core = cores.get(tokensPerSecond);
		if (!core) {
			core = createFauxCore({ api: "mock", provider: "mock", models, tokensPerSecond });
			cores.set(tokensPerSecond, core);
		}
		return core;
	};
	return {
		providerConfig: {
			baseUrl: "http://localhost:0",
			api: "mock",
			models,
			streamSimple: (model, context: TranscriptContext, options?: SimpleStreamOptions) => {
				requests.push({ model: model.id, apiKey: options?.apiKey });
				const prompt = context.messages.findLast((message) => message.role === "user");
				const text = textOf(prompt);
				const key = `${text}@${prompt?.timestamp}`;
				const call = calls.get(key) ?? 0;
				calls.set(key, call + 1);
				const scripted = replies.get(text);
				const reply =
					scripted?.[Math.min(call, scripted.length - 1)] ??
					fauxAssistantMessage("", { stopReason: "error", errorMessage: `no mock reply for "${text}"` });
				const { message, tokensPerSecond } = "message" in reply ? reply : { message: reply, tokensPerSecond: 0 };
				const core = coreFor(tokensPerSecond);
				core.setResponses([message]);
				return core.streamSimple(model, context, options);
			},
		},
		reply: (userText, ...scripted) => {
			replies.set(userText, scripted);
		},
		requests,
	};
}

function textOf(message: Message | undefined): string {
	if (message?.role !== "user") return "";
	if (typeof message.content === "string") return message.content;
	return message.content.map((part) => (part.type === "text" ? part.text : "")).join("");
}
