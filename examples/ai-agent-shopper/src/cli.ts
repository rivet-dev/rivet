import { intro, isCancel, outro, password, text } from "@clack/prompts";
import { TestClient } from "@rivet-gg/actor-client/test";
import type { CoreMessage } from "ai";
import colors from "picocolors";
import type ShopperAgent from "./shopper_agent";
import type { ShoppingCartItem } from "./shopper_agent";

function displayCart(cart: ShoppingCartItem[]) {
	console.log(`\n${colors.yellow("Current Shopping Cart:")}`);
	if (cart.length === 0) {
		console.log(colors.dim("(empty)"));
	} else {
		for (const item of cart) {
			console.log(colors.green(`- ${item.slug}: ${item.count} units`));
		}
	}
	console.log();
}

function displayMessages(messages: CoreMessage[]) {
	console.log(`\n${colors.yellow("Message History:")}`);
	if (messages.length === 0) {
		console.log(colors.dim("(no messages)"));
	} else {
		for (const msg of messages) {
			console.log(colors.dim(`\n[${msg.role}]:`));
			console.log(colors.white(msg.content as string));
		}
	}
	console.log();
}

async function main() {
	// Initialize the client
	const client = new TestClient();

	// Get OpenAI key from environment or prompt
	let openaiKey = process.env.OPENAI_KEY;
	if (!openaiKey) {
		openaiKey = (await password({
			message:
				"Please enter your OpenAI API key: (this is only stored in-memory)",
			mask: "•",
		})) as string;

		if (isCancel(openaiKey)) {
			outro(colors.red("Operation cancelled"));
			process.exit(0);
		}
	}
	if (!openaiKey) throw new Error("OpenAI API key is required");

	// Connect to the shopper agent
	const shopperAgent = await client.get<ShopperAgent>(
		{
			name: "shopper_agent",
		},
		{
			parameters: {
				openaiKey,
			},
		},
	);

	// Set up text streaming with better formatting
	shopperAgent.on("textPart", (text: string) => {
		process.stdout.write(colors.cyan(text));
	});

	shopperAgent.on("textFinish", () => {
		process.stdout.write("\n\n");
	});

	// Show welcome message
	intro(colors.blue("Welcome to the Hardware Store Assistant!"));

	// Replace the initial message history and cart display with helper functions
	const messages = await shopperAgent.getMessages();
	displayMessages(messages);

	const cart = await shopperAgent.getShoppingCart();
	displayCart(cart);

	// Main interaction loop
	while (true) {
		const question = await text({
			message:
				"How can I help you? (type '/cart' to view cart, '/messages' to view history, '/search' to search catalog, '/reset' to clear history, '/exit' to quit)",
			placeholder: "What tools do you recommend for...",
		});

		if (isCancel(question) || question === "/exit") {
			break;
		}

		if (question.startsWith("/")) {
			if (question === "/cart") {
				const cart = await shopperAgent.getShoppingCart();
				displayCart(cart);
				continue;
			}

			if (question === "/messages") {
				const messages = await shopperAgent.getMessages();
				displayMessages(messages);
				continue;
			}

			if (question === "/reset") {
				await shopperAgent.resetState();
				console.log(
					colors.yellow(
						"\nChat history and shopping cart have been reset.\n",
					),
				);
				continue;
			}

			if (question.startsWith("/search")) {
				const searchQuery = question.slice(7).trim();
				if (!searchQuery) {
					console.log(
						colors.dim(
							"\nPlease provide search terms after /search, e.g. '/search hammer, nails'\n",
						),
					);
					continue;
				}

				const searchTerms = searchQuery
					.split(/[^a-zA-Z]+/)
					.map((term) => term.trim().toLowerCase())
					.filter((term) => term.length > 0);
				const results = await shopperAgent.searchCatalog(searchTerms);

				console.log(`\n${colors.yellow("Search Results:")}`);
				if (results.length === 0) {
					console.log(colors.dim("(no results)"));
				} else {
					for (const item of results) {
						console.log(
							colors.green(`- ${item.slug}: ${item.name}`),
						);
						console.log(colors.blue(`  Price: $${item.price}`));
					}
				}
				console.log();
				continue;
			}

			if (question === "/exit") {
				break;
			}

			// If none of the valid commands match, throw an error
			throw new Error(`Invalid command: ${question}`);
		}

		if (question) {
			try {
				await shopperAgent.processPrompt(question);
			} catch (error) {
				console.error(colors.red("Error:"), `${error}`);
			}
		}
	}

	// Cleanup
	await shopperAgent.disconnect();
	outro(colors.blue("Thanks for shopping with us!"));
}

await main();
