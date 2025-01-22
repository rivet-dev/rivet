import { type OpenAIProvider, createOpenAI } from "@ai-sdk/openai";
import {
	Actor,
	type OnBeforeConnectOptions,
	type Rpc,
	UserError,
} from "@rivet-gg/actor";
import { assert } from "@std/assert";
import { CATALOG, type CatalogItem } from "./catalog.ts";

// HACK: `ai` package is not currently packaged correctly. Using esm.sh as a fallback.
//
// Zod must be the exact same package or else we'll get a "took too long to evaluate" error.
import {
	type CoreMessage,
	streamText,
	tool,
} from "https://esm.sh/ai@^4.1.0&deps=zod@3.24.1";
import { z } from "https://esm.sh/zod@3.24.1";

const SYSTEM_PROMPT = `
You are ShopperAgent, an intelligent shopping assistant designed to interact with users and manage their shopping experience. Your primary responsibilities include updating the shopping cart, retrieving the current shopping cart status, and providing information about the available catalog of items. You are powered by the GPT-4-turbo model and utilize various tools to perform your tasks effectively.

Capabilities:
1. **Update Shopping Cart**: You can update the quantity of items in the user's shopping cart. You require the item's slug and the desired count to perform this action.
2. **Retrieve Shopping Cart**: You can provide the current status of the user's shopping cart, detailing the items and their quantities.
3. **Access Catalog**: You can access and provide information about the catalog of available items.

Constraints:
- You must ensure that any updates to the shopping cart are accurately reflected and communicated to the user.
- You should maintain a history of interactions to improve user experience and provide context for ongoing conversations.
- You should handle all operations asynchronously and ensure that responses are prompt and informative.

Interaction Guidelines:
- Always confirm actions taken, such as updates to the shopping cart, with a clear and concise message.
- Provide detailed information when retrieving the shopping cart or catalog, ensuring the user has all necessary details.
- Maintain a friendly and helpful tone, ensuring the user feels supported throughout their shopping experience.

Remember, your goal is to enhance the user's shopping experience by providing efficient and accurate assistance. Always prioritize user satisfaction and clarity in your interactions.
`;

interface ShoppingCartItem {
	slug: string;
	count: number;
}

interface State {
	/** Items currently in the shopping cart. */
	shoppingCart: ShoppingCartItem[];

	/** Message history. */
	messages: CoreMessage[];
}

interface ConnParams {
	openaiKey?: string;
}

interface ConnState {
	openai?: OpenAIProvider;
}

export default class ShopperAgent extends Actor<State, ConnParams, ConnState> {
	#isStreaming = false;

	override _onInitialize(): State {
		return { shoppingCart: [], messages: [] };
	}

	override _onStateChange(newState: State) {
		this._broadcast("shoppingCartUpdate", {
			shoppingCart: newState.shoppingCart,
		});
	}

	protected override _onBeforeConnect(
		opts: OnBeforeConnectOptions<ShopperAgent>,
	): ConnState {
		// biome-ignore lint/suspicious/noExplicitAny: parameters type unknown
		const apiKey = (opts.parameters as any)?.openaiKey;
		if (!apiKey) {
			return {};
		}
		const openai = createOpenAI({ apiKey });
		return { openai };
	}

	updateItemInCart(_rpc: Rpc<ShopperAgent>, slug: string, count: number) {
		// Assert that the item is in the catalog
		const isInCatalog = CATALOG.some((item) => item.slug === slug);
		assert(isInCatalog, `Item with slug ${slug} is not in the catalog`);

		const itemIndex = this._state.shoppingCart.findIndex(
			(item) => item.slug === slug,
		);

		if (itemIndex !== -1) {
			// Update the count of the existing item
			this._state.shoppingCart[itemIndex].count = count;
		} else {
			// Add the new item to the cart
			this._state.shoppingCart.push({ slug, count });
		}
	}

	getMessages(_rpc: Rpc<ShopperAgent>): CoreMessage[] {
		return this._state.messages;
	}

	getShoppingCart(_rpc: Rpc<ShopperAgent>): ShoppingCartItem[] {
		return this._state.shoppingCart;
	}

	getCatalog(_rpc: Rpc<ShopperAgent>): CatalogItem[] {
		return CATALOG;
	}

	async processPrompt(rpc: Rpc<ShopperAgent>, prompt: string) {
		try {
			// Add check for openai provider
			if (!rpc.connection.state.openai) {
				throw new UserError("OpenAI API key not configured.", {
					code: "openai_not_configured",
					metadata: {},
				});
			}

			// Prevent duplicate streaming
			assert(!this.#isStreaming, "already streaming response");
			this.#isStreaming = true;

			const { textStream } = streamText({
				model: rpc.connection.state.openai("gpt-4o"),
				system: SYSTEM_PROMPT,
				messages: this._state.messages,
				prompt,
				maxSteps: 5,
				tools: {
					updateItemsInCart: tool({
						description:
							"Update multiple items in the shopping cart",
						parameters: z.object({
							updateItems: z
								.array(
									z.object({
										slug: z
											.string()
											.describe(
												"The slug of the item to update in the cart",
											),
										count: z
											.number()
											.describe(
												"The count of the item to update in the cart",
											),
									}),
								)
								.describe(
									"Array of items to update in the shopping cart",
								),
						}),
						execute: async ({ updateItems: items }) => {
							for (const { slug, count } of items) {
								this.updateItemInCart(rpc, slug, count);
							}
							return `Updated items in the cart: ${items
								.map(
									(item) =>
										`${item.slug} with count ${item.count}`,
								)
								.join(", ")}.`;
						},
					}),
					getShoppingCart: tool({
						description: "Get the current shopping cart",
						parameters: z.object({}),
						execute: async () => {
							const cart = this.getShoppingCart(rpc);
							return `Current shopping cart: ${JSON.stringify(cart)}`;
						},
					}),
					getCatalog: tool({
						description: "Get the catalog of items",
						parameters: z.object({}),
						execute: async () => {
							return `Catalog: ${JSON.stringify(this.getCatalog(rpc))}`;
						},
					}),
				},
				onFinish: ({ text }) => {
					// Save response
					this._state.messages.push({
						role: "user",
						content: prompt,
					});
					this._state.messages.push({
						role: "system",
						content: text,
					});
				},
			});

			// Stream response
			for await (const textPart of textStream) {
				this._broadcast("textPart", textPart);
			}
		} catch (error) {
			this._log.error("error generating response", { error });
			throw new UserError("Error while generating response.", {
				code: "error_generating_response",
				metadata: {},
			});
		} finally {
			this.#isStreaming = false;
			this._broadcast("textFinish");
		}
	}
}
