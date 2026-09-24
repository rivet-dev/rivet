import { randomUUID } from "node:crypto";
import { fauxAssistantMessage } from "@earendil-works/pi-ai";
import { setup, UserError } from "rivetkit";
import { setupTest } from "rivetkit/test";
import { beforeAll, describe, expect, test } from "vitest";
import { type PiCredentialSource, type PiProviderCredential, type PiProviderConfig, pi } from "../src/index.js";
import { createMockModel, type MockModel } from "./helpers/mock-model.js";

let mockModel: MockModel;
let registry: ReturnType<typeof buildRegistry>;

/** Credentials by actor key, as an application's own storage would hold them. */
const stored = new Map<string, Map<string, PiProviderCredential>>();
/** Providers the application refreshed, in order. */
const refreshed: string[] = [];

/** A subscription provider whose tokens Pi actors use but never refresh. */
function subscriptionProvider(mock: MockModel): PiProviderConfig {
	return {
		...mock.providerConfig,
		models: [{ ...mock.providerConfig.models![0]!, id: "sub-model", name: "sub-model" }],
		oauth: {
			name: "Test subscription",
			login: async () => {
				throw new Error("the application logs in");
			},
			refreshToken: async () => {
				throw new Error("Pi actors must not refresh application tokens");
			},
			getApiKey: (credential) => credential.access,
		},
	};
}

/** An application credential source backed by `stored`, keyed by the actor's key. */
function mapSource(c: { key: readonly string[] }): PiCredentialSource {
	const owner = c.key[0]!;
	const credentials = () => {
		if (owner.startsWith("down-")) {
			throw new UserError("The account service is unavailable.", { code: "account_unavailable" });
		}
		return stored.get(owner) ?? new Map<string, PiProviderCredential>();
	};
	return {
		list: async () =>
			[...credentials()].map(([providerId, credential]) => ({ providerId, type: credential.type })),
		read: async (providerId) => credentials().get(providerId),
		refresh: async (providerId) => {
			refreshed.push(providerId);
			const next = { type: "oauth" as const, access: "fresh-access", expires: Date.now() + 3_600_000 };
			credentials().set(providerId, next);
			return next;
		},
	};
}

function buildRegistry(mock: MockModel) {
	const agent = pi({
		providers: { mock: mock.providerConfig, sub: subscriptionProvider(mock) },
		model: "mock/mock-model",
		scopedModels: ["mock/mock-model", "sub/sub-model"],
		credentials: mapSource,
	});
	return setup({ use: { agent } });
}

beforeAll(async () => {
	mockModel = createMockModel();
	mockModel.reply("say hello", fauxAssistantMessage("Hi there!"));
	registry = buildRegistry(mockModel);
});

/** The API keys the model received for one prompt. */
async function promptOnce(handle: { prompt(text: string): Promise<void> }) {
	const before = mockModel.requests.length;
	await handle.prompt("say hello");
	return mockModel.requests.slice(before).map((request) => request.apiKey);
}

describe("pi application credentials", () => {
	test("a key the application supplies is used, and a logout applies from the next prompt", async (c) => {
		const { client } = await setupTest(c, registry);
		const owner = `alice-${randomUUID()}`;
		stored.set(owner, new Map([["mock", { type: "api_key", key: "alice-key" }]]));
		const agent = client.agent.getOrCreate([owner]);

		expect(await promptOnce(agent)).toEqual(["alice-key"]);

		stored.get(owner)!.delete("mock");
		await expect(agent.prompt("say hello")).rejects.toMatchObject({ code: "model_unavailable" });
	});

	test("a subscription token that expires soon is refreshed by the application before the model call", async (c) => {
		const { client } = await setupTest(c, registry);
		const owner = `bob-${randomUUID()}`;
		stored.set(owner, new Map([["sub", { type: "oauth", access: "old-access", expires: Date.now() + 60_000 }]]));
		const agent = client.agent.getOrCreate([owner]);
		await agent.setModel("sub", "sub-model");

		refreshed.length = 0;
		expect(await promptOnce(agent)).toEqual(["fresh-access"]);
		expect(refreshed).toEqual(["sub"]);
	});

	test("an error from the application's credential source rejects the prompt with that error", async (c) => {
		const { client } = await setupTest(c, registry);
		const agent = client.agent.getOrCreate([`down-${randomUUID()}`]);

		await expect(agent.prompt("say hello")).rejects.toMatchObject({
			code: "account_unavailable",
			message: "The account service is unavailable.",
		});
	});
});
