import { CodeBlock } from "../CodeBlock";
import { Code, CodeGroup } from "../mdx";

const startCode = `import { registry } from "./registry";

// Starts the server and serves the actor API
registry.start();`;

const honoCode = `import { Hono } from "hono";
import { createClient } from "rivetkit/client";
import { registry } from "./registry";

// Build client to communicate with actors (optional)
const client = createClient<typeof registry>("http://localhost:6420");

const app = new Hono();

// Exposes Rivet API to communicate with actors
app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));

// Example endpoint using actors from within your backend (optional)
app.post("/increment/:name", async (c) => {
	const name = c.req.param("name");

	const counter = client.counter.getOrCreate(name);
	const newCount = await counter.increment(1);

	return c.text(\`New Count: \${newCount}\`);
});

export default app;`;

const elysiaCode = `import { Elysia } from "elysia";
import { createClient } from "rivetkit/client";
import { registry } from "./registry";

// Build client to communicate with actors (optional)
const client = createClient<typeof registry>("http://localhost:6420");

const app = new Elysia()
	// Exposes Rivet API to communicate with actors
	.all("/api/rivet/*", (c) => registry.handler(c.request))
	// Example endpoint using actors from within your backend (optional)
	.get("/increment/:name", async ({ params }) => {
		const counter = client.counter.getOrCreate(params.name);
		const newCount = await counter.increment(1);
		return \`New Count: \${newCount}\`;
	});

export default app;`;

export function StepSetupServer() {
	return (
		<>
			<p>Integrate with your preferred web framework:</p>

			<CodeGroup>
				<Code title="registry.start()" language="ts">
					<CodeBlock lang="typescript" code={startCode} />
				</Code>
				<Code title="Hono" language="ts">
					<CodeBlock lang="typescript" code={honoCode} />
				</Code>
				<Code title="Elysia" language="ts">
					<CodeBlock lang="typescript" code={elysiaCode} />
				</Code>
			</CodeGroup>
		</>
	);
}
