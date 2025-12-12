export interface PromptMessage {
	id: string;
	content: string;
	timestamp: number;
}

export interface ResponseChunk {
	promptId: string;
	content: string;
	isComplete: boolean;
	timestamp: number;
}
