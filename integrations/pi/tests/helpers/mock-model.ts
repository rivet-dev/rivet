import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
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
import { ModelRuntime } from "@earendil-works/pi-coding-agent";

export type PiModel = NonNullable<ReturnType<ModelRuntime["getModel"]>>;

/** A scripted model reply. A `tokensPerSecond` reply streams slowly, for tests that act during a run. */
export type MockReply = AssistantMessage | { message: AssistantMessage; tokensPerSecond: number };

export interface MockModel {
	model: PiModel;
	modelRuntime: ModelRuntime;
	/**
	 * Scripts the replies to a user message. The model calls of one prompt take
	 * them in order, including tool follow-ups and retries; the last one repeats.
	 */
	reply(userText: string, ...replies: MockReply[]): void;
	dispose(): Promise<void>;
}

/** A reply that calls one tool. */
export function toolCall(name: string, args: ToolCall["arguments"]): AssistantMessage {
	return fauxAssistantMessage(fauxToolCall(name, args), { stopReason: "toolUse" });
}

/** A reply that streams at 25 tokens a second. */
export function slowly(message: AssistantMessage): MockReply {
	return { message, tokensPerSecond: 25 };
}

/**
 * Pi's faux provider as provider `mock`, in a `ModelRuntime` whose
 * credentials live in a temp dir, so the test never touches `~/.pi`.
 */
export async function startMockModel(): Promise<MockModel> {
	const mock = createMockStream();
	const agentDir = await mkdtemp(join(tmpdir(), "rivet-pi-test-agent-"));
	const modelRuntime = await ModelRuntime.create({
		authPath: join(agentDir, "auth.json"),
		modelsPath: null,
	});
	modelRuntime.registerProvider("mock", {
		baseUrl: "http://localhost:0",
		api: "mock",
		apiKey: "mock",
		streamSimple: mock.streamSimple,
		models: [MOCK_MODEL],
	});
	await modelRuntime.refresh({ allowNetwork: false });
	const model = modelRuntime.getModel("mock", MOCK_MODEL.id);
	if (!model) throw new Error("mock model was not registered");
	return {
		model,
		modelRuntime,
		reply: mock.reply,
		dispose: () => rm(agentDir, { recursive: true, force: true }),
	};
}

const MOCK_MODEL = {
	id: "mock-model",
	name: "mock-model",
	reasoning: false,
	input: ["text" as const],
	cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
	contextWindow: 128_000,
	maxTokens: 4_096,
};

function createMockStream() {
	const replies = new Map<string, MockReply[]>();
	const calls = new Map<string, number>();
	const cores = new Map<number, ReturnType<typeof createFauxCore>>();
	const coreFor = (tokensPerSecond: number) => {
		let core = cores.get(tokensPerSecond);
		if (!core) {
			core = createFauxCore({ api: "mock", provider: "mock", models: [MOCK_MODEL], tokensPerSecond });
			cores.set(tokensPerSecond, core);
		}
		return core;
	};
	return {
		reply: (userText: string, ...scripted: MockReply[]) => {
			replies.set(userText, scripted);
		},
		streamSimple: (model: PiModel, context: TranscriptContext, options?: SimpleStreamOptions) => {
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
	};
}

function textOf(message: Message | undefined): string {
	if (message?.role !== "user") return "";
	if (typeof message.content === "string") return message.content;
	return message.content.map((part) => (part.type === "text" ? part.text : "")).join("");
}
