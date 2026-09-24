import { createInterface } from "node:readline/promises";
import { type Credential, InMemoryCredentialStore } from "@earendil-works/pi-ai";
import { ModelRuntime } from "@earendil-works/pi-coding-agent";

/** Runs Pi's provider login in the terminal and returns the credential it produced. */
export async function loginInTerminal(): Promise<{ provider: string; credential: Credential }> {
	const terminal = createInterface({ input: process.stdin, output: process.stderr });
	const provider = (await terminal.question("Provider (for example anthropic or openai-codex): ")).trim();
	const type = (await terminal.question("1) Subscription or 2) API key? [1] ")).trim() === "2" ? "api_key" : "oauth";
	const runtime = await ModelRuntime.create({ credentials: new InMemoryCredentialStore(), modelsPath: null });
	const credential = await runtime.login(provider, type, {
		notify: (event) => {
			switch (event.type) {
				case "auth_url":
					return console.error(`Open ${event.url}\n${event.instructions ?? ""}`);
				case "device_code":
					return console.error(`Open ${event.verificationUri} and enter ${event.userCode}`);
				case "info":
				case "progress":
					return console.error(event.message);
			}
		},
		prompt: async (prompt) => {
			if (prompt.type === "select") {
				for (const [i, option] of prompt.options.entries()) console.error(`${i + 1}) ${option.label}`);
			}
			const answer = (await terminal.question(`${prompt.message} `)).trim();
			return prompt.type === "select" ? (prompt.options[Number(answer) - 1]?.id ?? answer) : answer;
		},
	});
	terminal.close();
	return { provider, credential };
}
