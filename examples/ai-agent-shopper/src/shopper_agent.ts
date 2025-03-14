import { type OpenAIProvider, createOpenAI } from "@ai-sdk/openai";
import {
	Actor,
	type OnBeforeConnectOptions,
	type Rpc,
	UserError,
} from "@rivet-gg/actor";
import { type CoreMessage, streamText, tool } from "ai";
import { z } from "zod";
import {
	type CatalogItem,
	getCatalogItemBySlug,
	searchCatalogByKeywords,
} from "./catalog";

const SYSTEM_PROMPT = `You are a helpful hardware store shopping assistant. Your role is to help customers find and purchase items from our catalog.

Key responsibilities:
- Use the searchCatalog tool with relevant tags to find items that match the customer's needs
- When searching, include common synonyms and related terms for more complete results (e.g. "saw" should include "cutting", "blade"; "hammer" should include "mallet", "striking")
- Help customers add items to their cart using updateItemsInCart
- Check the current cart contents using getShoppingCart when needed
- Only recommend items that are confirmed to exist in the catalog search results
- Ask for clarification if the customer's request is ambiguous

Important restrictions:
- Never make assumptions about item availability - always search the catalog first
- You are not able to list the entire catalog, but you can search by keywords
- Only suggest items that are explicitly returned by the searchCatalog tool
- If a requested item cannot be found, explain this to the customer and offer to search for alternatives
- Do not make up prices, specifications, or details about items
- If unsure about a specific item, ask the customer for more details to refine the search

When searching:
1. Break down the customer's request into relevant search tags
2. Use searchCatalog to find matching items
3. Review the search results and recommend appropriate items
4. Help add selected items to cart with specific quantities

Always maintain a helpful and professional tone while staying strictly within the bounds of available catalog items.
`;

export interface ShoppingCartItem {
	slug: string;
	count: number;
}

interface State {
	/** Items currently in the shopping cart. */
	shoppingCart: ShoppingCartItem[];

	/** Message history. */
	messages: CoreMessage[];
}

const MAX_MESSAGE_HISTORY = 20;

interface ConnParams {
	openaiKey?: string;
}

interface ConnState {
	openai?: OpenAIProvider;
}

export default class ShopperAgent extends Actor<State, ConnParams, ConnState> {
	#isStreaming = false;

	constructor() {
		super({
			rpc: {
				timeout: 60_000,
			},
		});
	}

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
		const isInCatalog = getCatalogItemBySlug(slug);
		if (!isInCatalog)
			throw new Error(`Item with slug ${slug} is not in the catalog`);

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

	async searchCatalog(
		_rpc: Rpc<ShopperAgent>,
		keywords: string[],
	): Promise<CatalogItem[]> {
		return searchCatalogByKeywords(keywords);
	}

	resetState() {
		this._state.messages = [];
		this._state.shoppingCart = [];
	}

	async processPrompt(
		rpc: Rpc<ShopperAgent>,
		prompt: string,
	): Promise<string> {
		try {
			// Add check for openai provider
			if (!rpc.connection.state.openai) {
				throw new UserError("OpenAI API key not configured.", {
					code: "openai_not_configured",
					metadata: {},
				});
			}

			// Prevent duplicate streaming
			if (this.#isStreaming)
				throw new Error("already streaming response");
			this.#isStreaming = true;

			// Trim history if it exceeds the maximum
			if (this._state.messages.length >= MAX_MESSAGE_HISTORY) {
				this._state.messages = this._state.messages.slice(
					-MAX_MESSAGE_HISTORY,
				);
			}

			this._state.messages.push({
				role: "user",
				content: prompt,
			});

			this._log.info("prompt start", { prompt });
			const startTime = performance.now();
			const { textStream } = streamText({
				model: rpc.connection.state.openai("gpt-4o"),
				system: SYSTEM_PROMPT,
				messages: this._state.messages,
				maxSteps: 5,
				maxTokens: 16_000,
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
											.refine(
												(slug) =>
													getCatalogItemBySlug(
														slug,
													) !== undefined,
												(slug) => ({
													message: `Invalid catalog item: ${slug}`,
												}),
											)
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
					searchCatalog: tool({
						description: "Search the catalog of items using tags",
						parameters: z.object({
							tags: z
								.array(z.string())
								.describe(
									"Array of tags to search for in the catalog",
								),
						}),
						execute: async ({ tags }) => {
							const results = await this.searchCatalog(rpc, tags);
							return `Search results for tags [${tags.join(", ")}]: ${JSON.stringify(results)}`;
						},
					}),
					getCatalogItems: tool({
						description:
							"Look up specific catalog items by their slugs",
						parameters: z.object({
							slugs: z
								.array(z.string())
								.describe("Array of item slugs to look up"),
						}),
						execute: async ({ slugs }) => {
							const items = slugs
								.map((slug) => getCatalogItemBySlug(slug))
								.filter(
									(item): item is CatalogItem =>
										item !== undefined,
								);
							return `Items found: ${JSON.stringify(items)}`;
						},
					}),
				},
				onStepFinish: ({
					text,
					stepType,
					finishReason,
					toolCalls,
					toolResults,
				}) => {
					this._log.info("prompt step finish", {
						text,
						stepType,
						finishReason,
						toolCalls: toolCalls.map(({ toolName, args }) => ({
							name: toolName,
							args,
						})),
						toolResults,
					});
				},
				onFinish: ({ text }) => {
					// Save response
					this._state.messages.push({
						role: "system",
						content: text,
					});

					const elapsed = performance.now() - startTime;
					this._log.info("prompt finish", {
						text,
						elapsed: (elapsed / 1000).toFixed(3),
					});
				},
			});

			// Stream response
			let fullText = "";
			for await (const textPart of textStream) {
				fullText += textPart;
				this._broadcast("textPart", textPart);
			}

			return fullText;
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
