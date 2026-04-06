import { anthropic } from "@ai-sdk/anthropic";
import type { DurableStream } from "@durable-streams/client";
import { streamText } from "ai";
import { type ActorContextOf, actor, setup, event } from "rivetkit";
import { getStreams } from "./shared/streams.ts";
import type { PromptMessage, ResponseChunk } from "./shared/types.ts";

interface State {
	conversationId: string;
	promptStreamOffset: string | undefined;
}

export const aiAgent = actor({
	createState: (_c, input: { conversationId: string }): State => ({
		conversationId: input.conversationId,
		// Offset tracking for durable stream consumption
		// undefined means start from the beginning of the stream
		promptStreamOffset: undefined as string | undefined,
	}),
	events: {
		responseError: event<{ promptId: string; error: string }>(),
		responseComplete: event<{ promptId: string; fullResponse: string }>(),
	},

	onWake: (c) => {
		consumeStream(c);
	},

	actions: {
		getConversationId: (c) => c.state.conversationId,
		getPromptStreamOffset: (c) => c.state.promptStreamOffset,
	},

	options: {
		// IMPORTANT: Keep actor alive to continuously consume prompts
		//
		// Future versions will enable sleep/wake for durable streams
		noSleep: true,
	},
});

async function consumeStream(c: ActorContextOf<typeof aiAgent>) {
	c.log.info({
		msg: "consumeStream started",
		conversationId: c.state.conversationId,
		offset: c.state.promptStreamOffset,
	});

	const { promptStream, responseStream } = await getStreams(
		c.state.conversationId,
	);

	c.log.info({ msg: "streams obtained" });

	const decoder = new TextDecoder();

	try {
		// NOTE: promptStream.json does not provide offsets, have to manually
		// parse
		c.log.info({
			msg: "starting read loop",
			offset: c.state.promptStreamOffset,
		});
		const streamIter = promptStream.read({
			offset: c.state.promptStreamOffset,
			live: "long-poll",
			signal: c.abortSignal,
		});
		for await (const chunk of streamIter) {
			c.log.info({
				msg: "received chunk",
				dataLength: chunk.data.length,
				offset: chunk.offset,
			});

			if (c.aborted) break;

			c.state.promptStreamOffset = chunk.offset;

			if (chunk.data.length === 0) continue;

			const text = decoder.decode(chunk.data);
			const lines = text.split("\n").filter((l) => l.trim());

			c.log.info({ msg: "parsed lines", lineCount: lines.length, text });

			for (const line of lines) {
				let prompts: PromptMessage[];
				try {
					prompts = JSON.parse(line);
				} catch (e) {
					c.log.error({
						msg: "failed to parse json",
						line,
						error: e,
					});
					continue;
				}

				if (!Array.isArray(prompts)) {
					throw new Error(
						`Expected array of prompts, got: ${typeof prompts}`,
					);
				}

				for (const prompt of prompts) {
					c.log.info({
						msg: "parsed prompt",
						promptId: prompt.id,
						content: prompt.content,
					});

					// Skip if prompt is invalid
					if (!prompt.id) {
						c.log.warn({ msg: "skipping prompt with no id" });
						continue;
					}

					c.log.info({
						msg: "processing prompt",
						promptId: prompt.id,
					});
					await processPrompt(c, prompt, responseStream);
					c.log.info({
						msg: "finished processing prompt",
						promptId: prompt.id,
					});
				}
			}
		}
		c.log.info({ msg: "read loop ended" });
	} catch (error) {
		c.log.error({
			msg: "error in consumeStream",
			error,
			aborted: c.aborted,
		});
		if (!c.aborted) {
			c.log.error({ msg: "error consuming prompts", error });
		}
	}
}

async function processPrompt(
	c: ActorContextOf<typeof aiAgent>,
	prompt: PromptMessage,
	responseStream: DurableStream,
) {
	c.log.info({ msg: "processPrompt starting", promptId: prompt.id });

	let streamError: Error | null = null;

	c.log.info({ msg: "calling streamText", promptContent: prompt.content });
	const result = streamText({
		model: anthropic("claude-sonnet-4-20250514"),
		prompt: prompt.content,
		onError: (error) => {
			c.log.error({ msg: "streamText onError", error: error.error });
			streamError =
				error.error instanceof Error
					? error.error
					: new Error(String(error.error));
		},
	});

	let fullResponse = "";
	let chunkCount = 0;

	c.log.info({ msg: "starting to consume textStream" });
	for await (const textPart of result.textStream) {
		chunkCount++;
		fullResponse += textPart;

		const responseChunk: ResponseChunk = {
			promptId: prompt.id,
			content: textPart,
			isComplete: false,
			timestamp: Date.now(),
		};

		await responseStream.append(JSON.stringify(responseChunk) + "\n", {
			contentType: "application/json",
		});
	}

	c.log.info({
		msg: "finished consuming textStream",
		chunkCount,
		fullResponseLength: fullResponse.length,
	});

	if (streamError !== null) {
		const errorChunk: ResponseChunk = {
			promptId: prompt.id,
			content: `Error: ${(streamError as Error).message}`,
			isComplete: true,
			timestamp: Date.now(),
		};

		await responseStream.append(JSON.stringify(errorChunk) + "\n", {
			contentType: "application/json",
		});
		c.broadcast("responseError", {
			promptId: prompt.id,
			error: errorChunk.content,
		});
		return;
	}

	// Send completion marker
	const completeChunk: ResponseChunk = {
		promptId: prompt.id,
		content: "",
		isComplete: true,
		timestamp: Date.now(),
	};

	await responseStream.append(JSON.stringify(completeChunk) + "\n", {
		contentType: "application/json",
	});
	c.broadcast("responseComplete", { promptId: prompt.id, fullResponse });
}

export const registry = setup({
	use: { aiAgent },
});
