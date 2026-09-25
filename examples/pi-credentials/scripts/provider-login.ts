/**
 * Logs in to a provider in the terminal and saves the login to a user's
 * `credentials` actor. This script is only for trying the example. Your app
 * needs its own login flow, such as a settings page in your web app, that
 * saves the result with the same `save` action.
 */
import { createInterface } from "node:readline/promises";
import { InMemoryCredentialStore } from "@earendil-works/pi-ai";
import { ModelRuntime } from "@earendil-works/pi-coding-agent";
import { createClient } from "rivetkit/client";
import type { registry } from "../src/actors.ts";

const user = process.argv[2] ?? "me";
const terminal = createInterface({
	input: process.stdin,
	output: process.stderr,
});
const provider = (
	await terminal.question(
		"Provider (for example anthropic or openai-codex): ",
	)
).trim();
const type =
	(await terminal.question("1) Subscription or 2) API key? [1] ")).trim() ===
	"2"
		? "api_key"
		: "oauth";

const runtime = await ModelRuntime.create({
	credentials: new InMemoryCredentialStore(),
	modelsPath: null,
});
const credential = await runtime.login(provider, type, {
	notify: (event) => {
		switch (event.type) {
			case "auth_url":
				return console.error(
					`Open ${event.url}\n${event.instructions ?? ""}`,
				);
			case "device_code":
				return console.error(
					`Open ${event.verificationUri} and enter ${event.userCode}`,
				);
			case "info":
			case "progress":
				return console.error(event.message);
		}
	},
	prompt: async (prompt) => {
		if (prompt.type === "select") {
			for (const [i, option] of prompt.options.entries())
				console.error(`${i + 1}) ${option.label}`);
		}
		const answer = (await terminal.question(`${prompt.message} `)).trim();
		return prompt.type === "select"
			? (prompt.options[Number(answer) - 1]?.id ?? answer)
			: answer;
	},
});

await createClient<typeof registry>()
	.credentials.getOrCreate([user])
	.save(provider, credential);
console.error(`Saved the ${provider} login for ${user}.`);
process.exit(0);
