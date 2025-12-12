import { DurableStream } from "@durable-streams/client";

export const STREAMS_SERVER_URL = "http://127.0.0.1:8787/v1/stream";

export interface ConversationStreams {
	promptStream: DurableStream;
	responseStream: DurableStream;
}

export async function getStreams(
	conversationId: string,
): Promise<ConversationStreams> {
	const promptStreamUrl = `${STREAMS_SERVER_URL}/conversations/${conversationId}/prompts`;
	const responseStreamUrl = `${STREAMS_SERVER_URL}/conversations/${conversationId}/responses`;

	let promptStream: DurableStream;
	let responseStream: DurableStream;

	try {
		promptStream = await DurableStream.create({
			url: promptStreamUrl,
			contentType: "application/json",
		});
	} catch {
		promptStream = new DurableStream({ url: promptStreamUrl });
	}

	try {
		responseStream = await DurableStream.create({
			url: responseStreamUrl,
			contentType: "application/json",
		});
	} catch {
		responseStream = new DurableStream({ url: responseStreamUrl });
	}

	return { promptStream, responseStream };
}

/**
 * Get the stream paths for a conversation (useful for UI display)
 */
export function getStreamPaths(conversationId: string) {
	return {
		promptStreamPath: `conversations/${conversationId}/prompts`,
		responseStreamPath: `conversations/${conversationId}/responses`,
	};
}
