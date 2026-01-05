export type Message = {
	role: "user" | "assistant";
	content: string;
	timestamp: number;
};
