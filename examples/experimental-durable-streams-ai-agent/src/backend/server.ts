import { registry } from "./registry";

if (!process.env.ANTHROPIC_API_KEY) {
	throw new Error("ANTHROPIC_API_KEY environment variable is required");
}

registry.start();
